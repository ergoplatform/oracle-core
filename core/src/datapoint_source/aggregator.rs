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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datapoint_source::assets_exchange_rate::{NanoErg, Usd};

    fn rate(r: f64) -> AssetsExchangeRate<Usd, NanoErg> {
        AssetsExchangeRate {
            per1: Usd {},
            get: NanoErg {},
            rate: r,
        }
    }

    fn ok(name: &'static str, r: f64) -> Source<Usd, NanoErg> {
        source(name, futures::future::ready(Ok(rate(r))))
    }

    fn failing(name: &'static str) -> Source<Usd, NanoErg> {
        source(
            name,
            futures::future::ready(Err(DataPointSourceError::NoDataPoints)),
        )
    }

    fn aggregated(sources: Vec<Source<Usd, NanoErg>>, min: usize) -> RateResult<Usd, NanoErg> {
        tokio_test::block_on(fetch_aggregated("ERG/USD", sources, min))
    }

    #[test]
    fn test_aggregate_median() {
        let median = |rs: &[f64]| aggregate(rs.iter().map(|r| rate(*r)).collect()).rate;
        assert_eq!(median(&[5.0]), 5.0);
        assert_eq!(median(&[5.0, 3.0]), 4.0);
        assert_eq!(median(&[5.0, 1.0, 3.0]), 3.0);
        assert_eq!(median(&[5.0, 1.0, 100.0, 3.0]), 4.0);
        // one outlier among several does not move it
        assert_eq!(median(&[3.0, 3.1, 2.9, 3.0, 1000.0]), 3.0);
    }

    #[test]
    fn test_quorum_boundaries() {
        let sources = |n: usize| {
            ["a", "b", "c", "d"][..n]
                .iter()
                .map(|name| ok(name, 3.0))
                .collect::<Vec<_>>()
        };
        // min - 1
        assert!(matches!(
            aggregated(sources(2), 3),
            Err(DataPointSourceError::NotEnoughSources { got: 2, min: 3, .. })
        ));
        // min
        assert_eq!(aggregated(sources(3), 3).unwrap().rate, 3.0);
        // min + 1
        assert_eq!(aggregated(sources(4), 3).unwrap().rate, 3.0);
    }

    #[test]
    fn test_failed_sources_do_not_count() {
        let sources = vec![ok("a", 1.0), ok("b", 2.0), failing("c"), failing("d")];
        assert!(matches!(
            aggregated(sources, 3),
            Err(DataPointSourceError::NotEnoughSources { got: 2, min: 3, .. })
        ));
        let sources = vec![ok("a", 1.0), ok("b", 2.0), failing("c"), ok("d", 3.0)];
        assert_eq!(aggregated(sources, 3).unwrap().rate, 2.0);
    }

    #[test]
    fn test_invalid_rates_do_not_count() {
        let sources = vec![
            ok("a", 1.0),
            ok("b", 2.0),
            ok("nan", f64::NAN),
            ok("inf", f64::INFINITY),
            ok("zero", 0.0),
            ok("negative", -2.0),
        ];
        assert!(matches!(
            aggregated(sources, 3),
            Err(DataPointSourceError::NotEnoughSources { got: 2, min: 3, .. })
        ));
        let sources = vec![ok("a", 1.0), ok("b", 2.0), ok("nan", f64::NAN)];
        assert_eq!(aggregated(sources, 1).unwrap().rate, 1.5);
    }

    #[test]
    fn test_no_sources() {
        assert!(matches!(
            aggregated(vec![failing("a")], 1),
            Err(DataPointSourceError::NoDataPoints)
        ));
        // a minimum of 0 still needs one source
        assert!(matches!(
            aggregated(vec![], 0),
            Err(DataPointSourceError::NoDataPoints)
        ));
    }
}
