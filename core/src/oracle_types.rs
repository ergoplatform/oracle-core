use serde::Deserialize;
use serde::Serialize;

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(transparent)]
pub struct BlockHeight(pub u32);

impl std::ops::Sub<EpochLength> for BlockHeight {
    type Output = BlockHeight;
    fn sub(self, other: EpochLength) -> BlockHeight {
        BlockHeight(self.0 - other.0 as u32)
    }
}

impl std::ops::Sub<u32> for BlockHeight {
    type Output = BlockHeight;
    fn sub(self, other: u32) -> BlockHeight {
        // Unwrap here to panic on overflow instead of wrapping around
        BlockHeight(self.0.checked_sub(other).unwrap())
    }
}

impl std::fmt::Display for BlockHeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(transparent)]
pub struct EpochLength(pub i32);

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(transparent)]
pub struct EpochCounter(pub u32);

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(transparent)]
pub struct MinDatapoints(pub i32);
