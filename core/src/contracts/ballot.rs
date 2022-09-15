use derive_more::From;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTreeConstantError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractFromError;
use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;
use ergo_lib::ergotree_ir::serialization::SigmaParsingError;
use ergo_lib::ergotree_ir::serialization::SigmaSerializable;
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct BallotContract {
    ergo_tree: ErgoTree,
    min_storage_rent_index: usize,
    update_nft_index: usize,
}

#[derive(Debug, Error, From)]
pub enum BallotContractError {
    #[error("ballot contract: parameter error: {0}")]
    ParametersError(BallotContractParametersError),
    #[error("ballot contract: unknown update NFT defined in constant")]
    UnknownUpdateNftId,
    #[error("ballot contract: sigma parsing error {0}")]
    SigmaParsing(SigmaParsingError),
    #[error("ballot contract: ergo tree constant error {0:?}")]
    ErgoTreeConstant(ErgoTreeConstantError),
    #[error("ballot contract: TryExtractFrom error {0:?}")]
    TryExtractFrom(TryExtractFromError),
    #[error("contract error: {1:?}, expected P2S: {0}")]
    WrappedWithExpectedP2SAddress(String, Box<Self>),
}

#[derive(Clone, Debug)]
pub struct BallotContractInputs {
    contract_parameters: BallotContractParameters,
    pub update_nft_token_id: TokenId,
}

impl BallotContractInputs {
    pub fn build_with(
        contract_parameters: BallotContractParameters,
        update_nft_token_id: TokenId,
    ) -> Result<Self, BallotContractError> {
        let ballot_contract = BallotContract::build_with(&BallotContractInputs {
            contract_parameters,
            update_nft_token_id: update_nft_token_id.clone(),
        })?;
        let new_parameters = ballot_contract.parameters();
        Ok(Self {
            contract_parameters: new_parameters,
            update_nft_token_id,
        })
    }

    pub fn checked_load(
        contract_parameters: BallotContractParameters,
        update_nft_token_id: TokenId,
    ) -> Result<Self, BallotContractError> {
        let contract_inputs = Self {
            contract_parameters,
            update_nft_token_id,
        };
        let _ = BallotContract::checked_load(&contract_inputs)?;
        Ok(contract_inputs)
    }

    pub fn contract_parameters(&self) -> &BallotContractParameters {
        &self.contract_parameters
    }
}

impl BallotContract {
    pub fn checked_load(inputs: &BallotContractInputs) -> Result<Self, BallotContractError> {
        let ergo_tree =
            ErgoTree::sigma_parse_bytes(inputs.contract_parameters.ergo_tree_bytes.as_slice())?;
        let contract = Self::from_ergo_tree(ergo_tree, inputs).map_err(|e| {
            let expected_base16 = Self::build_with(inputs)
                .unwrap()
                .ergo_tree
                .to_base16_bytes()
                .unwrap();
            BallotContractError::WrappedWithExpectedP2SAddress(expected_base16, e.into())
        })?;
        Ok(contract)
    }

    fn build_with(inputs: &BallotContractInputs) -> Result<Self, BallotContractError> {
        let parameters = inputs.contract_parameters.clone();
        let ergo_tree =
            ErgoTree::sigma_parse_bytes(inputs.contract_parameters.ergo_tree_bytes.as_slice())?
                .with_constant(
                    parameters.min_storage_rent_index,
                    (parameters.min_storage_rent as i64).into(),
                )?
                .with_constant(
                    parameters.update_nft_index,
                    inputs.update_nft_token_id.clone().into(),
                )?;
        let contract = Self::from_ergo_tree(ergo_tree, inputs)?;
        Ok(contract)
    }

    pub fn from_ergo_tree(
        ergo_tree: ErgoTree,
        inputs: &BallotContractInputs,
    ) -> Result<Self, BallotContractError> {
        dbg!(ergo_tree.get_constants().unwrap());
        let parameters = inputs.contract_parameters.clone();
        let min_storage_rent = ergo_tree
            .get_constant(parameters.min_storage_rent_index)
            .map_err(|_| {
                BallotContractError::ParametersError(
                    BallotContractParametersError::NoMinStorageRent,
                )
            })?
            .ok_or(BallotContractError::ParametersError(
                BallotContractParametersError::NoMinStorageRent,
            ))?
            .try_extract_into::<i64>()? as u64;
        if min_storage_rent != parameters.min_storage_rent {
            return Err(BallotContractError::ParametersError(
                BallotContractParametersError::MinStorageRentDiffers {
                    expected: parameters.min_storage_rent,
                    actual: min_storage_rent,
                },
            ));
        }

        let token_id = ergo_tree
            .get_constant(parameters.update_nft_index)
            .map_err(|_| {
                BallotContractError::ParametersError(BallotContractParametersError::NoUpdateNftId)
            })?
            .ok_or(BallotContractError::ParametersError(
                BallotContractParametersError::NoUpdateNftId,
            ))?
            .try_extract_into::<TokenId>()?;
        if token_id != inputs.update_nft_token_id {
            return Err(BallotContractError::UnknownUpdateNftId);
        }
        Ok(Self {
            ergo_tree,
            min_storage_rent_index: parameters.min_storage_rent_index,
            update_nft_index: parameters.update_nft_index,
        })
    }

    pub fn min_storage_rent(&self) -> u64 {
        self.ergo_tree
            .get_constant(self.min_storage_rent_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<i64>()
            .unwrap() as u64
    }

    pub fn update_nft_token_id(&self) -> TokenId {
        self.ergo_tree
            .get_constant(self.update_nft_index)
            .unwrap()
            .unwrap()
            .try_extract_into::<TokenId>()
            .unwrap()
    }

    pub fn ergo_tree(&self) -> ErgoTree {
        self.ergo_tree.clone()
    }

    pub fn parameters(&self) -> BallotContractParameters {
        BallotContractParameters {
            ergo_tree_bytes: self.ergo_tree.sigma_serialize_bytes().unwrap(),
            min_storage_rent_index: self.min_storage_rent_index,
            min_storage_rent: self.min_storage_rent(),
            update_nft_index: self.update_nft_index,
        }
    }
}

#[derive(Debug, Clone)]
/// Parameters for the ballot contract
pub struct BallotContractParameters {
    ergo_tree_bytes: Vec<u8>,
    min_storage_rent_index: usize,
    min_storage_rent: u64,
    update_nft_index: usize,
}

#[derive(Debug, Error, From)]
pub enum BallotContractParametersError {
    #[error("ballot contract parameters: failed to get update NFT from constants")]
    NoUpdateNftId,
    #[error("ballot contract parameters: failed to get minStorageRent from constants")]
    NoMinStorageRent,
    #[error(
        "ballot contract parameters: unexpected `min storage rent` value. Expected {expected:?}, got {actual:?}"
    )]
    MinStorageRentDiffers { expected: u64, actual: u64 },
    #[error("ballot contract parameters: sigma parsing error {0}")]
    SigmaParsing(SigmaParsingError),
    #[error("ballot contract parameters: TryExtractFrom error {0:?}")]
    TryExtractFrom(TryExtractFromError),
}

impl BallotContractParameters {
    pub fn build_with(
        ergo_tree_bytes: Vec<u8>,
        min_storage_rent_index: usize,
        update_nft_index: usize,
    ) -> Result<Self, BallotContractParametersError> {
        let ergo_tree = ErgoTree::sigma_parse_bytes(ergo_tree_bytes.as_slice())?;
        let min_storage_rent = ergo_tree
            .get_constant(min_storage_rent_index)
            .map_err(|_| BallotContractParametersError::NoMinStorageRent)?
            .ok_or(BallotContractParametersError::NoMinStorageRent)?
            .try_extract_into::<i64>()? as u64;

        Ok(Self {
            ergo_tree_bytes,
            min_storage_rent_index,
            min_storage_rent,
            update_nft_index,
        })
    }

    pub fn checked_load(
        ergo_tree_bytes: Vec<u8>,
        min_storage_rent: u64,
        min_storage_rent_index: usize,
        update_nft_index: usize,
    ) -> Result<Self, BallotContractParametersError> {
        let ergo_tree = ErgoTree::sigma_parse_bytes(ergo_tree_bytes.as_slice())?;
        let actual_min_storage_rent = ergo_tree
            .get_constant(min_storage_rent_index)
            .map_err(|_| BallotContractParametersError::NoMinStorageRent)?
            .ok_or(BallotContractParametersError::NoMinStorageRent)?
            .try_extract_into::<i64>()? as u64;
        if actual_min_storage_rent != min_storage_rent {
            return Err(BallotContractParametersError::MinStorageRentDiffers {
                expected: min_storage_rent,
                actual: actual_min_storage_rent,
            });
        }

        Ok(Self {
            ergo_tree_bytes,
            min_storage_rent_index,
            min_storage_rent,
            update_nft_index,
        })
    }

    pub fn ergo_tree_bytes(&self) -> Vec<u8> {
        self.ergo_tree_bytes.clone()
    }

    pub fn min_storage_rent_index(&self) -> usize {
        self.min_storage_rent_index
    }

    pub fn min_storage_rent(&self) -> u64 {
        self.min_storage_rent
    }

    pub fn update_nft_index(&self) -> usize {
        self.update_nft_index
    }
}

#[cfg(test)]
mod tests {
    use sigma_test_util::force_any_val;

    use super::*;

    #[test]
    fn test_constant_parsing() {
        let contract_parameters = BallotContractParameters::default();
        let update_nft_token_id = force_any_val::<TokenId>();
        let inputs = BallotContractInputs {
            contract_parameters,
            update_nft_token_id: update_nft_token_id.clone(),
        };
        let c = BallotContract::build_with(&inputs).unwrap();
        assert_eq!(c.update_nft_token_id(), update_nft_token_id);
    }
}
