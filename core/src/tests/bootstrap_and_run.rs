use std::cell::RefCell;
use std::convert::TryInto;

use ergo_chain_sim::Block;
use ergo_chain_sim::ChainSim;
use ergo_lib::chain::ergo_state_context::ErgoStateContext;
use ergo_lib::chain::transaction::Transaction;
use ergo_lib::ergotree_interpreter::sigma_protocol::private_input::DlogProverInput;
use ergo_lib::ergotree_ir::chain::address::Address;
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;
use ergo_lib::wallet::Wallet;
use sigma_test_util::force_any_val;

use crate::cli_commands::bootstrap::perform_bootstrap_chained_transaction;
use crate::cli_commands::bootstrap::Addresses;
use crate::cli_commands::bootstrap::BootstrapConfig;
use crate::cli_commands::bootstrap::BootstrapInput;
use crate::cli_commands::bootstrap::NftMintDetails;
use crate::cli_commands::bootstrap::TokenMintDetails;
use crate::cli_commands::bootstrap::TokensToMint;
use crate::contracts::ballot::BallotContractParameters;
use crate::contracts::oracle::OracleContractParameters;
use crate::contracts::pool::PoolContractParameters;
use crate::contracts::refresh::RefreshContractParameters;
use crate::contracts::update::UpdateContractParameters;
use crate::node_interface;
use crate::node_interface::SubmitTransaction;
use crate::oracle_config::OracleConfig;
use crate::oracle_config::BASE_FEE;
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

fn bootstrap(wallet: &Wallet, address: &Address, chain: &mut ChainSim) -> OracleConfig {
    let ctx = force_any_val::<ErgoStateContext>();

    let unspent_boxes = chain.get_unspent_boxes(&address.script().unwrap());
    let change_address = address;
    let network_address = NetworkAddress::new(NetworkPrefix::Mainnet, address);

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
        oracle_contract_parameters: OracleContractParameters::default(),
        refresh_contract_parameters: RefreshContractParameters::default(),
        pool_contract_parameters: PoolContractParameters::default(),
        update_contract_parameters: UpdateContractParameters::default(),
        ballot_contract_parameters: BallotContractParameters::default(),
        addresses: Addresses {
            address_for_oracle_tokens: network_address.clone(),
            wallet_address_for_chain_transaction: network_address.clone(),
            ballot_token_owner_address: network_address,
        },
        node_ip: "127.0.0.1".into(),
        node_port: 9053,
        node_api_key: "hello".into(),
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
        tx_fee: *BASE_FEE,
        erg_value_per_box: *BASE_FEE,
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
