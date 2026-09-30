//! Datapoint sources for oracle-core
mod ada_usd;
mod aggregator;
mod assets_exchange_rate;
mod bitpanda;
mod coingecko;
mod coinpaprika;
mod custom_ext_script;
mod erg_btc;
mod erg_usd;
mod erg_xau;
mod exchanges;
mod predef;

use crate::oracle_types::Rate;
use crate::pool_config::PredefinedDataPointSource;

use self::custom_ext_script::ExternalScript;
use self::custom_ext_script::ExternalScriptError;
use self::predef::sync_fetch_predef_source_aggregated;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub trait DataPointSource {
    fn get_datapoint(&self) -> Result<Rate, DataPointSourceError>;
}

#[derive(Debug, Error)]
pub enum DataPointSourceError {
    #[error("external script error: {0}")]
    ExternalScript(#[from] ExternalScriptError),
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] json::Error),
    #[error("Missing JSON field {field} in {json}")]
    JsonMissingField { field: String, json: String },
    #[error("No datapoints from any source")]
    NoDataPoints,
    #[error("Not enough {pair} price sources: {got} answered, {min} required")]
    NotEnoughSources {
        pair: &'static str,
        got: usize,
        min: usize,
    },
    #[error("Invalid rate {0}, must be finite and positive")]
    InvalidRate(f64),
    #[error("Stale quote: {field} is {age_secs} s old, limit {max_age_secs} s")]
    StaleQuote {
        field: String,
        age_secs: i64,
        max_age_secs: u64,
    },
}

/// Minimum number of price sources that must answer before a datapoint is
/// posted, per price leg. Set in oracle_config.yaml under `min_price_sources`;
/// missing keys take the defaults below.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct MinPriceSources {
    /// ERG/USD, also the ERG leg of ERG/XAU
    pub erg_usd: usize,
    /// gold/USD leg of ERG/XAU
    pub gold_usd: usize,
}

impl Default for MinPriceSources {
    fn default() -> Self {
        Self {
            erg_usd: 3,
            gold_usd: 2,
        }
    }
}

pub enum RuntimeDataPointSource {
    Predefined(PredefinedDataPointSource, MinPriceSources),
    ExternalScript(ExternalScript),
}

impl RuntimeDataPointSource {
    pub fn new(
        predef_datapoint_source: Option<PredefinedDataPointSource>,
        custom_datapoint_source_shell_cmd: Option<String>,
        min_price_sources: MinPriceSources,
    ) -> Result<RuntimeDataPointSource, anyhow::Error> {
        if let Some(external_script_name) = custom_datapoint_source_shell_cmd.clone() {
            Ok(RuntimeDataPointSource::ExternalScript(ExternalScript::new(
                external_script_name.clone(),
            )))
        } else {
            match predef_datapoint_source {
                Some(predef_datasource) => Ok(RuntimeDataPointSource::Predefined(
                    predef_datasource,
                    min_price_sources,
                )),
                _ => Err(anyhow!(
                    "pool config data_point_source is empty along with data_point_source_custom_script in the oracle config"
                )),
            }
        }
    }
}

impl DataPointSource for RuntimeDataPointSource {
    fn get_datapoint(&self) -> Result<Rate, DataPointSourceError> {
        match self {
            RuntimeDataPointSource::Predefined(predef, min_price_sources) => {
                sync_fetch_predef_source_aggregated(predef, *min_price_sources)
            }
            RuntimeDataPointSource::ExternalScript(script) => script.get_datapoint(),
        }
    }
}
