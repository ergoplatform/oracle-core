//! Datapoint sources for oracle-core
mod ada_usd;
mod aggregator;
mod custom_ext_script;
mod erg_usd;
pub mod erg_xau;
mod predef;

use crate::pool_config::PredefinedDataPointSource;

use self::custom_ext_script::ExternalScript;
use self::custom_ext_script::ExternalScriptError;
use self::predef::data_point_source_from_predef;
pub use ada_usd::NanoAdaUsd;
pub use erg_usd::NanoErgUsd;

use anyhow::anyhow;
use derive_more::From;
use futures::future::BoxFuture;
use thiserror::Error;

pub fn load_datapoint_source(
    predef_datapoint_source: Option<PredefinedDataPointSource>,
    custom_datapoint_source_shell_cmd: Option<String>,
) -> Result<Box<dyn DataPointSource>, anyhow::Error> {
    if let Some(external_script_name) = custom_datapoint_source_shell_cmd.clone() {
        Ok(Box::new(ExternalScript::new(external_script_name.clone())))
    } else {
        match predef_datapoint_source {
            Some(predef_datasource) => Ok(data_point_source_from_predef(predef_datasource)),
            _ => Err(anyhow!(
                "pool config data_point_source is empty along with data_point_source_custom_script in the oracle config"
            )),
        }
    }
}

pub trait DataPointSource {
    fn get_datapoint(&self) -> Result<i64, DataPointSourceError>;

    // fn get_datapoint_retry(&self, retries: u8) -> Result<i64, DataPointSourceError> {
    //     let mut last_error = None;
    //     for _ in 0..retries {
    //         match self.get_datapoint() {
    //             Ok(datapoint) => return Ok(datapoint),
    //             Err(err) => {
    //                 log::warn!("Failed to get datapoint from source: {}, retrying ...", err);
    //                 last_error = Some(err)
    //             }
    //         }
    //     }
    //     Err(last_error.unwrap())
    // }
}

#[derive(Debug, From, Error)]
pub enum DataPointSourceError {
    #[error("external script error: {0}")]
    ExternalScript(ExternalScriptError),
    #[error("Reqwest error: {0}")]
    Reqwest(reqwest::Error),
    #[error("JSON parse error: {0}")]
    JsonParse(json::Error),
    #[error("Missing JSON field")]
    JsonMissingField,
}

pub trait Asset {}

pub struct NanoErg {}
pub struct Erg {}
pub struct KgAu {}
pub struct Xau {}
pub struct Usd {}

impl Asset for Erg {}
impl Asset for NanoErg {}
impl Asset for KgAu {}
impl Asset for Xau {}
impl Asset for Usd {}

pub struct AssetsExchangeRate<PER1: Asset, GET: Asset> {
    per1: PER1,
    get: GET,
    rate: f64,
}

pub trait AssetsExchangeRateSource<L: Asset, R: Asset> {
    fn get_rate(&self) -> BoxFuture<Result<AssetsExchangeRate<L, R>, DataPointSourceError>>;
}
