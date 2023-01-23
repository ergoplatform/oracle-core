use super::assets_exchange_rate::Asset;
use super::assets_exchange_rate::AssetsExchangeRateSource;
use super::DataPointSource;
use super::DataPointSourceError;

pub struct DataPointSourceAggregator<PER1: Asset, GET: Asset> {
    pub fetchers: Vec<Box<dyn AssetsExchangeRateSource<PER1, GET>>>,
}

impl<PER1: Asset, GET: Asset> DataPointSourceAggregator<PER1, GET> {
    pub fn new(fetchers: Vec<Box<dyn AssetsExchangeRateSource<PER1, GET>>>) -> Self {
        Self { fetchers }
    }

    pub async fn fetch_datapoints(&self) -> Result<Vec<i64>, DataPointSourceError> {
        let mut futures = Vec::new();
        for fetcher in &self.fetchers {
            futures.push(fetcher.get_rate());
        }
        let results = futures::future::join_all(futures).await;
        let ok_results: Vec<i64> = results
            .into_iter()
            .flat_map(|res| res.ok())
            .map(|r| r.rate as i64)
            .collect();
        Ok(ok_results)
    }
}

fn aggregate(rates: Vec<i64>) -> i64 {
    // TODO: filter out outliers if > 2 datapoints?
    let average = rates.iter().sum::<i64>() / rates.len() as i64;
    average
}

impl<PER1: Asset, GET: Asset> DataPointSource for DataPointSourceAggregator<PER1, GET> {
    fn get_datapoint(&self) -> Result<i64, DataPointSourceError> {
        let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
        let rates = tokio_runtime.block_on(self.fetch_datapoints())?;
        Ok(aggregate(rates))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregate() {
        assert_eq!(aggregate(vec![1, 2, 3]), 2);
        assert_eq!(aggregate(vec![1, 3]), 2);
    }
}
