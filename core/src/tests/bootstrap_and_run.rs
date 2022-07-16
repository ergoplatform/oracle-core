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
use crate::cli_commands::bootstrap::BootstrapRefreshContractParameters;
use crate::cli_commands::bootstrap::NftMintDetails;
use crate::cli_commands::bootstrap::OracleConfigFields;
use crate::cli_commands::bootstrap::TokenMintDetails;
use crate::cli_commands::bootstrap::TokensToMint;
use crate::node_interface;
use crate::node_interface::SubmitTransaction;
use crate::pool_commands::test_utils::init_log_tests;
use crate::pool_commands::test_utils::make_update_contract_parameters;
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
    let refresh_box_address = AddressEncoder::new(NetworkPrefix::Mainnet).parse_address_from_str("oq3jWGvabYxVYtceq1RGzFD4UdcdHcqY861G7H4mDiEnYQHya17A2w5r7u45moTpjAqfsNTm2XyhRNvYHiZhDTpmnfVa9XHSsbs5zjEw5UmgQfuP5d3NdFVy7oiAvLP1sjZN8qiHryzFoenLgtsxV8wLAeBaRChy73dd3rgyVfZipVL5LCXQyXMqp9oFFzPtTPkBw3ha7gJ4Bs5KjeUkVXJRVQ2Tdhg51Sdb6fEkHRtRuvCpynxYokQXP6SNif1M6mPcBR3B4zMLcFvmGxwNkZ3mRFzqHVzHV8Syu5AzueJEmMTrvWAXnhpYE7WcFbmDt3dqyXq7x9DNyKq1VwRwgFscLYDenAHqqHKd3jsJ6Grs8uFvvvJGKdqzdoJ3qCcCRXeDcZAKmExJMH4hJbsk8b1ct5YDBcNrq3LUr319XkS8miZDbHdHa88MSpCJQJmE51hmWVAV1yXrpyxqXqAXXPpSaGCP38BwCv8hYFK37DyA4mQd5r7vF9vNo5DEXwQ5wA2EivwRtNqpKUxXtKuZWTNC7Pu7NmvEHSuJPnaoCUujCiPtLM4dR64u8Gp7X3Ujo3o9zuMc6npemx3hf8rQS18QXgKJLwfeSqVYkicbVcGZRHsPsGxwrf1Wixp45E8d5e97MsKTCuqSskPKaHUdQYW1JZ8djcr4dxg1qQN81m7u2q8dwW6AK32mwRSS3nj27jkjML6n6GBpNZk9AtB2uMx3CHo6pZSaxgeCXuu3amrdeYmbuSqHUNZHU").unwrap();
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
        refresh_contract_parameters: BootstrapRefreshContractParameters {
            p2s: NetworkAddress::new(NetworkPrefix::Mainnet, &refresh_box_address),
            epoch_length: 30,
            buffer_length: 4,
            min_data_points: 4,
            max_deviation_percent: 5,
            min_votes: 6,
            pool_nft_index: 17,
            oracle_token_id_index: 3,
            min_data_points_index: 13,
            buffer_index: 21,
            max_deviation_percent_index: 15,
            epoch_length_index: 0,
        },
        pool_contract_parameters: BootstrapPoolContractParameters {
            p2s: NetworkAddress::new(NetworkPrefix::Mainnet, &pool_box_address),
            refresh_nft_index: 2,
            update_nft_index: 3,
        },
        update_contract_parameters: make_update_contract_parameters(),
        addresses: Addresses {
            address_for_oracle_tokens: address.clone(),
            wallet_address_for_chain_transaction: address.clone(),
        },
        node_ip: "127.0.0.1".into(),
        node_port: "9053".into(),
        node_api_key: "hello".into(),
        is_mainnet,
        total_oracles: 15,
        total_ballots: 15,
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
