#![allow(unused_imports)]

use std::{
    cmp::max,
    convert::{TryFrom, TryInto},
    io::Write,
};

use derive_more::From;
use ergo_lib::{
    chain::{
        ergo_box::box_builder::{ErgoBoxCandidateBuilder, ErgoBoxCandidateBuilderError},
        transaction::Transaction,
    },
    ergo_chain_types::blake2b256_hash,
    ergotree_ir::{
        chain::{
            address::{Address, AddressEncoder, AddressEncoderError},
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
    box_kind::{PoolBox, PoolBoxWrapperInputs, RefreshBoxWrapperInputs, UpdateBoxWrapperInputs},
    contracts::{
        pool::{PoolContractError, PoolContractParameters},
        refresh::{
            RefreshContract, RefreshContractError, RefreshContractInputs, RefreshContractParameters,
        },
        update::{
            UpdateContract, UpdateContractError, UpdateContractInputs, UpdateContractParameters,
        },
    },
    node_interface::{new_node_interface, SignTransaction, SubmitTransaction},
    oracle_config::{OracleConfig, BASE_FEE, ORACLE_CONFIG},
    oracle_state::{OraclePool, StageDataSource},
    serde::{OracleConfigSerde, SerdeConversionError, UpdateBootstrapConfigSerde},
    spec_token::{
        BallotTokenId, OracleTokenId, RefreshTokenId, RewardTokenId, TokenIdKind, UpdateTokenId,
    },
    wallet::{WalletDataError, WalletDataSource},
};

use super::bootstrap::{NftMintDetails, TokenMintDetails};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UpdateTokensToMint {
    pub refresh_nft: Option<NftMintDetails>,
    pub update_nft: Option<NftMintDetails>,
    pub oracle_tokens: Option<TokenMintDetails>,
    pub ballot_tokens: Option<TokenMintDetails>,
    pub reward_tokens: Option<TokenMintDetails>,
}

#[derive(Clone)]
pub struct UpdateBootstrapConfig {
    pub pool_contract_parameters: Option<PoolContractParameters>, // New pool script, etc. Note that we don't actually mint any new pool NFT in the update step, instead this is simply passed to the new oracle config for convenience
    pub refresh_contract_parameters: Option<RefreshContractParameters>,
    pub update_contract_parameters: Option<UpdateContractParameters>,
    pub tokens_to_mint: UpdateTokensToMint,
}

pub fn prepare_update(config_file_name: String) -> Result<(), PrepareUpdateError> {
    let s = std::fs::read_to_string(config_file_name)?;
    let config_serde: UpdateBootstrapConfigSerde = serde_yaml::from_str(&s)?;

    let node_interface = new_node_interface();
    let change_address = AddressEncoder::unchecked_parse_address_from_str(
        &node_interface
            .wallet_status()?
            .change_address
            .ok_or(PrepareUpdateError::NoChangeAddressSetInNode)?,
    )?;
    let config = UpdateBootstrapConfig::try_from(config_serde)?;
    let update_bootstrap_input = PrepareUpdateInput {
        config: config.clone(),
        wallet: &node_interface,
        tx_signer: &node_interface,
        submit_tx: &node_interface,
        tx_fee: *BASE_FEE,
        erg_value_per_box: *BASE_FEE,
        change_address,
        height: node_interface
            .current_block_height()
            .unwrap()
            .try_into()
            .unwrap(),
        old_config: ORACLE_CONFIG.clone(),
    };

    let new_config = perform_update_chained_transaction(update_bootstrap_input)?;
    let blake2b_pool_ergo_tree: String = blake2b256_hash(
        new_config
            .pool_box_wrapper_inputs
            .contract_inputs
            .contract_parameters()
            .ergo_tree_bytes()
            .as_slice(),
    )
    .into();

    info!("Update chain-transaction complete");
    info!("Writing new config file to oracle_config_updated.yaml");
    let config = OracleConfigSerde::from(new_config);
    let s = serde_yaml::to_string(&config)?;
    let mut file = std::fs::File::create("oracle_config_updated.yaml")?;
    file.write_all(s.as_bytes())?;
    info!("Updated oracle configuration file oracle_config_updated.yaml");
    info!(
        "Base16-encoded blake2b hash of the serialized new pool box contract(ErgoTree): {}",
        blake2b_pool_ergo_tree
    );
    print_hints_for_voting()?;
    Ok(())
}

fn print_hints_for_voting() -> Result<(), PrepareUpdateError> {
    let epoch_length = ORACLE_CONFIG
        .refresh_box_wrapper_inputs
        .contract_inputs
        .contract_parameters()
        .epoch_length()
        .0 as u32;
    let current_height: u32 = new_node_interface().current_block_height()? as u32;
    let op = OraclePool::new().unwrap();
    let oracle_boxes = op.datapoint_stage.stage.get_boxes().unwrap();
    let min_oracle_box_height = current_height - epoch_length;
    let active_oracle_count = oracle_boxes
        .into_iter()
        .filter(|b| b.creation_height as u32 >= min_oracle_box_height)
        .count() as u32;
    let pool_box = op.get_pool_box_source().get_pool_box().unwrap();
    let pool_box_height = pool_box.get_box().creation_height;
    let next_epoch_height = max(pool_box_height + epoch_length, current_height);
    let reward_tokens_left = *pool_box.reward_token().amount.as_u64();
    let update_box = op.get_update_box_source().get_update_box().unwrap();
    let update_box_height = update_box.get_box().creation_height;
    info!("Update box height: {}", update_box_height);
    info!(
        "Reward token id in the pool box: {}",
        String::from(pool_box.reward_token().token_id.token_id())
    );
    info!(
        "Current height is {}, pool box height (epoch start) {}, epoch length is {}",
        current_height, pool_box_height, epoch_length
    );
    info!(
        "Estimated active oracle count is {}, reward tokens in the pool box {}",
        active_oracle_count, reward_tokens_left
    );
    for i in 0..10 {
        info!(
            "On new epoch height {} estimating reward tokens in the pool box: {}",
            next_epoch_height + i * (epoch_length + 1),
            reward_tokens_left - ((i + 1) * (active_oracle_count * 2)) as u64
        );
    }
    Ok(())
}

struct PrepareUpdateInput<'a> {
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

fn perform_update_chained_transaction(
    input: PrepareUpdateInput,
) -> Result<OracleConfig, PrepareUpdateError> {
    let PrepareUpdateInput {
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

    let wallet_pk_ergo_tree = old_config.oracle_address.address().script()?;
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
     -> Result<(Token, Transaction), PrepareUpdateError> {
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
        new_oracle_config.token_ids.oracle_token_id =
            OracleTokenId::from_token_id_unchecked(token.token_id);
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
        new_oracle_config.token_ids.ballot_token_id =
            BallotTokenId::from_token_id_unchecked(token.token_id);
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
        new_oracle_config.token_ids.reward_token_id =
            RewardTokenId::from_token_id_unchecked(token.token_id);
        inputs = filter_tx_outputs(tx.outputs.clone()).try_into().unwrap();
        transactions.push(tx);
    }
    if let Some(ref contract_parameters) = config.refresh_contract_parameters {
        info!("Creating new refresh NFT");
        let refresh_contract_inputs = RefreshContractInputs::build_with(
            contract_parameters.clone(),
            new_oracle_config.token_ids.oracle_token_id.clone(),
            old_config.token_ids.pool_nft_token_id,
        )?;
        let refresh_contract = RefreshContract::checked_load(&refresh_contract_inputs)?;
        let refresh_nft_details = config
            .tokens_to_mint
            .refresh_nft
            .ok_or(PrepareUpdateError::NoMintDetails)?;
        let (token, tx) = mint_token(
            inputs.into(),
            &mut num_transactions_left,
            refresh_nft_details.name.clone(),
            refresh_nft_details.description.clone(),
            1.try_into().unwrap(),
            Some(refresh_contract.ergo_tree()),
        )?;
        new_oracle_config.token_ids.refresh_nft_token_id =
            RefreshTokenId::from_token_id_unchecked(token.token_id.clone());
        new_oracle_config.refresh_box_wrapper_inputs = RefreshBoxWrapperInputs {
            contract_inputs: refresh_contract_inputs,
            refresh_nft_token_id: new_oracle_config.token_ids.refresh_nft_token_id.clone(),
        };
        inputs = filter_tx_outputs(tx.outputs.clone()).try_into().unwrap();
        info!("Refresh contract tx id: {:?}", tx.id());
        transactions.push(tx);
    }
    if let Some(ref contract_parameters) = config.update_contract_parameters {
        info!("Creating new update NFT");
        let update_contract_inputs = UpdateContractInputs::build_with(
            contract_parameters.clone(),
            new_oracle_config.token_ids.pool_nft_token_id.clone(),
            new_oracle_config.token_ids.ballot_token_id.clone(),
        )?;
        let update_contract = UpdateContract::checked_load(&update_contract_inputs)?;
        let update_nft_details = config
            .tokens_to_mint
            .update_nft
            .ok_or(PrepareUpdateError::NoMintDetails)?;
        let (token, tx) = mint_token(
            inputs.into(),
            &mut num_transactions_left,
            update_nft_details.name.clone(),
            update_nft_details.description.clone(),
            1.try_into().unwrap(),
            Some(update_contract.ergo_tree()),
        )?;
        new_oracle_config.token_ids.update_nft_token_id =
            UpdateTokenId::from_token_id_unchecked(token.token_id.clone());
        new_oracle_config.update_box_wrapper_inputs = UpdateBoxWrapperInputs {
            contract_inputs: update_contract_inputs,
            update_nft_token_id: new_oracle_config.token_ids.update_nft_token_id.clone(),
        };
        info!("Update contract tx id: {:?}", tx.id());
        transactions.push(tx);
    }

    if let Some(new_pool_contract_parameters) = config.pool_contract_parameters {
        let new_pool_box_wrapper_inputs = PoolBoxWrapperInputs::build_with(
            new_pool_contract_parameters,
            new_oracle_config.token_ids.refresh_nft_token_id.clone(),
            new_oracle_config.token_ids.update_nft_token_id.clone(),
            new_oracle_config.token_ids.pool_nft_token_id.clone(),
            new_oracle_config.token_ids.reward_token_id.clone(),
        )?;
        new_oracle_config.pool_box_wrapper_inputs = new_pool_box_wrapper_inputs;
    } else if new_oracle_config.token_ids.refresh_nft_token_id
        != old_config.token_ids.refresh_nft_token_id
        || new_oracle_config.token_ids.update_nft_token_id
            != old_config.token_ids.update_nft_token_id
    {
        return Err(PrepareUpdateError::PoolContractParametersNotProvided);
    }

    for tx in transactions {
        let tx_id = submit_tx.submit_transaction(&tx)?;
        info!("Tx submitted {}", tx_id);
    }
    Ok(new_oracle_config)
}

#[derive(Debug, Error, From)]
pub enum PrepareUpdateError {
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
    #[error("Pool contract failed: {0}")]
    PoolContract(PoolContractError),
    #[error("Bootstrap config file already exists")]
    ConfigFilenameAlreadyExists,
    #[error("No parameters were added for update")]
    NoOpUpgrade,
    #[error("No mint details were provided for update/refresh contract in tokens_to_mint")]
    NoMintDetails,
    #[error("Serde conversion error {0}")]
    SerdeConversion(SerdeConversionError),
    #[error("WalletData error: {0}")]
    WalletData(WalletDataError),
    #[error("new tokens minted, pool contract has to be updated as well, please provide pool_contract_parameters")]
    PoolContractParametersNotProvided,
}

#[cfg(test)]
mod test {
    use ergo_lib::{
        chain::{ergo_state_context::ErgoStateContext, transaction::TxId},
        ergotree_interpreter::sigma_protocol::private_input::DlogProverInput,
        ergotree_ir::chain::{
            address::{AddressEncoder, NetworkAddress, NetworkPrefix},
            ergo_box::{ErgoBox, NonMandatoryRegisters},
        },
        wallet::Wallet,
    };
    use sigma_test_util::force_any_val;

    use super::*;
    use crate::cli_commands::bootstrap::tests::SubmitTxMock;
    use crate::pool_commands::test_utils::{LocalTxSigner, WalletDataMock};

    #[test]
    fn test_prepare_update_transaction() {
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
rescan_height: 0

data_point_source: NanoErgUsd
addresses:
  wallet_address_for_chain_transaction: 3WzD3VNSK4RtDCZe8njzLzRnWbxcfpCneUcQncAVV9JBDE37nLxR
  ballot_token_owner_address: 3WzD3VNSK4RtDCZe8njzLzRnWbxcfpCneUcQncAVV9JBDE37nLxR

refresh_contract_parameters:
  ergo_tree_bytes: 1016043c040004000e206e5c5a93e24ba3bdcd2b510d8623a9c6e74efe7e0e6d20c114c47a19b299f57f01000502010105000400040004020402040204080400040a05c8010e2014717f917cdb1951fc378e31ffc0a0a778bddb10d6509c201cb4ed2550ef9f830400040404020408d80ed60199a37300d602b2a4730100d603b5a4d901036395e6c672030605eded928cc77203017201938cb2db6308720373020001730393e4c672030504e4c6720205047304d604b17203d605b0720386027305860273067307d901053c413d0563d803d607e4c68c7205020605d6088c720501d6098c720802860272078602ed8c720901908c72080172079a8c7209027207d6068c720502d6078c720501d608db63087202d609b27208730800d60ab2a5730900d60bdb6308720ad60cb2720b730a00d60db27208730b00d60eb2a5730c00ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02cde4c6b27203e4e30004000407d18f8cc77202017201d1927204730dd18c720601d190997207e4c6b27203730e0006059d9c72077e730f057310d1938c7209017311d193b2720b7312007209d1938c720c018c720d01d1928c720c02998c720d027e9c7204731305d193b1720bb17208d193e4c6720a04059d8c7206027e720405d193e4c6720a05049ae4c6720205047314d193c2720ac27202d192c1720ac17202d1928cc7720a0199a37315d193db6308720edb6308a7d193c2720ec2a7d192c1720ec1a7
  pool_nft_index: 17
  oracle_token_id_index: 3
  min_data_points_index: 13
  min_data_points: 4
  buffer_length_index: 21
  buffer_length: 4
  max_deviation_percent_index: 15
  max_deviation_percent: 5
  epoch_length_index: 0
  epoch_length: 30
pool_contract_parameters:
  ergo_tree_bytes: 1004040204000e202f9111945d8f05f5c12732743e601b1de802fe741c38065e6a6372e1329c96fa0e2048e025a1e3e789adceddc5d212590b0f1f62113c4882795712d542a866d117e8d801d6018cb2db6308b2a473000073010001d1ec93720173029372017303
  refresh_nft_index: 2
  update_nft_index: 3
update_contract_parameters:
  ergo_tree_bytes: 100e040004000400040204020e2014717f917cdb1951fc378e31ffc0a0a778bddb10d6509c201cb4ed2550ef9f830400040004000e20b1f2f35d727bf21c599c7ea151184d91df7567c4b1423e54b7fb93cfdc77f810010005000400040cd806d601b2a4730000d602b2db63087201730100d603b2a5730200d604db63087203d605b2a5730300d606b27204730400d1ededed938c7202017305ededededed937202b27204730600938cc77201018cc772030193c17201c1720393c672010405c67203040593c672010504c672030504efe6c672030661edededed93db63087205db6308a793c27205c2a792c17205c1a7918cc77205018cc7a701efe6c67205046192b0b5a4d9010763d801d609db630872079591b172097307edededed938cb2720973080001730993e4c6720705048cc7a70193e4c67207060ecbc2720393e4c67207070e8c72060193e4c6720708058c720602730a730bd9010741639a8c7207018cb2db63088c720702730c00027e730d05
  pool_nft_index: 5
  ballot_token_index: 9
  min_votes_index: 13
  min_votes: 6
ballot_contract_parameters:
  ergo_tree_bytes: 10070580dac409040204020400040204000e2048e025a1e3e789adceddc5d212590b0f1f62113c4882795712d542a866d117e8d803d601b2a5e4e3000400d602c672010407d603e4c6a70407ea02d1ededede6720293c27201c2a793db63087201db6308a792c172017300eb02cd7203d1ededededed91b1a4730191b1db6308b2a47302007303938cb2db6308b2a473040073050001730693e47202720392c17201c1a7efe6c672010561
  min_storage_rent_index: 0
  min_storage_rent: 10000000
  update_nft_index: 6
oracle_contract_parameters:
    ergo_tree_bytes: 100a040004000580dac409040004000e2014717f917cdb1951fc378e31ffc0a0a778bddb10d6509c201cb4ed2550ef9f830402040204020402d804d601b2a5e4e3000400d602db63087201d603db6308a7d604e4c6a70407ea02d1ededed93b27202730000b2720373010093c27201c2a7e6c67201040792c172017302eb02cd7204d1ededededed938cb2db6308b2a4730300730400017305938cb27202730600018cb2720373070001918cb27202730800028cb272037309000293e4c672010407720492c17201c1a7efe6c672010561
    pool_nft_index: 5
    min_storage_rent_index: 2
    min_storage_rent: 10000000
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
        let network_address = NetworkAddress::new(
            NetworkPrefix::Testnet,
            &Address::P2Pk(secret.public_image()),
        );
        let old_config = OracleConfig {
            oracle_address: network_address.clone(),
            ..old_config
        };
        let wallet = Wallet::from_secrets(vec![secret.clone().into()]);
        let ergo_tree = network_address.address().script().unwrap();

        let value = BASE_FEE.checked_mul_u32(10000).unwrap();
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
        };

        let height = ctx.pre_header.height;
        let submit_tx = SubmitTxMock::default();
        let oracle_config = perform_update_chained_transaction(PrepareUpdateInput {
            config: state.clone(),
            wallet: &WalletDataMock {
                unspent_boxes: unspent_boxes.clone(),
            },
            tx_signer: &mut LocalTxSigner {
                ctx: &ctx,
                wallet: &wallet,
            },
            submit_tx: &submit_tx,
            tx_fee: *BASE_FEE,
            erg_value_per_box: *BASE_FEE,
            change_address,
            height,
            old_config: old_config.clone(),
        })
        .unwrap();

        assert!(oracle_config.token_ids != old_config.token_ids);
    }
}
