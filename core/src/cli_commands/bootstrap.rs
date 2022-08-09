//! Bootstrap a new oracle pool
use std::{convert::TryInto, io::Write, path::Path};

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
use ergo_node_interface::{node_interface::NodeError, NodeInterface};
use log::{debug, info};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    box_kind::{make_pool_box_candidate, make_refresh_box_candidate, RefreshBoxWrapperInputs},
    contracts::{
        ballot::BallotContractParameters,
        pool::{PoolContract, PoolContractInputs, PoolContractParameters},
        refresh::{RefreshContract, RefreshContractError, RefreshContractParameters},
        update::{
            UpdateContract, UpdateContractError, UpdateContractInputs, UpdateContractParameters,
        },
    },
    node_interface::{assert_wallet_unlocked, SignTransaction, SubmitTransaction},
    oracle_config::TokenIds,
    wallet::WalletDataSource,
};

/// Loads bootstrap configuration file and performs the chain-transactions for minting of tokens and
/// box creations. An oracle configuration file is then created which contains the `TokenId`s of the
/// minted tokens.
pub fn bootstrap(config_file_name: String) -> Result<(), BootstrapError> {
    let s = std::fs::read_to_string(config_file_name)?;
    let config: BootstrapConfig = serde_yaml::from_str(&s)?;

    // We can't call any functions from the `crate::node_interface` module because we don't have an
    // `oracle_config.yaml` file to work from here.
    let node = NodeInterface::new(&config.node_api_key, &config.node_ip, &config.node_port);
    assert_wallet_unlocked(&node);
    let prefix = if config.on_mainnet {
        NetworkPrefix::Mainnet
    } else {
        NetworkPrefix::Testnet
    };
    let change_address_str = node
        .wallet_status()?
        .change_address
        .ok_or(BootstrapError::NoChangeAddressSetInNode)?;
    debug!("Change address: {}", change_address_str);

    let change_address = AddressEncoder::new(prefix).parse_address_from_str(&change_address_str)?;
    let input = BootstrapInput {
        config,
        wallet: &node as &dyn WalletDataSource,
        tx_signer: &node as &dyn SignTransaction,
        submit_tx: &node as &dyn SubmitTransaction,
        tx_fee: BoxValue::SAFE_USER_MIN,
        erg_value_per_box: BoxValue::SAFE_USER_MIN,
        change_address,
        height: node.current_block_height()? as u32,
    };
    let oracle_config = perform_bootstrap_chained_transaction(input)?;
    info!("Bootstrap chain-transaction complete");
    let s = serde_yaml::to_string(&oracle_config)?;
    let mut file = std::fs::File::create(crate::oracle_config::DEFAULT_CONFIG_FILE_NAME)?;
    file.write_all(s.as_bytes())?;
    info!(
        "Oracle configuration file created: {}",
        crate::oracle_config::DEFAULT_CONFIG_FILE_NAME
    );
    Ok(())
}

pub fn generate_bootstrap_config_template(config_file_name: String) -> Result<(), BootstrapError> {
    if Path::new(&config_file_name).exists() {
        return Err(BootstrapError::ConfigFilenameAlreadyExists);
    }
    let address = AddressEncoder::new(NetworkPrefix::Mainnet)
        .parse_address_from_str("9hEQHEMyY1K1vs79vJXFtNjr2dbQbtWXF99oVWGJ5c4xbcLdBsw")?;
    let config = BootstrapConfig {
        tokens_to_mint: TokensToMint {
            pool_nft: NftMintDetails {
                name: "pool NFT".into(),
                description: "Pool NFT".into(),
            },
            refresh_nft: NftMintDetails {
                name: "refresh NFT".into(),
                description: "refresh NFT".into(),
            },
            update_nft: NftMintDetails {
                name: "update NFT".into(),
                description: "update NFT".into(),
            },
            oracle_tokens: TokenMintDetails {
                name: "oracle token".into(),
                description: "oracle token".into(),
                quantity: 15,
            },
            ballot_tokens: TokenMintDetails {
                name: "ballot token".into(),
                description: "ballot token".into(),
                quantity: 15,
            },
            reward_tokens: TokenMintDetails {
                name: "reward token".into(),
                description: "reward token".into(),
                quantity: 100_000_000,
            },
        },
        addresses: Addresses {
            address_for_oracle_tokens: address.clone(),
            wallet_address_for_chain_transaction: address,
        },
        node_ip: "127.0.0.1".into(),
        node_port: "9053".into(),
        node_api_key: "hello".into(),
        on_mainnet: true,
        refresh_contract_parameters: RefreshContractParameters::default(),
        pool_contract_parameters: PoolContractParameters::default(),
        update_contract_parameters: UpdateContractParameters::default(),
        ballot_contract_parameters: BallotContractParameters::default(),
    };

    let s = serde_yaml::to_string(&config)?;
    let mut file = std::fs::File::create(&config_file_name)?;
    file.write_all(s.as_bytes())?;
    Ok(())
}

pub struct BootstrapInput<'a> {
    pub config: BootstrapConfig,
    pub wallet: &'a dyn WalletDataSource,
    pub tx_signer: &'a dyn SignTransaction,
    pub submit_tx: &'a dyn SubmitTransaction,
    pub tx_fee: BoxValue,
    pub erg_value_per_box: BoxValue,
    pub change_address: Address,
    pub height: u32,
}

/// Perform and submit to the mempool the chained-transaction to boostrap the oracle pool. We first
/// mint the oracle-pool tokens then create the pool and refresh boxes as described in EIP-23:
/// https://github.com/ergoplatform/eips/blob/eip23/eip-0023.md#tokens
pub(crate) fn perform_bootstrap_chained_transaction(
    input: BootstrapInput,
) -> Result<OracleConfigFields, BootstrapError> {
    let BootstrapInput {
        config,
        wallet,
        tx_signer: wallet_sign,
        submit_tx,
        tx_fee,
        erg_value_per_box,
        change_address,
        height,
        ..
    } = input;

    // We can calculate the amount of ERGs necessary to effect this chained-transaction upfront.
    // We're going to mint 6 distinct types of tokens and create the pool and refresh boxes as
    // described in EIP-23. The minting of each type of token requires a distinct transaction, so we
    // need 8 transactions in total. We assume that the resulting token-holding boxes generated from
    // these transactions each has a box value of `erg_value_per_box`. Similarly the pool and
    // refresh boxes will also hold `erg_value_per_box`.
    //
    // Now define `E_i = i*(erg_value_per_box + tx_fee)` for `i = 1,2,.., 8`. `E_i` represents the
    // amount of ERGs necessary to effect `i` remaining transactions.
    //
    // So we require a total ERG value of `E_8 = 8*(erg_value_per_box + tx_fee)`
    //
    // The chain transaction is structured as follows:
    //   * First sweep the unspent boxes of the wallet for a target balance of `E_8`. Denote these
    //     input boxes by `I_1`.
    //
    //   * Mint the first token with `I_1` as input, resulting in two output boxes:
    //      - `B_1_token` containing the minted token and `ergo_value_per_box`
    //      - `B_1_remaining` containing `E_7` in ERG value.
    //
    //   * Mint the second token with input boxes containing `B_1_remaining`, resulting in two
    //     output boxes:
    //      - `B_2_token` containing the minted token and `ergo_value_per_box`
    //      - `B_2_remaining` containing `E_6` in ERG value.
    //
    // And so on.

    // This variable represents the index `i` described above.
    let mut num_transactions_left = 8;

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
     -> Result<(Token, Transaction), BootstrapError> {
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
        builder.mint_token(token.clone(), token_name, token_desc, 0);
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

    // Mint pool NFT token --------------------------------------------------------------------------
    info!("Minting pool NFT tx");
    let unspent_boxes = wallet.get_unspent_wallet_boxes()?;
    debug!("unspent boxes: {:?}", unspent_boxes);
    let target_balance = calc_target_balance(num_transactions_left)?;
    debug!("target_balance: {:?}", target_balance);
    let box_selector = SimpleBoxSelector::new();
    let box_selection = box_selector.select(unspent_boxes.clone(), target_balance, &[])?;
    debug!("box selection: {:?}", box_selection);

    let (pool_nft_token, signed_mint_pool_nft_tx) = mint_token(
        box_selection.boxes.as_vec().clone(),
        &mut num_transactions_left,
        config.tokens_to_mint.pool_nft.name.clone(),
        config.tokens_to_mint.pool_nft.description.clone(),
        1.try_into().unwrap(),
        None,
    )?;
    debug!("signed_mint_pool_nft_tx: {:?}", signed_mint_pool_nft_tx);

    // Mint refresh NFT token ----------------------------------------------------------------------
    info!("Minting refresh NFT tx");
    let inputs = filter_tx_outputs(signed_mint_pool_nft_tx.outputs.clone());
    debug!("inputs for refresh NFT mint: {:?}", inputs);
    let (refresh_nft_token, signed_mint_refresh_nft_tx) = mint_token(
        inputs,
        &mut num_transactions_left,
        config.tokens_to_mint.refresh_nft.name.clone(),
        config.tokens_to_mint.refresh_nft.description.clone(),
        1.try_into().unwrap(),
        None,
    )?;
    debug!(
        "signed_mint_refresh_nft_tx: {:?}",
        signed_mint_refresh_nft_tx
    );

    // Mint ballot tokens --------------------------------------------------------------------------
    info!("Minting ballot tokens tx");
    let inputs = filter_tx_outputs(signed_mint_refresh_nft_tx.outputs.clone());
    debug!("inputs for ballot tokens mint: {:?}", inputs);
    let (ballot_token, signed_mint_ballot_tokens_tx) = mint_token(
        inputs,
        &mut num_transactions_left,
        config.tokens_to_mint.ballot_tokens.name.clone(),
        config.tokens_to_mint.ballot_tokens.description.clone(),
        config
            .tokens_to_mint
            .ballot_tokens
            .quantity
            .try_into()
            .unwrap(),
        None,
    )?;
    debug!(
        "signed_mint_ballot_tokens_tx: {:?}",
        signed_mint_ballot_tokens_tx
    );

    // Mint update NFT token -----------------------------------------------------------------------

    let inputs = UpdateContractInputs {
        pool_nft_token_id: &pool_nft_token.token_id,
        ballot_token_id: &ballot_token.token_id,
        contract_parameters: &config.update_contract_parameters,
    };
    let update_contract = UpdateContract::new(inputs)?;

    info!("Minting update NFT tx");
    let inputs = filter_tx_outputs(signed_mint_ballot_tokens_tx.outputs.clone());
    debug!("inputs for update NFT mint: {:?}", inputs);
    let (update_nft_token, signed_mint_update_nft_tx) = mint_token(
        inputs,
        &mut num_transactions_left,
        config.tokens_to_mint.update_nft.name.clone(),
        config.tokens_to_mint.update_nft.description.clone(),
        1.try_into().unwrap(),
        Some(update_contract.ergo_tree()),
    )?;
    debug!("signed_mint_update_nft_tx: {:?}", signed_mint_update_nft_tx);

    // Mint oracle tokens --------------------------------------------------------------------------
    info!("Minting oracle tokens tx");
    let inputs = filter_tx_outputs(signed_mint_update_nft_tx.outputs.clone());
    debug!("inputs for oracle tokens mint: {:?}", inputs);
    let oracle_tokens_pk_ergo_tree = config.addresses.address_for_oracle_tokens.script()?;
    let (oracle_token, signed_mint_oracle_tokens_tx) = mint_token(
        inputs,
        &mut num_transactions_left,
        config.tokens_to_mint.oracle_tokens.name.clone(),
        config.tokens_to_mint.oracle_tokens.description.clone(),
        config
            .tokens_to_mint
            .oracle_tokens
            .quantity
            .try_into()
            .unwrap(),
        Some(oracle_tokens_pk_ergo_tree),
    )?;
    debug!(
        "signed_mint_oracle_tokens_tx: {:?}",
        signed_mint_oracle_tokens_tx
    );

    // Mint reward tokens --------------------------------------------------------------------------
    info!("Minting reward tokens tx");
    let inputs = filter_tx_outputs(signed_mint_oracle_tokens_tx.outputs.clone());
    debug!("inputs for reward tokens mint: {:?}", inputs);
    let (reward_token, signed_mint_reward_tokens_tx) = mint_token(
        inputs,
        &mut num_transactions_left,
        config.tokens_to_mint.reward_tokens.name.clone(),
        config.tokens_to_mint.reward_tokens.description.clone(),
        config
            .tokens_to_mint
            .reward_tokens
            .quantity
            .try_into()
            .unwrap(),
        None,
    )?;

    // Create pool box -----------------------------------------------------------------------------
    info!("Create pool box tx");

    let token_ids = TokenIds {
        pool_nft_token_id: pool_nft_token.token_id.clone(),
        refresh_nft_token_id: refresh_nft_token.token_id.clone(),
        update_nft_token_id: update_nft_token.token_id.clone(),
        oracle_token_id: oracle_token.token_id.clone(),
        reward_token_id: reward_token.token_id.clone(),
        ballot_token_id: ballot_token.token_id.clone(),
    };

    let pool_contract_parameters = PoolContractParameters {
        p2s: config.pool_contract_parameters.p2s,
        refresh_nft_index: config.pool_contract_parameters.refresh_nft_index,
        update_nft_index: config.pool_contract_parameters.update_nft_index,
    };
    let pool_contract = PoolContract::new(PoolContractInputs::from((
        &pool_contract_parameters,
        &token_ids,
    )))
    .unwrap();

    let reward_tokens_for_pool_box = Token {
        token_id: reward_token.token_id.clone(),
        amount: reward_token
            .amount
            // we must leave one reward token per oracle for their first datapoint box
            .checked_sub(&oracle_token.amount)
            .unwrap(),
    };
    let pool_box_candidate = make_pool_box_candidate(
        &pool_contract,
        // We intentionally set the initial datapoint to be 0, as it's treated as 'undefined' during bootstrap.
        0,
        1,
        pool_nft_token.clone(),
        reward_tokens_for_pool_box,
        erg_value_per_box,
        height,
    )?;
    let mut output_candidates = vec![pool_box_candidate];

    // Build box for remaining funds
    let builder = ErgoBoxCandidateBuilder::new(
        calc_target_balance(num_transactions_left - 1)?,
        wallet_pk_ergo_tree.clone(),
        height,
    );
    output_candidates.push(builder.build()?);

    let target_balance = calc_target_balance(num_transactions_left)?;
    let box_selector = SimpleBoxSelector::new();
    let mut inputs = filter_tx_outputs(signed_mint_reward_tokens_tx.outputs.clone());

    // Need to find the box containing the pool NFT, and transfer this token to the pool box.
    let box_with_pool_nft = signed_mint_pool_nft_tx
        .outputs
        .iter()
        .find(|b| {
            if let Some(tokens) = &b.tokens {
                tokens.iter().any(|t| t.token_id == pool_nft_token.token_id)
            } else {
                false
            }
        })
        .unwrap()
        .clone();
    inputs.push(box_with_pool_nft);

    let box_selection = box_selector.select(
        inputs,
        target_balance,
        &[pool_nft_token.clone(), reward_token.clone()],
    )?;
    let inputs = box_selection.boxes.clone();
    let tx_builder = TxBuilder::new(
        box_selection,
        output_candidates,
        height,
        tx_fee,
        change_address.clone(),
        BoxValue::MIN,
    );
    let pool_box_tx = tx_builder.build()?;
    debug!("unsigned pool_box_tx: {:?}", pool_box_tx);
    let signed_pool_box_tx =
        wallet_sign.sign_transaction_with_inputs(&pool_box_tx, inputs, None)?;
    num_transactions_left -= 1;

    // Create refresh box --------------------------------------------------------------------------
    info!("Create refresh box tx");

    let inputs = RefreshBoxWrapperInputs {
        contract_parameters: &config.refresh_contract_parameters,
        refresh_nft_token_id: &token_ids.refresh_nft_token_id,
        oracle_token_id: &token_ids.oracle_token_id,
        pool_nft_token_id: &token_ids.pool_nft_token_id,
    };
    let refresh_contract = RefreshContract::new(inputs.into())?;

    let refresh_box_candidate = make_refresh_box_candidate(
        &refresh_contract,
        refresh_nft_token.clone(),
        erg_value_per_box,
        height,
    )?;

    let output_candidates = vec![refresh_box_candidate];

    let target_balance = calc_target_balance(num_transactions_left)?;
    let box_selector = SimpleBoxSelector::new();
    let mut inputs = filter_tx_outputs(signed_pool_box_tx.outputs.clone());

    // Need to find the box containing the refresh NFT, and transfer this token to the refresh box.
    let box_with_refresh_nft = signed_mint_refresh_nft_tx
        .outputs
        .iter()
        .find(|b| {
            if let Some(tokens) = &b.tokens {
                tokens
                    .iter()
                    .any(|t| t.token_id == refresh_nft_token.token_id)
            } else {
                false
            }
        })
        .unwrap()
        .clone();
    inputs.push(box_with_refresh_nft);

    let box_selection =
        box_selector.select(inputs, target_balance, &[refresh_nft_token.clone()])?;
    let inputs = box_selection.boxes.clone();
    let tx_builder = TxBuilder::new(
        box_selection,
        output_candidates,
        height,
        tx_fee,
        change_address.clone(),
        BoxValue::MIN,
    );
    let refresh_box_tx = tx_builder.build()?;
    debug!("unsigned refresh_box_tx: {:?}", refresh_box_tx);
    let signed_refresh_box_tx =
        wallet_sign.sign_transaction_with_inputs(&refresh_box_tx, inputs, None)?;

    // ---------------------------------------------------------------------------------------------
    let tx_id = submit_tx.submit_transaction(&signed_mint_pool_nft_tx)?;
    info!("Minting pool NFT TxId: {}", tx_id);
    let tx_id = submit_tx.submit_transaction(&signed_mint_refresh_nft_tx)?;
    info!("Minting refresh NFT TxId: {}", tx_id);
    let tx_id = submit_tx.submit_transaction(&signed_mint_ballot_tokens_tx)?;
    info!("Minting ballot tokens TxId: {}", tx_id);
    let tx_id = submit_tx.submit_transaction(&signed_mint_update_nft_tx)?;
    info!("Minting update NFT TxId: {}", tx_id);
    let tx_id = submit_tx.submit_transaction(&signed_mint_oracle_tokens_tx)?;
    info!("Minting oracle tokens TxId: {}", tx_id);
    let tx_id = submit_tx.submit_transaction(&signed_mint_reward_tokens_tx)?;
    info!("Minting reward tokens TxId: {}", tx_id);
    let tx_id = submit_tx.submit_transaction(&signed_pool_box_tx)?;
    info!("Creating initial pool box TxId: {}", tx_id);
    let tx_id = submit_tx.submit_transaction(&signed_refresh_box_tx)?;
    info!("Creating initial refresh box TxId: {}", tx_id);

    Ok(OracleConfigFields {
        token_ids: TokenIds {
            pool_nft_token_id: pool_nft_token.token_id,
            refresh_nft_token_id: refresh_nft_token.token_id,
            update_nft_token_id: update_nft_token.token_id,
            oracle_token_id: oracle_token.token_id,
            reward_token_id: reward_token.token_id,
            ballot_token_id: ballot_token.token_id,
        },
        node_ip: config.node_ip,
        node_port: config.node_port,
        node_api_key: config.node_api_key,
    })
}

/// An instance of this struct is created from an operator-provided YAML file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    try_from = "crate::serde::BootstrapConfigSerde",
    into = "crate::serde::BootstrapConfigSerde"
)]
pub struct BootstrapConfig {
    pub refresh_contract_parameters: RefreshContractParameters,
    pub pool_contract_parameters: PoolContractParameters,
    pub update_contract_parameters: UpdateContractParameters,
    pub ballot_contract_parameters: BallotContractParameters,
    pub tokens_to_mint: TokensToMint,
    pub node_ip: String,
    pub node_port: String,
    pub node_api_key: String,
    pub on_mainnet: bool,
    pub addresses: Addresses,
}

#[derive(Clone, Debug)]
pub struct Addresses {
    pub address_for_oracle_tokens: Address,
    pub wallet_address_for_chain_transaction: Address,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokensToMint {
    pub pool_nft: NftMintDetails,
    pub refresh_nft: NftMintDetails,
    pub update_nft: NftMintDetails,
    pub oracle_tokens: TokenMintDetails,
    pub ballot_tokens: TokenMintDetails,
    pub reward_tokens: TokenMintDetails,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TokenMintDetails {
    pub name: String,
    pub description: String,
    pub quantity: u64,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct NftMintDetails {
    pub name: String,
    pub description: String,
}

#[derive(Serialize)]
pub struct OracleConfigFields {
    pub token_ids: TokenIds,
    pub node_ip: String,
    pub node_port: String,
    pub node_api_key: String,
}

#[derive(Debug, Error, From)]
pub enum BootstrapError {
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
    #[error("Node doesn't have a change address set")]
    RefreshContract(RefreshContractError),
    #[error("Update contract error: {0}")]
    UpdateContract(UpdateContractError),
    #[error("Bootstrap config file already exists")]
    ConfigFilenameAlreadyExists,
}

#[cfg(test)]
mod tests {
    use ergo_lib::{
        chain::{ergo_state_context::ErgoStateContext, transaction::TxId},
        ergotree_interpreter::sigma_protocol::private_input::DlogProverInput,
        ergotree_ir::chain::{
            address::AddressEncoder,
            ergo_box::{ErgoBox, NonMandatoryRegisters},
            token::TokenId,
        },
        wallet::Wallet,
    };
    use sigma_test_util::force_any_val;

    use super::*;
    use crate::pool_commands::test_utils::{LocalTxSigner, WalletDataMock};
    use std::cell::RefCell;
    #[derive(Default)]
    struct SubmitTxMock {
        transactions: RefCell<Vec<ergo_lib::chain::transaction::Transaction>>,
    }

    impl SubmitTransaction for SubmitTxMock {
        fn submit_transaction(
            &self,
            tx: &ergo_lib::chain::transaction::Transaction,
        ) -> crate::node_interface::Result<String> {
            self.transactions.borrow_mut().push(tx.clone());
            // Return empty string as TxId
            Ok("".into())
        }
    }

    #[test]
    fn test_bootstrap() {
        let ctx = force_any_val::<ErgoStateContext>();
        let height = ctx.pre_header.height;
        let secret = force_any_val::<DlogProverInput>();
        let address = Address::P2Pk(secret.public_image());
        let is_mainnet = address.content_bytes()[0] < NetworkPrefix::Testnet as u8;
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

        let state = BootstrapConfig {
            tokens_to_mint: TokensToMint {
                pool_nft: NftMintDetails {
                    name: "pool NFT".into(),
                    description: "Pool NFT".into(),
                },
                refresh_nft: NftMintDetails {
                    name: "refresh NFT".into(),
                    description: "refresh NFT".into(),
                },
                update_nft: NftMintDetails {
                    name: "update NFT".into(),
                    description: "update NFT".into(),
                },
                oracle_tokens: TokenMintDetails {
                    name: "oracle token".into(),
                    description: "oracle token".into(),
                    quantity: 15,
                },
                ballot_tokens: TokenMintDetails {
                    name: "ballot token".into(),
                    description: "ballot token".into(),
                    quantity: 15,
                },
                reward_tokens: TokenMintDetails {
                    name: "reward token".into(),
                    description: "reward token".into(),
                    quantity: 100_000_000,
                },
            },
            refresh_contract_parameters: RefreshContractParameters::default(),
            pool_contract_parameters: PoolContractParameters::default(),
            update_contract_parameters: UpdateContractParameters::default(),
            ballot_contract_parameters: BallotContractParameters::default(),
            addresses: Addresses {
                address_for_oracle_tokens: address.clone(),
                wallet_address_for_chain_transaction: address.clone(),
            },
            node_ip: "127.0.0.1".into(),
            node_port: "9053".into(),
            node_api_key: "hello".into(),
            on_mainnet: is_mainnet,
        };

        let height = ctx.pre_header.height;
        let submit_tx = SubmitTxMock::default();
        let oracle_config = perform_bootstrap_chained_transaction(BootstrapInput {
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
        })
        .unwrap();

        let token_ids = &oracle_config.token_ids;
        // Find output box guarding the Update NFT
        let txs = submit_tx.transactions.borrow();
        let update_nft_box = txs
            .iter()
            .flat_map(|tx| tx.outputs.iter())
            .find(|output| {
                output
                    .tokens
                    .clone()
                    .into_iter()
                    .flatten()
                    .any(|token| token.token_id == token_ids.update_nft_token_id)
            })
            .unwrap();
        // Check that Update NFT is guarded by UpdateContract, and parameters are correct

        let parameters = UpdateContractParameters::default();

        let update_contract_inputs = UpdateContractInputs {
            contract_parameters: &parameters,
            pool_nft_token_id: &token_ids.pool_nft_token_id,
            ballot_token_id: &token_ids.ballot_token_id,
        };
        let update_contract = crate::contracts::update::UpdateContract::from_ergo_tree(
            update_nft_box.ergo_tree.clone(),
            update_contract_inputs,
        )
        .unwrap();
        assert!(update_contract.min_votes() == state.update_contract_parameters.min_votes);
        assert!(update_contract.pool_nft_token_id() == token_ids.pool_nft_token_id);
        assert!(update_contract.ballot_token_id() == token_ids.ballot_token_id);
        let s = serde_yaml::to_string(&oracle_config).unwrap();
        println!("{}", s);

        // Quickly check an encoding
        let bytes: Vec<u8> = token_ids.ballot_token_id.clone().into();
        let encoded = base64::encode(bytes);
        let ballot_id = TokenId::from_base64(&encoded).unwrap();
        assert_eq!(token_ids.ballot_token_id, ballot_id);
    }
}
