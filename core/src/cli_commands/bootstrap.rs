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
            address::{Address, AddressEncoderError, NetworkAddress},
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
    box_kind::{make_pool_box_candidate, make_refresh_box_candidate},
    contracts::{
        ballot::{BallotContractError, BallotContractParameters},
        oracle::OracleContractParameters,
        pool::{PoolContract, PoolContractError, PoolContractInputs, PoolContractParameters},
        refresh::{
            RefreshContract, RefreshContractError, RefreshContractInputs, RefreshContractParameters,
        },
        update::{
            UpdateContract, UpdateContractError, UpdateContractInputs, UpdateContractParameters,
        },
    },
    datapoint_source::PredefinedDataPointSource,
    node_interface::{
        assert_wallet_unlocked,
        node_api::{NodeApi, NodeApiError},
        SignTransactionWithInputs, SubmitTransaction,
    },
    oracle_config::{BASE_FEE, ORACLE_CONFIG},
    pool_config::{PoolConfig, PoolConfigError, TokenIds},
    serde::BootstrapConfigSerde,
    spec_token::{
        BallotTokenId, OracleTokenId, PoolTokenId, RefreshTokenId, RewardTokenId, SpecToken,
        TokenIdKind, UpdateTokenId,
    },
    wallet::{WalletDataError, WalletDataSource},
};

/// Loads bootstrap configuration file and performs the chain-transactions for minting of tokens and
/// box creations. An oracle configuration file is then created which contains the `TokenId`s of the
/// minted tokens.
pub fn bootstrap(config_file_name: String) -> Result<(), BootstrapError> {
    let oracle_config = &ORACLE_CONFIG;
    let s = std::fs::read_to_string(config_file_name)?;
    let config: BootstrapConfig = serde_yaml::from_str(&s)?;

    let node_api = NodeApi::new(oracle_config.node_api_key.clone(), &oracle_config.node_url);
    assert_wallet_unlocked(&node_api.node);
    let change_address = node_api.get_change_address()?;
    debug!("Change address: {:?}", change_address);
    let erg_value_per_box = config.oracle_contract_parameters.min_storage_rent;
    let input = BootstrapInput {
        oracle_address: oracle_config.oracle_address.clone(),
        config,
        wallet: &node_api as &dyn WalletDataSource,
        tx_signer: &node_api.node as &dyn SignTransactionWithInputs,
        submit_tx: &node_api.node as &dyn SubmitTransaction,
        tx_fee: *BASE_FEE,
        erg_value_per_box,
        change_address: change_address.address(),
        height: node_api.node.current_block_height()? as u32,
    };
    let oracle_config = perform_bootstrap_chained_transaction(input)?;
    info!("Bootstrap chain-transaction complete");
    let s = serde_yaml::to_string(&oracle_config)?;
    let mut file = std::fs::File::create(crate::oracle_config::DEFAULT_ORACLE_CONFIG_FILE_NAME)?;
    file.write_all(s.as_bytes())?;
    info!(
        "Oracle configuration file created: {}",
        crate::oracle_config::DEFAULT_ORACLE_CONFIG_FILE_NAME
    );
    Ok(())
}

pub fn generate_bootstrap_config_template(config_file_name: String) -> Result<(), BootstrapError> {
    if Path::new(&config_file_name).exists() {
        return Err(BootstrapError::ConfigFilenameAlreadyExists);
    }

    let config = BootstrapConfig::default();
    let config_serde = BootstrapConfigSerde::from(config);

    let s = serde_yaml::to_string(&config_serde)?;
    let mut file = std::fs::File::create(&config_file_name)?;
    file.write_all(s.as_bytes())?;
    Ok(())
}

pub struct BootstrapInput<'a> {
    pub oracle_address: NetworkAddress,
    pub config: BootstrapConfig,
    pub wallet: &'a dyn WalletDataSource,
    pub tx_signer: &'a dyn SignTransactionWithInputs,
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
) -> Result<PoolConfig, BootstrapError> {
    let BootstrapInput {
        oracle_address,
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

    let wallet_pk_ergo_tree = oracle_address.address().script()?;
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
        );
        let mint_token_tx = tx_builder.build()?;
        debug!("Mint token unsigned transaction: {:?}", mint_token_tx);
        let signed_tx = wallet_sign.sign_transaction_with_inputs(&mint_token_tx, inputs, None)?;
        *num_transactions_left -= 1;
        Ok((token, signed_tx))
    };

    // Mint pool NFT token --------------------------------------------------------------------------
    info!("Creating and signing minting pool NFT tx");
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
    info!("Creating and signing minting refresh NFT tx");
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
    info!("Creating and signing minting ballot tokens tx");
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

    let update_contract = UpdateContract::checked_load(&UpdateContractInputs::build_with(
        config.update_contract_parameters.clone(),
        PoolTokenId::from_token_id_unchecked(pool_nft_token.token_id),
        BallotTokenId::from_token_id_unchecked(ballot_token.token_id),
    )?)?;

    info!("Creating and signing minting update NFT tx");
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
    info!("Creating and signing minting oracle tokens tx");
    let inputs = filter_tx_outputs(signed_mint_update_nft_tx.outputs.clone());
    debug!("inputs for oracle tokens mint: {:?}", inputs);
    let oracle_tokens_pk_ergo_tree = oracle_address.address().script()?;
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
    info!("Creating and signing minting reward tokens tx");
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
    info!("Create and sign pool box tx");

    // we don't have a working ORACLE_CONFIG during bootstrap so token ids are created without any checks
    let token_ids = TokenIds {
        pool_nft_token_id: PoolTokenId::from_token_id_unchecked(pool_nft_token.token_id),
        refresh_nft_token_id: RefreshTokenId::from_token_id_unchecked(refresh_nft_token.token_id),
        update_nft_token_id: UpdateTokenId::from_token_id_unchecked(update_nft_token.token_id),
        oracle_token_id: OracleTokenId::from_token_id_unchecked(oracle_token.token_id),
        reward_token_id: RewardTokenId::from_token_id_unchecked(reward_token.token_id),
        ballot_token_id: BallotTokenId::from_token_id_unchecked(ballot_token.token_id),
    };

    let pool_contract = PoolContract::build_with(&PoolContractInputs::build_with(
        config.pool_contract_parameters.clone(),
        token_ids.refresh_nft_token_id.clone(),
        token_ids.update_nft_token_id.clone(),
    )?)
    .unwrap();

    let reward_tokens_for_pool_box = Token {
        token_id: reward_token.token_id,
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
        SpecToken {
            token_id: token_ids.pool_nft_token_id.clone(),
            amount: pool_nft_token.amount,
        },
        SpecToken {
            token_id: token_ids.reward_token_id.clone(),
            amount: reward_tokens_for_pool_box.amount,
        },
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
        &[pool_nft_token.clone(), reward_tokens_for_pool_box.clone()],
    )?;
    let inputs = box_selection.boxes.clone();
    let tx_builder = TxBuilder::new(
        box_selection,
        output_candidates,
        height,
        tx_fee,
        change_address.clone(),
    );
    let pool_box_tx = tx_builder.build()?;
    debug!("unsigned pool_box_tx: {:?}", pool_box_tx);
    let signed_pool_box_tx =
        wallet_sign.sign_transaction_with_inputs(&pool_box_tx, inputs, None)?;
    num_transactions_left -= 1;

    // Create refresh box --------------------------------------------------------------------------
    info!("Create and sign refresh box tx");

    let refresh_contract_inputs = RefreshContractInputs::build_with(
        config.refresh_contract_parameters.clone(),
        token_ids.oracle_token_id.clone(),
        token_ids.pool_nft_token_id.clone(),
    )?;
    let refresh_contract = RefreshContract::checked_load(&refresh_contract_inputs)?;

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
    );
    let refresh_box_tx = tx_builder.build()?;
    debug!("unsigned refresh_box_tx: {:?}", refresh_box_tx);
    let signed_refresh_box_tx =
        wallet_sign.sign_transaction_with_inputs(&refresh_box_tx, inputs, None)?;

    // ---------------------------------------------------------------------------------------------
    let tx_id = submit_tx.submit_transaction(&signed_mint_pool_nft_tx)?;
    info!("Minted pool NFT TxId: {}", tx_id);
    let tx_id = submit_tx.submit_transaction(&signed_mint_refresh_nft_tx)?;
    info!("Minted refresh NFT TxId: {}", tx_id);
    let tx_id = submit_tx.submit_transaction(&signed_mint_ballot_tokens_tx)?;
    info!("Minted ballot tokens TxId: {}", tx_id);
    let tx_id = submit_tx.submit_transaction(&signed_mint_update_nft_tx)?;
    info!("Minted update NFT TxId: {}", tx_id);
    let tx_id = submit_tx.submit_transaction(&signed_mint_oracle_tokens_tx)?;
    info!("Minted oracle tokens TxId: {}", tx_id);
    let tx_id = submit_tx.submit_transaction(&signed_mint_reward_tokens_tx)?;
    info!("Minted reward tokens TxId: {}", tx_id);
    let tx_id = submit_tx.submit_transaction(&signed_pool_box_tx)?;
    info!("Created initial pool box TxId: {}", tx_id);
    let tx_id = submit_tx.submit_transaction(&signed_refresh_box_tx)?;
    info!("Created initial refresh box TxId: {}", tx_id);

    info!("Minted tokens: {:?}", token_ids);

    Ok(PoolConfig::create(config, token_ids)?)
}

/// An instance of this struct is created from an operator-provided YAML file.
#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "crate::serde::BootstrapConfigSerde")]
pub struct BootstrapConfig {
    pub data_point_source: Option<PredefinedDataPointSource>,
    pub oracle_contract_parameters: OracleContractParameters,
    pub refresh_contract_parameters: RefreshContractParameters,
    pub pool_contract_parameters: PoolContractParameters,
    pub update_contract_parameters: UpdateContractParameters,
    pub ballot_contract_parameters: BallotContractParameters,
    pub tokens_to_mint: TokensToMint,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        BootstrapConfig {
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
            oracle_contract_parameters: OracleContractParameters::default(),
            data_point_source: Some(PredefinedDataPointSource::NanoErgUsd),
        }
    }
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

#[derive(Debug, Error, From)]
pub enum BootstrapError {
    #[error("tx builder error: {0}")]
    TxBuilder(TxBuilderError),
    #[error("box builder error: {0}")]
    ErgoBoxCandidateBuilder(ErgoBoxCandidateBuilderError),
    #[error("node error: {0}")]
    Node(NodeError),
    #[error("node api error: {0}")]
    NodeApiError(NodeApiError),
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
    #[error("Ballot contract error: {0}")]
    BallotContractError(BallotContractError),
    #[error("Pool config error: {0}")]
    PoolConfigError(PoolConfigError),
    #[error("Pool contract error: {0}")]
    PoolContractError(PoolContractError),
    #[error("WalletData error: {0}")]
    WalletData(WalletDataError),
}

#[cfg(test)]
pub(crate) mod tests {
    use ergo_lib::{
        chain::{ergo_state_context::ErgoStateContext, transaction::TxId},
        ergotree_interpreter::sigma_protocol::private_input::DlogProverInput,
        ergotree_ir::chain::{
            address::{AddressEncoder, NetworkAddress, NetworkPrefix},
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
    pub(crate) struct SubmitTxMock {
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
        let address = NetworkAddress::new(
            NetworkPrefix::Mainnet,
            &Address::P2Pk(secret.public_image()),
        );
        let wallet = Wallet::from_secrets(vec![secret.clone().into()]);
        let ergo_tree = address.address().script().unwrap();

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
        let change_address = AddressEncoder::unchecked_parse_network_address_from_str(
            "9iHyKxXs2ZNLMp9N9gbUT9V8gTbsV7HED1C1VhttMfBUMPDyF7r",
        )
        .unwrap();

        let bootstrap_config = BootstrapConfig::default();

        let height = ctx.pre_header.height;
        let submit_tx = SubmitTxMock::default();
        let oracle_config = perform_bootstrap_chained_transaction(BootstrapInput {
            oracle_address: address,
            config: bootstrap_config.clone(),
            wallet: &WalletDataMock {
                unspent_boxes: unspent_boxes.clone(),
                change_address: change_address.clone(),
            },
            tx_signer: &mut LocalTxSigner {
                ctx: &ctx,
                wallet: &wallet,
            },
            submit_tx: &submit_tx,
            tx_fee: *BASE_FEE,
            erg_value_per_box: *BASE_FEE,
            change_address: change_address.address(),
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
                    .any(|token| token.token_id == token_ids.update_nft_token_id.token_id())
            })
            .unwrap();
        // Check that Update NFT is guarded by UpdateContract, and parameters are correct

        let update_contract_inputs = UpdateContractInputs::build_with(
            UpdateContractParameters::default(),
            token_ids.pool_nft_token_id.clone(),
            token_ids.ballot_token_id.clone(),
        )
        .unwrap();
        let update_contract = crate::contracts::update::UpdateContract::from_ergo_tree(
            update_nft_box.ergo_tree.clone(),
            &update_contract_inputs,
        )
        .unwrap();
        assert!(
            update_contract.min_votes() == bootstrap_config.update_contract_parameters.min_votes()
        );
        assert!(update_contract.pool_nft_token_id() == token_ids.pool_nft_token_id.token_id());
        assert!(update_contract.ballot_token_id() == token_ids.ballot_token_id.token_id());
        let s = serde_yaml::to_string(&oracle_config).unwrap();
        println!("{}", s);

        // Quickly check an encoding
        let bytes: Vec<u8> = token_ids.ballot_token_id.token_id().into();
        let encoded = base64::encode(bytes);
        let ballot_id = TokenId::from_base64(&encoded).unwrap();
        assert_eq!(token_ids.ballot_token_id.token_id(), ballot_id);

        // Check that refresh contract is updated
        assert_ne!(
            oracle_config
                .refresh_box_wrapper_inputs
                .contract_inputs
                .contract_parameters()
                .ergo_tree_bytes(),
            bootstrap_config
                .refresh_contract_parameters
                .ergo_tree_bytes()
        );
        // Check that ballot contract is updated
        assert_ne!(
            oracle_config
                .ballot_box_wrapper_inputs
                .contract_inputs
                .contract_parameters()
                .ergo_tree_bytes(),
            bootstrap_config
                .ballot_contract_parameters
                .ergo_tree_bytes()
        );
        // Check that oracle contract is updated
        assert_ne!(
            oracle_config
                .oracle_box_wrapper_inputs
                .contract_inputs
                .contract_parameters()
                .ergo_tree_bytes(),
            bootstrap_config
                .oracle_contract_parameters
                .ergo_tree_bytes()
        );
        // Check that pool contract is updated
        assert_ne!(
            oracle_config
                .pool_box_wrapper_inputs
                .contract_inputs
                .contract_parameters()
                .ergo_tree_bytes(),
            bootstrap_config.pool_contract_parameters.ergo_tree_bytes()
        );
        // Check that update contract is updated
        assert_ne!(
            oracle_config
                .update_box_wrapper_inputs
                .contract_inputs
                .contract_parameters()
                .ergo_tree_bytes(),
            bootstrap_config
                .update_contract_parameters
                .ergo_tree_bytes()
        );
    }

    #[test]
    fn test_custom_contract_param() {
        let config: BootstrapConfig = serde_yaml::from_str("
---
oracle_contract_parameters:
  ergo_tree_bytes: 100a040004000580dac409040004000e20472b4b6250655368566d597133743677397a24432646294a404d635166546a570402040204020402d804d601b2a5e4e3000400d602db63087201d603db6308a7d604e4c6a70407ea02d1ededed93b27202730000b2720373010093c27201c2a7e6c67201040792c172017302eb02cd7204d1ededededed938cb2db6308b2a4730300730400017305938cb27202730600018cb2720373070001918cb27202730800028cb272037309000293e4c672010407720492c17201c1a7efe6c672010561
  pool_nft_index: 5
  min_storage_rent_index: 2
  min_storage_rent: 10000000
refresh_contract_parameters:
  ergo_tree_bytes: 1016043c040004000e202a472d4a614e645267556b58703273357638792f423f4528482b4d625065536801000502010105000400040004020402040204080400040a05c8010e20472b4b6250655368566d597133743677397a24432646294a404d635166546a570400040404020408d80ed60199a37300d602b2a4730100d603b5a4d901036395e6c672030605eded928cc77203017201938cb2db6308720373020001730393e4c672030504e4c6720205047304d604b17203d605b0720386027305860273067307d901053c413d0563d803d607e4c68c7205020605d6088c720501d6098c720802860272078602ed8c720901908c72080172079a8c7209027207d6068c720502d6078c720501d608db63087202d609b27208730800d60ab2a5730900d60bdb6308720ad60cb2720b730a00d60db27208730b00d60eb2a5730c00ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02ea02cde4c6b27203e4e30004000407d18f8cc77202017201d1927204730dd18c720601d190997207e4c6b27203730e0006059d9c72077e730f057310d1938c7209017311d193b2720b7312007209d1938c720c018c720d01d1928c720c02998c720d027e9c7204731305d193b1720bb17208d193e4c6720a04059d8c7206027e720405d193e4c6720a05049ae4c6720205047314d193c2720ac27202d192c1720ac17202d1928cc7720a0199a37315d193db6308720edb6308a7d193c2720ec2a7d192c1720ec1a7
  pool_nft_index: 17
  oracle_token_id_index: 3
  min_data_points_index: 13
  min_data_points: 2
  buffer_length_index: 21
  buffer_length: 4
  max_deviation_percent_index: 15
  max_deviation_percent: 5
  epoch_length_index: 0
  epoch_length: 30
pool_contract_parameters:
  ergo_tree_bytes: 1004040204000e20546a576e5a7234753778214125442a472d4b614e645267556b587032733576380e206251655468576d5a7134743777217a25432a462d4a404e635266556a586e3272d801d6018cb2db6308b2a473000073010001d1ec93720173029372017303
  refresh_nft_index: 2
  update_nft_index: 3
update_contract_parameters:
  ergo_tree_bytes: 100e040004000400040204020e20472b4b6250655368566d597133743677397a24432646294a404d635166546a570400040004000e203f4428472d4b6150645367566b5970337336763979244226452948404d625165010005000400040cd806d601b2a4730000d602b2db63087201730100d603b2a5730200d604db63087203d605b2a5730300d606b27204730400d1ededed938c7202017305ededededed937202b27204730600938cc77201018cc772030193c17201c1720393c672010405c67203040593c672010504c672030504efe6c672030661edededed93db63087205db6308a793c27205c2a792c17205c1a7918cc77205018cc7a701efe6c67205046192b0b5a4d9010763d801d609db630872079591b172097307edededed938cb2720973080001730993e4c6720705048cc7a70193e4c67207060ecbc2720393e4c67207070e8c72060193e4c6720708058c720602730a730bd9010741639a8c7207018cb2db63088c720702730c00027e730d05
  pool_nft_index: 5
  ballot_token_index: 9
  min_votes_index: 13
  min_votes: 2
ballot_contract_parameters:
  ergo_tree_bytes: 10070580dac409040204020400040204000e206251655468576d5a7134743777217a25432a462d4a404e635266556a586e3272d803d601b2a5e4e3000400d602c672010407d603e4c6a70407ea02d1ededede6720293c27201c2a793db63087201db6308a792c172017300eb02cd7203d1ededededed91b1a4730191b1db6308b2a47302007303938cb2db6308b2a473040073050001730693e47202720392c17201c1a7efe6c672010561
  min_storage_rent_index: 0
  min_storage_rent: 10000000
  update_nft_index: 6
tokens_to_mint:
  pool_nft:
    name: pool NFT
    description: Pool NFT
  refresh_nft:
    name: refresh NFT
    description: refresh NFT
  update_nft:
    name: update NFT
    description: update NFT
  oracle_tokens:
    name: oracle token
    description: oracle token
    quantity: 15
  ballot_tokens:
    name: ballot token
    description: ballot token
    quantity: 15
  reward_tokens:
    name: reward token
    description: reward token
    quantity: 100000000
node_ip: 10.94.77.47
node_port: 9052
node_api_key: hello
core_api_port: 9010
data_point_source: NanoErgUsd
data_point_source_custom_script: ~
oracle_address: 3Wy3BaCjGDWE3bjjZkNo3aWaMz3cYrePMFhchcKovY9uG9vhpAuW
base_fee: 1100000
").unwrap();
        assert_eq!(config.refresh_contract_parameters.min_data_points(), 2);
    }
}
