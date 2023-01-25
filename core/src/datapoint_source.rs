//! Datapoint sources for oracle-core
mod ada_usd;
mod aggregator;
mod assets_exchange_rate;
mod coincap;
mod coingecko;
mod custom_ext_script;
mod erg_usd;
pub mod erg_xau;
mod predef;

use crate::pool_config::PredefinedDataPointSource;

use self::custom_ext_script::ExternalScript;
use self::custom_ext_script::ExternalScriptError;
use self::predef::data_point_source_from_predef;

use anyhow::anyhow;
use derive_more::From;
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
    #[error("Missing JSON field {field} in {json}")]
    JsonMissingField { field: String, json: String },
}
