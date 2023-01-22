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

    pub async fn fetch_datapoints_average(&self) -> Result<i64, DataPointSourceError> {
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
        let average = ok_results.iter().sum::<i64>() / ok_results.len() as i64;
        Ok(average)
    }
}

impl<PER1: Asset, GET: Asset> DataPointSource for DataPointSourceAggregator<PER1, GET> {
    fn get_datapoint(&self) -> Result<i64, DataPointSourceError> {
        let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
        tokio_runtime.block_on(self.fetch_datapoints_average())
    }
}
