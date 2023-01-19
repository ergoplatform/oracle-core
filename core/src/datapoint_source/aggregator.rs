use futures::future::BoxFuture;

use super::DataPointSource;
use super::DataPointSourceError;

pub trait DataPointFetcher: std::fmt::Debug {
    fn get_datapoint(&self) -> BoxFuture<'static, Result<i64, DataPointSourceError>>;
}

#[derive(Debug)]
pub struct DataPointSourceAggregator {
    pub fetchers: Vec<Box<dyn DataPointFetcher>>,
}

impl DataPointSourceAggregator {
    pub async fn fetch_datapoints_average(&self) -> Result<i64, DataPointSourceError> {
        let mut futures = Vec::new();
        for fetcher in &self.fetchers {
            futures.push(fetcher.get_datapoint());
        }
        let results = futures::future::join_all(futures).await;
        let ok_results: Vec<i64> = results.into_iter().flat_map(|res| res.ok()).collect();
        let average = ok_results.iter().sum::<i64>() / ok_results.len() as i64;
        Ok(average)
    }
}

impl DataPointSource for DataPointSourceAggregator {
    fn get_datapoint(&self) -> Result<i64, DataPointSourceError> {
        let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
        tokio_runtime.block_on(self.fetch_datapoints_average())
    }
}
