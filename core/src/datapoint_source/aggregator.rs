use std::pin::Pin;
use std::time::Duration;

use futures::Future;

use super::assets_exchange_rate::Asset;
use super::assets_exchange_rate::AssetsExchangeRate;
use super::DataPointSourceError;

const SOURCE_TIMEOUT: Duration = Duration::from_secs(15);

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

#[allow(clippy::type_complexity)]
pub async fn fetch_aggregated<PER1: Asset, GET: Asset>(
    sources: Vec<
        Pin<Box<dyn Future<Output = Result<AssetsExchangeRate<PER1, GET>, DataPointSourceError>>>>,
    >,
) -> Result<AssetsExchangeRate<PER1, GET>, DataPointSourceError> {
    let ok_results: Vec<AssetsExchangeRate<PER1, GET>> = fetch(sources).await?;
    if ok_results.is_empty() {
        return Err(DataPointSourceError::NoDataPoints);
    }
    let rate = aggregate(ok_results);
    Ok(rate)
}

#[allow(clippy::type_complexity)]
pub async fn fetch<PER1: Asset, GET: Asset>(
    sources: Vec<
        Pin<Box<dyn Future<Output = Result<AssetsExchangeRate<PER1, GET>, DataPointSourceError>>>>,
    >,
) -> Result<Vec<AssetsExchangeRate<PER1, GET>>, DataPointSourceError> {
    // a source that hangs must not hold up the others
    let results = futures::future::join_all(
        sources
            .into_iter()
            .map(|s| tokio::time::timeout(SOURCE_TIMEOUT, s)),
    )
    .await;
    let ok_results: Vec<AssetsExchangeRate<PER1, GET>> = results
        .into_iter()
        .filter_map(|res| match res {
            Ok(Ok(rate)) => Some(rate),
            Ok(Err(e)) => {
                log::debug!("datapoint source failed: {e}");
                None
            }
            Err(_) => {
                log::debug!("datapoint source timed out");
                None
            }
        })
        .collect();
    Ok(ok_results)
}
