use std::cell::RefCell;
use std::convert::TryInto;

use ergo_chain_sim::Block;
use ergo_chain_sim::ChainSim;
use ergo_lib::chain::ergo_state_context::ErgoStateContext;
use ergo_lib::chain::transaction::Transaction;
use ergo_lib::ergotree_interpreter::sigma_protocol::private_input::DlogProverInput;
use ergo_lib::ergotree_ir::chain::address::Address;
use ergo_lib::ergotree_ir::chain::address::AddressEncoder;
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::wallet::Wallet;
use sigma_test_util::force_any_val;

use crate::cli_commands::bootstrap::perform_bootstrap_chained_transaction;
use crate::cli_commands::bootstrap::Addresses;
use crate::cli_commands::bootstrap::BootstrapConfig;
use crate::cli_commands::bootstrap::BootstrapInput;
use crate::cli_commands::bootstrap::BootstrapPoolContractParameters;
use crate::cli_commands::bootstrap::NftMintDetails;
use crate::cli_commands::bootstrap::OracleConfigFields;
use crate::cli_commands::bootstrap::RefreshContractParameters;
use crate::cli_commands::bootstrap::TokenMintDetails;
use crate::cli_commands::bootstrap::TokensToMint;
use crate::node_interface;
use crate::node_interface::SubmitTransaction;
use crate::pool_commands::test_utils::init_log_tests;
use crate::pool_commands::test_utils::LocalTxSigner;
use crate::pool_commands::test_utils::WalletDataMock;

struct ChainSubmitTx<'a> {
    chain: RefCell<&'a mut ChainSim>,
}

impl<'a> SubmitTransaction for ChainSubmitTx<'a> {
    fn submit_transaction(&self, tx: &Transaction) -> node_interface::Result<String> {
        self.chain
            .borrow_mut()
            .add_block(Block::new(vec![tx.clone()]));
        Ok(tx.id().into())
    }
}

fn bootstrap(wallet: &Wallet, address: &Address, chain: &mut ChainSim) -> OracleConfigFields {
    let ctx = force_any_val::<ErgoStateContext>();
    let is_mainnet = true;

    let unspent_boxes = chain.get_unspent_boxes(&address.script().unwrap());
    let change_address = address;

    let pool_box_address = AddressEncoder::new(NetworkPrefix::Mainnet).parse_address_from_str("PViBL5acX6PoP6BQPsYtyNzW9aPXwxpRaUkXo4nE7RkxcBbZXJECUEBQm4g3MQCb2QsQALqPkrDN9TvsKuQkChF8sZSfnH5fifgKAkXhW8ifAcAE1qA67n9mabB3Mb2R8xT2v3SN49eN8mQ8HN95").unwrap();
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
        pool_contract_parameters: BootstrapPoolContractParameters {
            p2s: NetworkAddress::new(NetworkPrefix::Mainnet, &pool_box_address),
            refresh_nft_index: 2,
            update_nft_index: 3,
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
    let mut submit_tx_mock = ChainSubmitTx {
        chain: chain.into(),
    };
    perform_bootstrap_chained_transaction(BootstrapInput {
        config: state,
        wallet: &WalletDataMock {
            unspent_boxes: unspent_boxes.clone(),
        },
        tx_signer: &mut LocalTxSigner { ctx: &ctx, wallet },
        submit_tx: &mut submit_tx_mock,
        tx_fee: BoxValue::SAFE_USER_MIN,
        erg_value_per_box: BoxValue::SAFE_USER_MIN,
        change_address: change_address.clone(),
        height,
    })
    .unwrap()
}

#[test]
fn test_bootstrap_and_run() {
    init_log_tests();
    let mut chain = ChainSim::new();
    let secret = force_any_val::<DlogProverInput>();
    let wallet = Wallet::from_secrets(vec![secret.clone().into()]);
    let address = Address::P2Pk(secret.public_image());
    chain.generate_unspent_box(
        address.script().unwrap(),
        100_000_000_u64.try_into().unwrap(),
        None,
    );
    let _oracle_config = bootstrap(&wallet, &address, &mut chain);
    assert_eq!(chain.height, 8);
}
