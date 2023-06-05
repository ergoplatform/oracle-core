use std::iter::Sum;

use derive_more::Add;
use derive_more::Display;
use derive_more::Div;
use derive_more::From;
use derive_more::Into;
use derive_more::Mul;
use derive_more::Sub;
use serde::Deserialize;
use serde::Serialize;

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Serialize, Deserialize, Copy, Clone, From)]
#[serde(transparent)]
pub struct BlockHeight(pub u32);

impl std::ops::Sub<EpochLength> for BlockHeight {
    type Output = BlockHeight;
    fn sub(self, other: EpochLength) -> BlockHeight {
        BlockHeight(self.0 - other.0 as u32)
    }
}

impl std::ops::Add<EpochLength> for BlockHeight {
    type Output = BlockHeight;
    fn add(self, other: EpochLength) -> BlockHeight {
        BlockHeight(self.0 + other.0 as u32)
    }
}

impl std::ops::Add<u32> for BlockHeight {
    type Output = BlockHeight;
    fn add(self, other: u32) -> BlockHeight {
        // Unwrap here to panic on overflow instead of wrapping around
        BlockHeight(self.0.checked_add(other).unwrap())
    }
}

impl std::ops::Sub<u32> for BlockHeight {
    type Output = BlockHeight;
    fn sub(self, other: u32) -> BlockHeight {
        // Unwrap here to panic on overflow instead of wrapping around
        BlockHeight(self.0.checked_sub(other).unwrap())
    }
}

impl From<BlockHeight> for i64 {
    fn from(block_height: BlockHeight) -> Self {
        block_height.0 as i64
    }
}

impl std::fmt::Display for BlockHeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Serialize, Deserialize, Copy, Clone, From)]
#[serde(transparent)]
pub struct EpochLength(pub i32);

impl From<EpochLength> for i64 {
    fn from(epoch_length: EpochLength) -> Self {
        epoch_length.0 as i64
    }
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Serialize, Deserialize, Copy, Clone, From)]
#[serde(transparent)]
pub struct EpochCounter(pub u32);

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Serialize, Deserialize, Copy, Clone, From)]
#[serde(transparent)]
pub struct MinDatapoints(pub i32);

#[derive(
    PartialEq,
    PartialOrd,
    Eq,
    Ord,
    Debug,
    Serialize,
    Deserialize,
    Copy,
    Clone,
    From,
    Into,
    Display,
    Add,
    Mul,
    Div,
    Sub,
)]
#[serde(transparent)]
pub struct Rate(i64);

impl Rate {
    pub fn as_f32(&self) -> f32 {
        self.0 as f32
    }
}

impl Sum for Rate {
    fn sum<I: Iterator<Item = Rate>>(iter: I) -> Rate {
        iter.fold(Rate(0), |acc, x| acc + x)
    }
}

impl PartialEq<i64> for Rate {
    fn eq(&self, other: &i64) -> bool {
        self.0 == *other
    }
}
