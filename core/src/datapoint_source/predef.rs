use super::ada_usd::usd_lovelace_aggregator;
use super::erg_usd::usd_nanoerg_aggregator;
use super::erg_xau::kgau_nanoerg_aggregator;
use super::DataPointSource;
use super::PredefinedDataPointSource;

pub fn data_point_source_from_predef(
    predef_datasource: PredefinedDataPointSource,
) -> Box<dyn DataPointSource> {
    match predef_datasource {
        PredefinedDataPointSource::NanoErgUsd => usd_nanoerg_aggregator(),
        PredefinedDataPointSource::NanoErgXau => kgau_nanoerg_aggregator(),
        PredefinedDataPointSource::NanoAdaUsd => usd_lovelace_aggregator(),
    }
}
