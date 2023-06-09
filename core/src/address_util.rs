use ergo_lib::ergo_chain_types::EcPoint;
use ergo_lib::ergotree_ir::chain::address::Address;
use ergo_lib::ergotree_ir::chain::address::AddressEncoderError;
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::address::NetworkPrefix;
use ergo_lib::ergotree_ir::serialization::SigmaParsingError;
use ergo_lib::ergotree_ir::serialization::SigmaSerializationError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AddressUtilError {
    #[error("address encoder error: {0}")]
    AddressEncoderError(#[from] AddressEncoderError),
    #[error("expected P2PK address")]
    ExpectedP2PK,
    #[error("expected P2S address")]
    ExpectedP2S,
    #[error("serialization error: {0}")]
    SigmaSerializationError(#[from] SigmaSerializationError),
    #[error("sigma parse error: {0}")]
    SigmaParsingError(#[from] SigmaParsingError),
    #[error("base16 error: {0}")]
    Base16DecodeError(#[from] base16::DecodeError),
}

pub fn pks_to_network_addresses(
    pks: Vec<EcPoint>,
    network_prefix: NetworkPrefix,
) -> Vec<NetworkAddress> {
    pks.into_iter()
        .map(|pk| NetworkAddress::new(network_prefix, &Address::P2Pk(pk.into())))
        .collect()
}
