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

use std::sync::Mutex;
use std::time::{Duration, Instant};

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
    #[error("Quote time {field} is {ahead_secs} s in the future")]
    FutureQuote { field: String, ahead_secs: i64 },
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

/// How long a fetched rate is reused before the sources are queried again.
/// While a datapoint tx is unconfirmed the main loop keeps rebuilding it (every
/// 30 s, until the box is seen on chain), and each rebuild asks for the rate;
/// without the cache every rebuild hit every source, which tripped their rate
/// limits (CoinMarketCap, CoinGecko) and spent the LiveCoinWatch quota.
const RATE_CACHE_TTL: Duration = Duration::from_secs(120);

pub enum RuntimeDataPointSource {
    Predefined(PredefinedSource),
    ExternalScript(ExternalScript),
}

/// A predefined pair with its quorums, keys and the rate cache.
pub struct PredefinedSource {
    source: PredefinedDataPointSource,
    min_price_sources: MinPriceSources,
    livecoinwatch_api_key: Option<String>,
    cache: CachedRate,
}

impl RuntimeDataPointSource {
    pub fn new(
        predef_datapoint_source: Option<PredefinedDataPointSource>,
        custom_datapoint_source_shell_cmd: Option<String>,
        min_price_sources: MinPriceSources,
        livecoinwatch_api_key: Option<String>,
    ) -> Result<RuntimeDataPointSource, anyhow::Error> {
        if let Some(external_script_name) = custom_datapoint_source_shell_cmd.clone() {
            Ok(RuntimeDataPointSource::ExternalScript(ExternalScript::new(
                external_script_name.clone(),
            )))
        } else {
            match predef_datapoint_source {
                Some(predef_datasource) => {
                    Ok(RuntimeDataPointSource::Predefined(PredefinedSource {
                        source: predef_datasource,
                        min_price_sources,
                        livecoinwatch_api_key: livecoinwatch_api_key.filter(|k| !k.is_empty()),
                        cache: CachedRate::new(RATE_CACHE_TTL),
                    }))
                }
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
            RuntimeDataPointSource::Predefined(predef) => predef.cache.get_or_fetch(|| {
                sync_fetch_predef_source_aggregated(
                    &predef.source,
                    predef.min_price_sources,
                    predef.livecoinwatch_api_key.as_deref(),
                )
            }),
            RuntimeDataPointSource::ExternalScript(script) => script.get_datapoint(),
        }
    }
}

/// The last successfully fetched rate, reused until it is older than `ttl`.
/// Errors are not cached: the next call fetches again.
pub struct CachedRate {
    ttl: Duration,
    last: Mutex<Option<(Rate, Instant)>>,
}

impl CachedRate {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            last: Mutex::new(None),
        }
    }

    pub fn get_or_fetch(
        &self,
        fetch: impl FnOnce() -> Result<Rate, DataPointSourceError>,
    ) -> Result<Rate, DataPointSourceError> {
        let mut last = self.last.lock().unwrap();
        if let Some((rate, fetched_at)) = *last {
            let age = fetched_at.elapsed();
            if age < self.ttl {
                log::debug!("using the rate fetched {} s ago: {rate}", age.as_secs());
                return Ok(rate);
            }
        }
        let rate = fetch()?;
        *last = Some((rate, Instant::now()));
        Ok(rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn test_rate_cache_reuses_within_ttl() {
        let cache = CachedRate::new(Duration::from_millis(50));
        let calls = Cell::new(0);
        let fetch = || {
            calls.set(calls.get() + 1);
            Ok(Rate::from(calls.get() as i64))
        };
        // two calls inside the TTL: one fetch, the same rate
        assert_eq!(cache.get_or_fetch(fetch).unwrap(), Rate::from(1));
        assert_eq!(cache.get_or_fetch(fetch).unwrap(), Rate::from(1));
        assert_eq!(calls.get(), 1);
        // after the TTL: fetched again
        std::thread::sleep(Duration::from_millis(60));
        assert_eq!(cache.get_or_fetch(fetch).unwrap(), Rate::from(2));
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn test_rate_cache_does_not_keep_errors() {
        let cache = CachedRate::new(Duration::from_secs(60));
        let calls = Cell::new(0);
        let fetch = || {
            calls.set(calls.get() + 1);
            if calls.get() == 1 {
                Err(DataPointSourceError::NoDataPoints)
            } else {
                Ok(Rate::from(7))
            }
        };
        assert!(cache.get_or_fetch(fetch).is_err());
        assert_eq!(cache.get_or_fetch(fetch).unwrap(), Rate::from(7));
        assert_eq!(cache.get_or_fetch(fetch).unwrap(), Rate::from(7));
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn test_livecoinwatch_key_from_config_only() {
        // the shared LCW_API_KEY env var is not read: it belonged to another program
        // on the operator's host and this one silently spent its quota
        std::env::set_var("LCW_API_KEY", "env-key");
        let with = |key: Option<&str>| match RuntimeDataPointSource::new(
            Some(PredefinedDataPointSource::NanoErgUsd),
            None,
            MinPriceSources::default(),
            key.map(String::from),
        )
        .unwrap()
        {
            RuntimeDataPointSource::Predefined(p) => p.livecoinwatch_api_key,
            RuntimeDataPointSource::ExternalScript(_) => unreachable!(),
        };
        assert_eq!(with(None), None);
        assert_eq!(with(Some("")), None);
        assert_eq!(with(Some("cfg-key")), Some("cfg-key".to_string()));
        std::env::remove_var("LCW_API_KEY");
    }
}
