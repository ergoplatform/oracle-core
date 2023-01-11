use serde::Deserialize;
use serde::Serialize;

pub struct MinDataPoints(u32);

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(transparent)]
pub struct BlockHeight(pub u32);

impl Into<u32> for BlockHeight {
    fn into(self) -> u32 {
        self.0
    }
}

impl std::ops::Sub<EpochLength> for BlockHeight {
    type Output = BlockHeight;
    fn sub(self, other: EpochLength) -> BlockHeight {
        BlockHeight(self.0 - other.0 as u32)
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

impl Into<i32> for EpochLength {
    fn into(self) -> i32 {
        self.0
    }
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Serialize, Deserialize, Copy, Clone)]
#[serde(transparent)]
pub struct EpochCounter(pub u32);
