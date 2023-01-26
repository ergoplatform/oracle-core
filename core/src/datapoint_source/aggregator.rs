use std::pin::Pin;

use futures::Future;

use super::assets_exchange_rate::Asset;
use super::assets_exchange_rate::AssetsExchangeRate;
use super::DataPointSourceError;

pub fn aggregate<PER1: Asset, GET: Asset>(
    rates: Vec<AssetsExchangeRate<PER1, GET>>,
) -> AssetsExchangeRate<PER1, GET> {
    // TODO: filter out outliers if > 2 datapoints?
    let average = rates.iter().map(|r| r.rate).sum::<f64>() / rates.len() as f64;
    AssetsExchangeRate {
        rate: average,
        ..rates[0]
    }
}

#[allow(clippy::type_complexity)]
pub async fn fetch_aggregated<PER1: Asset, GET: Asset>(
    sources: Vec<
        Pin<Box<dyn Future<Output = Result<AssetsExchangeRate<PER1, GET>, DataPointSourceError>>>>,
    >,
) -> Result<AssetsExchangeRate<PER1, GET>, DataPointSourceError> {
    let results = futures::future::join_all(sources).await;
    let ok_results: Vec<AssetsExchangeRate<PER1, GET>> =
        results.into_iter().flat_map(|res| res.ok()).collect();
    if ok_results.is_empty() {
        return Err(DataPointSourceError::NoDataPoints);
    }
    let rate = aggregate(ok_results);
    Ok(rate)
}
