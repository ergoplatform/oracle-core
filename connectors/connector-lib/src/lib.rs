// Coding conventions
#![allow(dead_code)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::unit_arg)]
#![forbid(unsafe_code)]
#![deny(non_upper_case_globals)]
#![deny(non_camel_case_types)]
#![deny(non_snake_case)]
#![deny(unused_mut)]
#![deny(unused_imports)]
#![deny(clippy::wildcard_enum_match_arm)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]

#[macro_use]
extern crate json;

pub mod connector;
pub mod oracle_core;

pub use connector::{Connector, Datapoint};
pub use oracle_core::{get_core_api_port, OracleCore};
