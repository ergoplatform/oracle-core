use std::{convert::TryInto, io::Write};

use derive_more::From;
use ergo_lib::{
    chain::{
        ergo_box::box_builder::{ErgoBoxCandidateBuilder, ErgoBoxCandidateBuilderError},
        transaction::Transaction,
    },
    ergotree_ir::{
        chain::{
            address::{Address, AddressEncoder, AddressEncoderError, NetworkPrefix},
            ergo_box::{
                box_value::{BoxValue, BoxValueError},
                ErgoBox,
            },
            token::Token,
        },
        ergo_tree::ErgoTree,
        serialization::SigmaParsingError,
    },
    wallet::{
        box_selector::{BoxSelector, BoxSelectorError, SimpleBoxSelector},
        tx_builder::{TxBuilder, TxBuilderError},
    },
};
use ergo_node_interface::node_interface::NodeError;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    contracts::{
        pool::PoolContractParameters,
        refresh::{
            RefreshContract, RefreshContractError, RefreshContractInputs, RefreshContractParameters,
        },
        update::{
            UpdateContract, UpdateContractError, UpdateContractInputs, UpdateContractParameters,
        },
    },
    node_interface::{new_node_interface, SignTransaction, SubmitTransaction},
    oracle_config::{OracleConfig, ORACLE_CONFIG},
    wallet::WalletDataSource,
};

use super::bootstrap::{Addresses, NftMintDetails, TokenMintDetails};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateTokensToMint {
    pub refresh_nft: Option<NftMintDetails>,
    pub update_nft: Option<NftMintDetails>,
    pub oracle_tokens: Option<TokenMintDetails>,
    pub ballot_tokens: Option<TokenMintDetails>,
    pub reward_tokens: Option<TokenMintDetails>,
}

#[derive(Clone, Deserialize)]
#[serde(try_from = "crate::serde::UpdateBootstrapConfigSerde")]
pub struct UpdateBootstrapConfig {
    pub pool_contract_parameters: Option<PoolContractParameters>, // New pool script, etc. Note that we don't actually mint any new pool NFT in the update step, instead this is simply passed to the new oracle config for convenience
    pub refresh_contract_parameters: Option<RefreshContractParameters>,
    pub update_contract_parameters: Option<UpdateContractParameters>,
    pub tokens_to_mint: UpdateTokensToMint,
    pub addresses: Addresses,
}

pub fn update(config_file_name: String) -> Result<(), UpdateBootstrapError> {
    let s = std::fs::read_to_string(config_file_name)?;
    let config: UpdateBootstrapConfig = serde_yaml::from_str(&s)?;

    let node_interface = new_node_interface();
    let prefix = if ORACLE_CONFIG.on_mainnet {
        NetworkPrefix::Mainnet
    } else {
        NetworkPrefix::Testnet
    };
    let change_address = AddressEncoder::new(prefix).parse_address_from_str(
        &node_interface
            .wallet_status()?
            .change_address
            .ok_or(UpdateBootstrapError::NoChangeAddressSetInNode)?,
    )?;
    let update_bootstrap_input = UpdateBootstrapInput {
        config: config.clone(),
        wallet: &node_interface,
        tx_signer: &node_interface,
        submit_tx: &node_interface,
        tx_fee: BoxValue::SAFE_USER_MIN,
        erg_value_per_box: BoxValue::SAFE_USER_MIN,
        change_address,
        height: node_interface
            .current_block_height()
            .unwrap()
            .try_into()
            .unwrap(),
        old_config: ORACLE_CONFIG.clone(),
    };

    let new_config = perform_update_chained_transaction(update_bootstrap_input)?;

    info!("Update chain-transaction complete");
    info!("Writing new config file to oracle_config_updated.yaml");
    let s = serde_yaml::to_string(&new_config)?;
    let mut file = std::fs::File::create("oracle_config_updated.yaml")?;
    file.write_all(s.as_bytes())?;
    info!("Updated oracle configuration file oracle_config_updated.yaml");
    Ok(())
}

pub struct UpdateBootstrapInput<'a> {
    pub config: UpdateBootstrapConfig,
    pub wallet: &'a dyn WalletDataSource,
    pub tx_signer: &'a dyn SignTransaction,
    pub submit_tx: &'a dyn SubmitTransaction,
    pub tx_fee: BoxValue,
    pub erg_value_per_box: BoxValue,
    pub change_address: Address,
    pub height: u32,
    pub old_config: OracleConfig,
}

pub(crate) fn perform_update_chained_transaction(
    input: UpdateBootstrapInput,
) -> Result<OracleConfig, UpdateBootstrapError> {
    let UpdateBootstrapInput {
        config,
        wallet,
        tx_signer: wallet_sign,
        submit_tx,
        tx_fee,
        erg_value_per_box,
        change_address,
        height,
        old_config,
        ..
    } = input;

    let mut num_transactions_left = 1;

    if config.refresh_contract_parameters.is_some() {
        num_transactions_left += 1;
    }
    if config.update_contract_parameters.is_some() {
        num_transactions_left += 1;
    }
    if config.tokens_to_mint.oracle_tokens.is_some() {
        num_transactions_left += 1;
    }
    if config.tokens_to_mint.ballot_tokens.is_some() {
        num_transactions_left += 1;
    }
    if config.tokens_to_mint.reward_tokens.is_some() {
        num_transactions_left += 1;
    }

    let wallet_pk_ergo_tree = config
        .addresses
        .wallet_address_for_chain_transaction
        .script()?;
    let guard = wallet_pk_ergo_tree.clone();

    // Since we're building a chain of transactions, we need to filter the output boxes of each
    // constituent transaction to be only those that are guarded by our wallet's key.
    let filter_tx_outputs = move |outputs: Vec<ErgoBox>| -> Vec<ErgoBox> {
        outputs
            .clone()
            .into_iter()
            .filter(|b| b.ergo_tree == guard)
            .collect()
    };

    // This closure computes `E_{num_transactions_left}`.
    let calc_target_balance = |num_transactions_left| {
        let b = erg_value_per_box.checked_mul_u32(num_transactions_left)?;
        let fees = tx_fee.checked_mul_u32(num_transactions_left)?;
        b.checked_add(&fees)
    };

    // Effect a single transaction that mints a token with given details, as described in comments
    // at the beginning. By default it uses `wallet_pk_ergo_tree` as the guard for the token box,
    // but this can be overriden with `different_token_box_guard`.
    let mint_token = |input_boxes: Vec<ErgoBox>,
                      num_transactions_left: &mut u32,
                      token_name,
                      token_desc,
                      token_amount,
                      different_token_box_guard: Option<ErgoTree>|
     -> Result<(Token, Transaction), UpdateBootstrapError> {
        let target_balance = calc_target_balance(*num_transactions_left)?;
        let box_selector = SimpleBoxSelector::new();
        let box_selection = box_selector.select(input_boxes, target_balance, &[])?;
        let token = Token {
            token_id: box_selection.boxes.first().box_id().into(),
            amount: token_amount,
        };
        let token_box_guard =
            different_token_box_guard.unwrap_or_else(|| wallet_pk_ergo_tree.clone());
        let mut builder = ErgoBoxCandidateBuilder::new(erg_value_per_box, token_box_guard, height);
        builder.mint_token(token.clone(), token_name, token_desc, 1);
        let mut output_candidates = vec![builder.build()?];

        let remaining_funds = ErgoBoxCandidateBuilder::new(
            calc_target_balance(*num_transactions_left - 1)?,
            wallet_pk_ergo_tree.clone(),
            height,
        )
        .build()?;
        output_candidates.push(remaining_funds.clone());

        let inputs = box_selection.boxes.clone();
        let tx_builder = TxBuilder::new(
            box_selection,
            output_candidates,
            height,
            tx_fee,
            change_address.clone(),
            BoxValue::MIN,
        );
        let mint_token_tx = tx_builder.build()?;
        debug!("Mint token unsigned transaction: {:?}", mint_token_tx);
        let signed_tx = wallet_sign.sign_transaction_with_inputs(&mint_token_tx, inputs, None)?;
        *num_transactions_left -= 1;
        Ok((token, signed_tx))
    };

    let unspent_boxes = wallet.get_unspent_wallet_boxes()?;
    debug!("unspent boxes: {:?}", unspent_boxes);
    let target_balance = calc_target_balance(num_transactions_left)?;
    debug!("target_balance: {:?}", target_balance);
    let box_selector = SimpleBoxSelector::new();
    let box_selection = box_selector.select(unspent_boxes.clone(), target_balance, &[])?;
    debug!("box selection: {:?}", box_selection);

    let mut new_oracle_config = old_config.clone();
    let mut transactions = vec![];
    let mut inputs = box_selection.boxes.clone(); // Inputs for each transaction in chained tx, updated after each mint step

    if let Some(ref token_mint_details) = config.tokens_to_mint.oracle_tokens {
        info!("Minting oracle tokens");
        let (token, tx) = mint_token(
            inputs.into(),
            &mut num_transactions_left,
            token_mint_details.name.clone(),
            token_mint_details.description.clone(),
            token_mint_details.quantity.try_into().unwrap(),
            None,
        )?;
        new_oracle_config.token_ids.oracle_token_id = token.token_id;
        inputs = filter_tx_outputs(tx.outputs.clone()).try_into().unwrap();
        transactions.push(tx);
    }
    if let Some(ref token_mint_details) = config.tokens_to_mint.ballot_tokens {
        info!("Minting ballot tokens");
        let (token, tx) = mint_token(
            inputs.into(),
            &mut num_transactions_left,
            token_mint_details.name.clone(),
            token_mint_details.description.clone(),
            token_mint_details.quantity.try_into().unwrap(),
            None,
        )?;
        new_oracle_config.token_ids.ballot_token_id = token.token_id;
        inputs = filter_tx_outputs(tx.outputs.clone()).try_into().unwrap();
    }
    if let Some(ref token_mint_details) = config.tokens_to_mint.reward_tokens {
        info!("Minting reward tokens");
        let (token, tx) = mint_token(
            inputs.into(),
            &mut num_transactions_left,
            token_mint_details.name.clone(),
            token_mint_details.description.clone(),
            token_mint_details.quantity.try_into().unwrap(),
            None,
        )?;
        new_oracle_config.token_ids.reward_token_id = token.token_id;
        inputs = filter_tx_outputs(tx.outputs.clone()).try_into().unwrap();
        transactions.push(tx);
    }
    if let Some(ref contract_parameters) = config.refresh_contract_parameters {
        info!("Creating new refresh NFT");
        let refresh_contract_inputs = RefreshContractInputs {
            contract_parameters,
            oracle_token_id: &new_oracle_config.token_ids.oracle_token_id,
            pool_nft_token_id: &old_config.token_ids.pool_nft_token_id,
        };
        let refresh_contract = RefreshContract::new(refresh_contract_inputs)?;
        let refresh_nft_details = config
            .tokens_to_mint
            .refresh_nft
            .ok_or(UpdateBootstrapError::NoMintDetails)?;
        let (token, tx) = mint_token(
            inputs.into(),
            &mut num_transactions_left,
            refresh_nft_details.name.clone(),
            refresh_nft_details.description.clone(),
            1.try_into().unwrap(),
            Some(refresh_contract.ergo_tree()),
        )?;
        new_oracle_config.token_ids.refresh_nft_token_id = token.token_id;
        new_oracle_config.refresh_contract_parameters = contract_parameters.clone();
        inputs = filter_tx_outputs(tx.outputs.clone()).try_into().unwrap();
        info!("Refresh contract tx id: {:?}", tx.id());
        transactions.push(tx);
    }
    if let Some(ref contract_parameters) = config.update_contract_parameters {
        info!("Creating new update NFT");
        let update_contract_inputs = UpdateContractInputs {
            contract_parameters,
            ballot_token_id: &new_oracle_config.token_ids.ballot_token_id,
            pool_nft_token_id: &new_oracle_config.token_ids.pool_nft_token_id,
        };
        let update_contract = UpdateContract::new(update_contract_inputs)?;
        let update_nft_details = config
            .tokens_to_mint
            .update_nft
            .ok_or(UpdateBootstrapError::NoMintDetails)?;
        let (token, tx) = mint_token(
            inputs.into(),
            &mut num_transactions_left,
            update_nft_details.name.clone(),
            update_nft_details.description.clone(),
            1.try_into().unwrap(),
            Some(update_contract.ergo_tree()),
        )?;
        new_oracle_config.token_ids.update_nft_token_id = token.token_id;
        new_oracle_config.update_contract_parameters = contract_parameters.clone();
        info!("Update contract tx id: {:?}", tx.id());
        transactions.push(tx);
    }

    if let Some(new_pool_contract_parameters) = config.pool_contract_parameters {
        new_oracle_config.pool_contract_parameters = new_pool_contract_parameters;
    }

    for tx in transactions {
        let tx_id = submit_tx.submit_transaction(&tx)?;
        info!("Tx submitted {}", tx_id);
    }
    Ok(new_oracle_config)
}

#[derive(Debug, Error, From)]
pub enum UpdateBootstrapError {
    #[error("tx builder error: {0}")]
    TxBuilder(TxBuilderError),
    #[error("box builder error: {0}")]
    ErgoBoxCandidateBuilder(ErgoBoxCandidateBuilderError),
    #[error("node error: {0}")]
    Node(NodeError),
    #[error("box selector error: {0}")]
    BoxSelector(BoxSelectorError),
    #[error("box value error: {0}")]
    BoxValue(BoxValueError),
    #[error("IO error: {0}")]
    Io(std::io::Error),
    #[error("serde-yaml error: {0}")]
    SerdeYaml(serde_yaml::Error),
    #[error("yaml-rust error: {0}")]
    YamlRust(String),
    #[error("AddressEncoder error: {0}")]
    AddressEncoder(AddressEncoderError),
    #[error("SigmaParsing error: {0}")]
    SigmaParse(SigmaParsingError),
    #[error("Node doesn't have a change address set")]
    NoChangeAddressSetInNode,
    #[error("Refresh contract failed: {0}")]
    RefreshContract(RefreshContractError),
    #[error("Update contract error: {0}")]
    UpdateContract(UpdateContractError),
    #[error("Bootstrap config file already exists")]
    ConfigFilenameAlreadyExists,
    #[error("No parameters were added for update")]
    NoOpUpgrade,
    #[error("No mint details were provided for update/refresh contract in tokens_to_mint")]
    NoMintDetails,
}

#[cfg(test)]
mod test {
    use ergo_lib::{
        chain::{ergo_state_context::ErgoStateContext, transaction::TxId},
        ergotree_interpreter::sigma_protocol::private_input::DlogProverInput,
        ergotree_ir::chain::{
            address::AddressEncoder,
            ergo_box::{ErgoBox, NonMandatoryRegisters},
        },
        wallet::Wallet,
    };
    use sigma_test_util::force_any_val;

    use super::*;
    use crate::cli_commands::bootstrap::tests::SubmitTxMock;
    use crate::pool_commands::test_utils::{LocalTxSigner, WalletDataMock};

    fn test_update_transaction() {
        let old_config: OracleConfig = serde_yaml::from_str(
            "---
token_ids:
  pool_nft_token_id: FHF/kXzbGVH8N44x/8Cgp3i92xDWUJwgHLTtJVDvn4M=
  refresh_nft_token_id: L5ERlF2PBfXBJzJ0PmAbHegC/nQcOAZeamNy4TKclvo=
  update_nft_token_id: SOAloePnia3O3cXSElkLDx9iETxIgnlXEtVCqGbRF+g=
  oracle_token_id: blxak+JLo73NK1ENhiOpxudO/n4ObSDBFMR6GbKZ9X8=
  reward_token_id: ZdF48OHW1UjygIE8bxhjjUlZ3sHu/MsNKPpNH2EsWu8=
  ballot_token_id: sfLzXXJ78hxZnH6hURhNkd91Z8SxQj5Ut/uTz9x3+BA=

node_ip: 127.0.0.1
node_port: 9052
node_api_key: hello
base_fee: 10000
core_api_port: 9053
oracle_address: 3WzD3VNSK4RtDCZe8njzLzRnWbxcfpCneUcQncAVV9JBDE37nLxR
on_mainnet: false

data_point_source: NanoErgUsd
addresses:
  address_for_oracle_tokens: 3WzD3VNSK4RtDCZe8njzLzRnWbxcfpCneUcQncAVV9JBDE37nLxR
  wallet_address_for_chain_transaction: 3WzD3VNSK4RtDCZe8njzLzRnWbxcfpCneUcQncAVV9JBDE37nLxR

refresh_contract_parameters:
  p2s: 62TTAg5ZqAM7HwjB169cSSn844L2ZmMq4xjen6QcGt9Bb6ssYBgg3SNHuRiDqJmHYXbdgU179G5WiYPMq5VgUHDPbjJ814wuUgCARX1TiSavG2ycFFbJzqdfj7cmZHhNziVg5f7YsU365P49oCpVxmQoeVUnomDzKMrsn2M1VeAGrrnBdTBcjuSm6M1NURRNmfir3TsazHZbyPPrr6BajikkwxLQ9GiSWnY841qU8ZRHvhqVqiVZa13NiXqfTa36LBGQmsyYQuY4ThdgLkfgsNH24snGcx2UUyYsRSQfK8SWtVjbsjwRF4fDdST4St4ermWXP8JfPmXLFeg3m5NHEyd6W3WrCR2i14tenGKqU6aHzJ3ZCZCfKinaUFqr81NQ4kmH9kzpzs5KfXhyTKRd43jRuU2gP2hDLZ2N4isS1EYCsySp86yrM1VAP91f3sF9MHjUXXAXZHp4EvtnJdtkTgQxCYmV95XuEnBk1Kx2zNvqtERsRnyqbtvv78qY63DMCMiNNkPJeg1YbNk4Li7FzPokiNE4YfPZ3uQVEKzNdrgUCPuMh5ShcBmDV4v35AbbwWWQpfh6j7dLsXzDKMG5i4fuoctfthSBC8ipsbNo563zorHd6iyoFofLeAxNW9eTZC4oZdskhep1wu3BYyYCEdrmtUTXKueK3JTpnzb1uRKYNFWdQKMv2UwfxrxWwaDv7BxFbrRstAc4wznZkjtQ1C2VmKrjU3UuVv235r9tFwojEoCuRbGqtg2DzW
  pool_nft_index: 17
  oracle_token_id_index: 3
  min_data_points_index: 13
  min_data_points: 4
  buffer_index: 21
  buffer_length: 4
  max_deviation_percent_index: 15
  max_deviation_percent: 5
  epoch_length_index: 0
  epoch_length: 30
pool_contract_parameters:
  p2s: 3R221rmtBS5mwaKUTfwXpmGoUk4PjKLXuD6aRd1xyuCaRLNtzinwTGsZnhs4Fen3Rz46GYcgLezUe3Aunm7gZYWgYyEWCGfPYsmTptYtd8U2o2pp8NvNL7yQ79vJUamqqEtVt1i7eVHKAZarHP51n
  refresh_nft_index: 2
  update_nft_index: 3
update_contract_parameters:
  p2s: 3c2tfyDoE3VryrPkj4f6Drw4QCf4Cx2rHRwbMy3rp9JZwArEB7L18ePfXVpwxgPJ21E4vu5SnKzumuCSwgjUMxL6LTYncGWozPf71Xz1Bx8X2aUuNnkTxrJK9NJwuEyBdbnCiYv18a6Cbib5T9wovLuhDdcrdTsXbyyWy9iojqgvrApU4Ge31Xgxmir4sgVFX8pR6po1VCpSwbtvbP9pJyApVmkYGuWT7vMpoapPcsD4qRdn9cetGZL2Nz8dNvJyE4LRaE97VokTYFCpvM2NbAa8GkwXhBc4SsnFQKchR7PUch1CjZi4sVfBZL4Zma94UrpgP5oMNujzJx865mQsBm1h2dL87Dgfba81npafZxzDT9EU82UgWwhTYcnPiqZft5T9sQTsWiNtvMwTvXFVXAvfcPBr5meEkut2fx1p8ZmPLYoRGvDi9eLucRhpuhpAsLdBSS3iuHVn7bU4MAUYaquHD97CDFoTL2FZkfnELJHfqZMt3rtrjcC1VpMq2TgXPK5PsbsD9nEQYJTSex
  pool_nft_index: 5
  ballot_token_index: 9
  min_votes_index: 13
  min_votes: 6
ballot_contract_parameters:
  p2s: KKTr5Kf9nPN9o2FAhMHorL6oucAzavWXyqqzDhVVBPcbmtcSzCAWHXN4qeFJh58jbinfZcMCxHqrHp5GBVffxNxV2D1o91NimDxZVsgNjiGd1B5y5j9LsAixoU3GbmMeJKiXBvahu2emyLWQva3oWQaAPRGaSMY8fUeqvPNZcFqd2zgUTZ2gYWdDrsKZGK36mnTtio4F9kBkquPBt5VyQfGjjTjU3MhCRrKtg5UesyndY4mA
  min_storage_rent_index: 0
  min_storage_rent: 10000000
  update_nft_index: 6
oracle_contract_parameters:
    p2s: CxiAhQ241nmgPp39VgGTWPjKDNauyuu8p9Rc2x5pWf5znRwmFC3TbNqGBmBZmAhE6SUG2xMBpdLp3GZUQs9VEcrnHtCzpMqXcdsbtELqoPRAmHtNarK8VnfyhHzZijDGggbwEZzQRhw1U45N75BFYWrZSdgrgZLnpr5pC1Rs979hB8UTYHU2p1vPqPbJe44tvZ7E1mJJqqKFBdCR2hW8Ka7dCTWEv4yviUWRtHjpFnBCsFanMQ8R7YdHxh1Z36JFCscdvHvHCQsCZo7Z3AAdq8F5cwoWrZMY9eBP
    pool_nft_index: 5
ballot_parameters:
  contract_parameters:
    p2s: KKTr5Kf9nPN9o2FAhMHorL6oucAzavWXyqqzDhVVBPcbmtcSzCAWHXN4qeFJh58jbinfZcMCxHqrHp5GBVffxNxV2D1o91NimDxZVsgNjiGd1B5y5j9LsAixoU3GbmMeJKiXBvahu2emyLWQva3oWQaAPRGaSMY8fUeqvPNZcFqd2zgUTZ2gYWdDrsKZGK36mnTtio4F9kBkquPBt5VyQfGjjTjU3MhCRrKtg5UesyndY4mA
    min_storage_rent_index: 0
    min_storage_rent: 10000000
    update_nft_index: 6
  vote_parameters: ~
  ballot_token_owner_address: 3WzD3VNSK4RtDCZe8njzLzRnWbxcfpCneUcQncAVV9JBDE37nLxR").unwrap();
        let ctx = force_any_val::<ErgoStateContext>();
        let height = ctx.pre_header.height;
        let secret = force_any_val::<DlogProverInput>();
        let address = Address::P2Pk(secret.public_image());
        let wallet = Wallet::from_secrets(vec![secret.clone().into()]);
        let ergo_tree = address.script().unwrap();

        let value = BoxValue::SAFE_USER_MIN.checked_mul_u32(10000).unwrap();
        let unspent_boxes = vec![ErgoBox::new(
            value,
            ergo_tree.clone(),
            None,
            NonMandatoryRegisters::empty(),
            height - 9,
            force_any_val::<TxId>(),
            0,
        )
        .unwrap()];
        let change_address =
            AddressEncoder::new(ergo_lib::ergotree_ir::chain::address::NetworkPrefix::Mainnet)
                .parse_address_from_str("9iHyKxXs2ZNLMp9N9gbUT9V8gTbsV7HED1C1VhttMfBUMPDyF7r")
                .unwrap();

        let state = UpdateBootstrapConfig {
            tokens_to_mint: UpdateTokensToMint {
                refresh_nft: Some(NftMintDetails {
                    name: "refresh NFT".into(),
                    description: "refresh NFT".into(),
                }),
                update_nft: Some(NftMintDetails {
                    name: "update NFT".into(),
                    description: "update NFT".into(),
                }),
                oracle_tokens: Some(TokenMintDetails {
                    name: "oracle token".into(),
                    description: "oracle token".into(),
                    quantity: 15,
                }),
                ballot_tokens: Some(TokenMintDetails {
                    name: "ballot token".into(),
                    description: "ballot token".into(),
                    quantity: 15,
                }),
                reward_tokens: Some(TokenMintDetails {
                    name: "reward token".into(),
                    description: "reward token".into(),
                    quantity: 100_000_000,
                }),
            },
            refresh_contract_parameters: Some(RefreshContractParameters::default()),
            pool_contract_parameters: Some(PoolContractParameters::default()),
            update_contract_parameters: Some(UpdateContractParameters::default()),
            addresses: Addresses {
                address_for_oracle_tokens: address.clone(),
                wallet_address_for_chain_transaction: address.clone(),
            },
        };

        let height = ctx.pre_header.height;
        let submit_tx = SubmitTxMock::default();
        let oracle_config = perform_update_chained_transaction(UpdateBootstrapInput {
            config: state.clone(),
            wallet: &WalletDataMock {
                unspent_boxes: unspent_boxes.clone(),
            },
            tx_signer: &mut LocalTxSigner {
                ctx: &ctx,
                wallet: &wallet,
            },
            submit_tx: &submit_tx,
            tx_fee: BoxValue::SAFE_USER_MIN,
            erg_value_per_box: BoxValue::SAFE_USER_MIN,
            change_address,
            height,
            old_config: old_config.clone(),
        })
        .unwrap();

        assert!(oracle_config.token_ids != old_config.token_ids);
    }
}
