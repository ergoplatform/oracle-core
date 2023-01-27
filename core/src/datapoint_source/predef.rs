use super::ada_usd::usd_lovelace_sources;
use super::aggregator::fetch_aggregated;
use super::erg_usd::nanoerg_usd_sources;
use super::erg_xau::nanoerg_kgau_sources;
use super::DataPointSourceError;
use super::PredefinedDataPointSource;

pub fn sync_fetch_predef_source_aggregated(
    predef_datasource: &PredefinedDataPointSource,
) -> Result<i64, DataPointSourceError> {
    let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
    let rate = tokio_runtime.block_on(fetch_predef_source_aggregated(predef_datasource))?;
    Ok(rate)
}

async fn fetch_predef_source_aggregated(
    predef_datasource: &PredefinedDataPointSource,
) -> Result<i64, DataPointSourceError> {
    let rate_float = match predef_datasource {
        PredefinedDataPointSource::NanoErgUsd => {
            fetch_aggregated(nanoerg_usd_sources()).await?.rate
        }
        PredefinedDataPointSource::NanoErgXau => {
            fetch_aggregated(nanoerg_kgau_sources()).await?.rate
        }
        PredefinedDataPointSource::NanoAdaUsd => {
            fetch_aggregated(usd_lovelace_sources()).await?.rate
        }
    };
    Ok(rate_float as i64)
}
