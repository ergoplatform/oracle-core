use crate::oracle_types::Rate;

use super::ada_usd::usd_lovelace_sources;
use super::aggregator::fetch_aggregated;
use super::erg_btc::nanoerg_btc_sources;
use super::erg_usd::nanoerg_usd_sources;
use super::erg_xau::nanoerg_kgau;
use super::DataPointSourceError;
use super::MinPriceSources;
use super::PredefinedDataPointSource;

pub fn sync_fetch_predef_source_aggregated(
    predef_datasource: &PredefinedDataPointSource,
    min_price_sources: MinPriceSources,
) -> Result<Rate, DataPointSourceError> {
    let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
    let rate = tokio_runtime.block_on(fetch_predef_source_aggregated(
        predef_datasource,
        min_price_sources,
    ))?;
    Ok(rate)
}

async fn fetch_predef_source_aggregated(
    predef_datasource: &PredefinedDataPointSource,
    min_price_sources: MinPriceSources,
) -> Result<Rate, DataPointSourceError> {
    let rate_float = match predef_datasource {
        PredefinedDataPointSource::NanoErgUsd => {
            fetch_aggregated("ERG/USD", nanoerg_usd_sources(), min_price_sources.erg_usd)
                .await?
                .rate
        }
        PredefinedDataPointSource::NanoErgXau => nanoerg_kgau(min_price_sources).await?.rate,
        PredefinedDataPointSource::NanoAdaUsd => {
            fetch_aggregated("ADA/USD", usd_lovelace_sources(), 1)
                .await?
                .rate
        }
        PredefinedDataPointSource::NanoErgBTC => {
            fetch_aggregated("ERG/BTC", nanoerg_btc_sources(), 1)
                .await?
                .rate
        }
    };
    Ok((rate_float as i64).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fetch(
        predef: PredefinedDataPointSource,
        min_price_sources: MinPriceSources,
    ) -> Result<Rate, DataPointSourceError> {
        tokio_test::block_on(fetch_predef_source_aggregated(&predef, min_price_sources))
    }

    #[test]
    fn test_quorum_reaches_datapoint() {
        let default = MinPriceSources::default();
        assert!(fetch(PredefinedDataPointSource::NanoErgUsd, default).is_ok());
        assert!(fetch(PredefinedDataPointSource::NanoErgXau, default).is_ok());
        let erg_usd_99 = MinPriceSources {
            erg_usd: 99,
            ..default
        };
        assert!(fetch(PredefinedDataPointSource::NanoErgUsd, erg_usd_99).is_err());
        assert!(fetch(PredefinedDataPointSource::NanoErgXau, erg_usd_99).is_err());
        let gold_usd_4 = MinPriceSources {
            gold_usd: 4,
            ..default
        };
        assert!(fetch(PredefinedDataPointSource::NanoErgUsd, gold_usd_4).is_ok());
        // CoinGecko's ERG/XAU still answers, but is not posted alone
        assert!(fetch(PredefinedDataPointSource::NanoErgXau, gold_usd_4).is_err());
    }
}
