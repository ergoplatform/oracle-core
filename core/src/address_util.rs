use ergo_lib::ergotree_ir::{
    chain::address::{Address, AddressEncoder, AddressEncoderError},
    mir::constant::{Constant, Literal},
    serialization::{SigmaParsingError, SigmaSerializable, SigmaSerializationError},
    sigma_protocol::sigma_boolean::ProveDlog,
};
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

/// Given a P2S Ergo address, extract the hex-encoded serialized ErgoTree (script)
pub fn address_to_tree(address: &str) -> Result<String, AddressUtilError> {
    let address_parsed = AddressEncoder::unchecked_parse_network_address_from_str(address)?;
    let script = address_parsed.address().script()?;
    Ok(base16::encode_lower(&script.sigma_serialize_bytes()?))
}

/// Given a P2S Ergo address, convert it to a hex-encoded Sigma byte array constant
pub fn address_to_bytes(address: &str) -> Result<String, AddressUtilError> {
    let address_parsed = AddressEncoder::unchecked_parse_network_address_from_str(address)?;
    let script = address_parsed.address().script()?;
    Ok(base16::encode_lower(
        &Constant::from(script.sigma_serialize_bytes()?).sigma_serialize_bytes()?,
    ))
}

/// Given an Ergo P2PK Address, convert it to a raw hex-encoded EC point
/// and prepend the type bytes so it is encoded and ready
/// to be used in a register.
pub fn address_to_raw_for_register(address: &str) -> Result<String, AddressUtilError> {
    let address_parsed = AddressEncoder::unchecked_parse_network_address_from_str(address)?;
    match address_parsed.address() {
        Address::P2Pk(ProveDlog { h }) => Ok(base16::encode_lower(
            &Constant::from(*h).sigma_serialize_bytes()?,
        )),
        Address::P2SH(_) | Address::P2S(_) => Err(AddressUtilError::ExpectedP2PK),
    }
}

/// Given an Ergo P2PK Address, convert it to a raw hex-encoded EC point
pub fn address_to_raw(address: &str) -> Result<String, AddressUtilError> {
    let address_parsed = AddressEncoder::unchecked_parse_network_address_from_str(address)?;
    match address_parsed.address() {
        Address::P2Pk(_) => Ok(base16::encode_lower(
            &address_parsed.address().content_bytes(),
        )),
        Address::P2SH(_) | Address::P2S(_) => Err(AddressUtilError::ExpectedP2PK),
    }
}

/// Given a raw hex-encoded EC point, convert it to a P2PK address
pub fn raw_to_address(raw: &str) -> Result<Address, AddressUtilError> {
    let bytes = base16::decode(raw)?;
    Address::p2pk_from_pk_bytes(&bytes).map_err(Into::into)
}

/// Given a raw hex-encoded EC point from a register (thus with type encoded characters in front),
/// convert it to a P2PK address
pub fn raw_from_register_to_address(raw: &str) -> Result<Address, AddressUtilError> {
    let bytes = base16::decode(raw)?;
    let constant = Constant::sigma_parse_bytes(&bytes)?;
    if let Literal::GroupElement(h) = constant.v {
        Ok(Address::P2Pk(ProveDlog { h }))
    } else {
        Err(AddressUtilError::ExpectedP2PK)
    }
}

#[cfg(test)]
mod test {
    use ergo_lib::ergotree_ir::chain::address::{AddressEncoder, NetworkPrefix};

    use crate::address_util::{
        address_to_bytes, address_to_raw, address_to_raw_for_register, address_to_tree,
        raw_from_register_to_address, raw_to_address,
    };

    // Test serialization for default address argument of /utils/addressToRaw
    #[test]
    fn test_address_to_raw_for_register() {
        assert_eq!(
            "07028333f9f7454f8d5ff73dbac9833767ed6fc3a86cf0a73df946b32ea9927d9197",
            address_to_raw_for_register("3WwbzW6u8hKWBcL1W7kNVMr25s2UHfSBnYtwSHvrRQt7DdPuoXrt")
                .unwrap()
        );
        assert_eq!(
            "028333f9f7454f8d5ff73dbac9833767ed6fc3a86cf0a73df946b32ea9927d9197",
            address_to_raw("3WwbzW6u8hKWBcL1W7kNVMr25s2UHfSBnYtwSHvrRQt7DdPuoXrt").unwrap()
        );
    }
    #[test]
    fn test_address_raw_roundtrip() {
        let address = AddressEncoder::new(NetworkPrefix::Testnet)
            .parse_address_from_str("3WwbzW6u8hKWBcL1W7kNVMr25s2UHfSBnYtwSHvrRQt7DdPuoXrt")
            .unwrap();
        assert_eq!(
            address,
            raw_to_address(
                &address_to_raw("3WwbzW6u8hKWBcL1W7kNVMr25s2UHfSBnYtwSHvrRQt7DdPuoXrt").unwrap()
            )
            .unwrap()
        );
    }
    #[test]
    fn test_address_raw_register_roundtrip() {
        let address = AddressEncoder::new(NetworkPrefix::Testnet)
            .parse_address_from_str("3WwbzW6u8hKWBcL1W7kNVMr25s2UHfSBnYtwSHvrRQt7DdPuoXrt")
            .unwrap();
        assert_eq!(
            address,
            raw_from_register_to_address(
                &address_to_raw_for_register(
                    "3WwbzW6u8hKWBcL1W7kNVMr25s2UHfSBnYtwSHvrRQt7DdPuoXrt"
                )
                .unwrap()
            )
            .unwrap()
        );
    }

    // test serialization of "sigmaProp(true)" script
    #[test]
    fn test_address_to_tree() {
        assert_eq!(
            "10010101d17300",
            address_to_tree("Ms7smJwLGbUAjuWQ").unwrap()
        );
        assert_eq!(
            "0e0710010101d17300",
            address_to_bytes("Ms7smJwLGbUAjuWQ").unwrap()
        );
    }
}
