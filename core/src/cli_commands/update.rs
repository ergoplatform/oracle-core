use std::{convert::TryInto, io::Write};

use derive_more::From;
use ergo_lib::{
    chain::{
        ergo_box::box_builder::{ErgoBoxCandidateBuilder, ErgoBoxCandidateBuilderError},
        transaction::Transaction,
    },
    ergotree_ir::{
        chain::{
            address::{Address, AddressEncoderError},
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
        ballot::BallotContractParameters,
        oracle::OracleContractInputs,
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
    oracle_state::OraclePool,
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
    pub refresh_contract_parameters: Option<RefreshContractParameters>,
    pub update_contract_parameters: Option<UpdateContractParameters>,
    pub tokens_to_mint: UpdateTokensToMint,
    pub addresses: Addresses,
}

fn update(config_file_name: String) -> Result<(), UpdateBootstrapError> {
    let s = std::fs::read_to_string(config_file_name)?;
    let config: UpdateBootstrapConfig = serde_yaml::from_str(&s)?;

    let node_interface = new_node_interface();
    let update_bootstrap_input = UpdateBootstrapInput {
        config: config.clone(),
        wallet: &node_interface,
        tx_signer: &node_interface,
        submit_tx: &node_interface,
        tx_fee: BoxValue::SAFE_USER_MIN,
        erg_value_per_box: BoxValue::SAFE_USER_MIN,
        change_address: config
            .addresses
            .wallet_address_for_chain_transaction
            .clone(),
        height: node_interface
            .current_block_height()
            .unwrap()
            .try_into()
            .unwrap(),
    };

    let new_config = perform_update_bootstrap_chained_transaction(update_bootstrap_input)?;

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
}

pub(crate) fn perform_update_bootstrap_chained_transaction(
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
        ..
    } = input;

    // count number of transactions, TODO: add update_contract_parameters, mint reward tokens, etc
    let mut num_transactions_left = 0;

    if config.refresh_contract_parameters.is_some() {
        num_transactions_left += 1;
    }
    if config.tokens_to_mint.oracle_tokens.is_some() {
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

    let mut new_oracle_config = ORACLE_CONFIG.clone();
    let mut transactions = vec![];
    let mut inputs = box_selection.boxes.clone(); // Inputs for each transaction in chained tx, updated after each mint step

    if let Some(ref nft_mint_details) = config.tokens_to_mint.oracle_tokens {
        let (token, tx) = mint_token(
            inputs.as_vec().clone(),
            &mut num_transactions_left,
            nft_mint_details.name.clone(),
            nft_mint_details.description.clone(),
            nft_mint_details.quantity.try_into().unwrap(),
            None,
        )?;
        new_oracle_config.token_ids.oracle_token_id = token.token_id;
        inputs = filter_tx_outputs(tx.outputs.clone()).try_into().unwrap();
    }
    if let Some(ref contract_parameters) = config.refresh_contract_parameters {
        let refresh_contract_inputs = RefreshContractInputs {
            contract_parameters,
            oracle_token_id: &new_oracle_config.token_ids.oracle_token_id,
            pool_nft_token_id: &ORACLE_CONFIG.token_ids.pool_nft_token_id,
        };
        let refresh_contract = RefreshContract::new(refresh_contract_inputs)?;
        let refresh_nft_details = config
            .tokens_to_mint
            .refresh_nft
            .ok_or(UpdateBootstrapError::NoMintDetails)?;
        let (token, tx) = mint_token(
            inputs.as_vec().clone(),
            &mut num_transactions_left,
            refresh_nft_details.name.clone(),
            refresh_nft_details.description.clone(),
            1.try_into().unwrap(),
            Some(refresh_contract.ergo_tree()),
        )?;
        new_oracle_config.token_ids.refresh_nft_token_id = token.token_id;
        new_oracle_config.refresh_contract_parameters = contract_parameters.clone();
        //TODO: inputs = filter_tx_outputs(tx.outputs.clone()).try_into().unwrap();
        transactions.push(tx);
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
    #[error("Node doesn't have a change address set")]
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
