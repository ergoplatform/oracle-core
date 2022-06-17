use ergo_lib::chain::ergo_state_context::ErgoStateContext;
use ergo_lib::chain::transaction::TxId;
use ergo_lib::ergotree_interpreter::sigma_protocol::private_input::DlogProverInput;
use ergo_lib::ergotree_ir::chain::address::Address;
use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisters;
use ergo_lib::wallet::Wallet;
use sigma_test_util::force_any_val;

use crate::cli_commands::bootstrap::perform_bootstrap_chained_transaction;
use crate::cli_commands::bootstrap::Addresses;
use crate::cli_commands::bootstrap::BootstrapConfig;
use crate::cli_commands::bootstrap::BootstrapInput;
use crate::cli_commands::bootstrap::NftMintDetails;
use crate::cli_commands::bootstrap::OracleConfigFields;
use crate::cli_commands::bootstrap::RefreshContractParameters;
use crate::cli_commands::bootstrap::TokenMintDetails;
use crate::cli_commands::bootstrap::TokensToMint;
use crate::node_interface::SubmitTransaction;
use crate::pool_commands::test_utils::LocalTxSigner;
use crate::pool_commands::test_utils::WalletDataMock;

struct SubmitTxMock {}

impl SubmitTransaction for SubmitTxMock {
    fn submit_transaction(
        &self,
        _: &ergo_lib::chain::transaction::Transaction,
    ) -> crate::node_interface::Result<String> {
        // TODO: submit to the ChainSim
        Ok("".to_string())
    }
}

fn bootstrap(wallet: Wallet, address: Address) -> OracleConfigFields {
    let ctx = force_any_val::<ErgoStateContext>();
    let height = ctx.pre_header.height;
    let is_mainnet = true;
    let ergo_tree = address.script().unwrap();

    let value = BoxValue::SAFE_USER_MIN.checked_mul_u32(10000).unwrap();
    // TODO: get from the ChainSim
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
        refresh_contract_parameters: RefreshContractParameters {
            epoch_length: 30,
            buffer: 4,
            total_oracles: 15,
            min_data_points: 4,
            max_deviation_percent: 5,
            total_ballots: 15,
            min_votes: 6,
        },
        addresses: Addresses {
            address_for_oracle_tokens: address.clone(),
            wallet_address_for_chain_transaction: address.clone(),
        },
        node_ip: "127.0.0.1".into(),
        node_port: "9053".into(),
        node_api_key: "hello".into(),
        is_mainnet,
    };

    let height = ctx.pre_header.height;
    perform_bootstrap_chained_transaction(BootstrapInput {
        config: state,
        wallet: &WalletDataMock {
            unspent_boxes: unspent_boxes.clone(),
        },
        tx_signer: &mut LocalTxSigner { ctx, wallet },
        submit_tx: &SubmitTxMock {},
        tx_fee: BoxValue::SAFE_USER_MIN,
        erg_value_per_box: BoxValue::SAFE_USER_MIN,
        change_address,
        height,
    })
    .unwrap()
}

#[test]
fn test_bootstrap_and_run() {
    let secret = force_any_val::<DlogProverInput>();
    let wallet = Wallet::from_secrets(vec![secret.clone().into()]);
    let address = Address::P2Pk(secret.public_image());
    let _oracle_config = bootstrap(wallet, address);
}
