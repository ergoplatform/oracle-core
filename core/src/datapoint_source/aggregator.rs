use std::pin::Pin;
use std::time::Duration;

use futures::Future;

use super::assets_exchange_rate::Asset;
use super::assets_exchange_rate::AssetsExchangeRate;
use super::DataPointSourceError;

const SOURCE_TIMEOUT: Duration = Duration::from_secs(15);

pub type RateResult<PER1, GET> = Result<AssetsExchangeRate<PER1, GET>, DataPointSourceError>;

/// A price source: its name (for the logs) and the future that fetches its rate.
pub struct Source<PER1: Asset, GET: Asset> {
    pub name: &'static str,
    pub rate: Pin<Box<dyn Future<Output = RateResult<PER1, GET>>>>,
}

pub fn source<PER1: Asset, GET: Asset>(
    name: &'static str,
    rate: impl Future<Output = RateResult<PER1, GET>> + 'static,
) -> Source<PER1, GET> {
    Source {
        name,
        rate: Box::pin(rate),
    }
}

pub fn aggregate<PER1: Asset, GET: Asset>(
    rates: Vec<AssetsExchangeRate<PER1, GET>>,
) -> AssetsExchangeRate<PER1, GET> {
    // median rather than mean, so one bad source among many cannot move the rate
    let mut sorted: Vec<f64> = rates.iter().map(|r| r.rate).collect();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let mid = sorted.len() / 2;
    let median = if sorted.len() % 2 == 0 {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    };
    AssetsExchangeRate {
        rate: median,
        ..rates[0]
    }
}

/// Median of the sources that answered with a valid rate. Fails if fewer than
/// `min_sources` (at least 1) did: skipping a datapoint is safer than posting a
/// price that too few sources agree on.
pub async fn fetch_aggregated<PER1: Asset, GET: Asset>(
    pair: &'static str,
    sources: Vec<Source<PER1, GET>>,
    min_sources: usize,
) -> RateResult<PER1, GET> {
    let rates = fetch(pair, sources).await;
    let min_sources = min_sources.max(1);
    if rates.is_empty() {
        return Err(DataPointSourceError::NoDataPoints);
    }
    if rates.len() < min_sources {
        return Err(DataPointSourceError::NotEnoughSources {
            pair,
            got: rates.len(),
            min: min_sources,
        });
    }
    let count = rates.len();
    let rate = aggregate(rates);
    check_rate(rate.rate)?;
    log::debug!("{pair}: {} (median of {count} sources)", rate.rate);
    Ok(rate)
}

/// Fetches all sources concurrently and returns the valid rates. Failed, timed
/// out and invalid (non-finite or non-positive) sources are logged and left out.
pub async fn fetch<PER1: Asset, GET: Asset>(
    pair: &'static str,
    sources: Vec<Source<PER1, GET>>,
) -> Vec<AssetsExchangeRate<PER1, GET>> {
    let names: Vec<&str> = sources.iter().map(|s| s.name).collect();
    // a source that hangs must not hold up the others
    let results = futures::future::join_all(
        sources
            .into_iter()
            .map(|s| tokio::time::timeout(SOURCE_TIMEOUT, s.rate)),
    )
    .await;
    names
        .into_iter()
        .zip(results)
        .filter_map(|(name, res)| match res {
            Ok(Ok(rate)) => match check_rate(rate.rate) {
                Ok(()) => {
                    log::debug!("{pair} source {name}: {}", rate.rate);
                    Some(rate)
                }
                Err(e) => {
                    log::warn!("{pair} source {name} failed: {e}");
                    None
                }
            },
            Ok(Err(e)) => {
                log::warn!("{pair} source {name} failed: {e}");
                None
            }
            Err(_) => {
                log::warn!(
                    "{pair} source {name} timed out after {} s",
                    SOURCE_TIMEOUT.as_secs()
                );
                None
            }
        })
        .collect()
}

fn check_rate(rate: f64) -> Result<(), DataPointSourceError> {
    if rate.is_finite() && rate > 0.0 {
        Ok(())
    } else {
        Err(DataPointSourceError::InvalidRate(rate))
    }
}
