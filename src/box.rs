/// The Ergo `Box` struct definition & json parsing to generate said struct

use crate::NanoErg;


/// Rust representation of an Ergo box
struct Box {
    box_id: String,
    value: NanoErg,
    ergo_tree: String,
    creation_height: u64,
    assets: Vec<Asset>,
    registers: Vec<Register>,
    transaction_id: String,
    index: u64,
}

/// Representation of an Ergo asset/token
struct Asset {
    token_id: String,
    amount: u64,
}

/// Representation of an Ergo box register
struct Register {
    register: String,
    value: String,
}