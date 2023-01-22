use super::erg_xau::kgau_nanoerg_aggregator;
use super::DataPointSource;
use super::NanoAdaUsd;
use super::NanoErgUsd;
use super::PredefinedDataPointSource;

pub fn data_point_source_from_predef(
    predef_datasource: PredefinedDataPointSource,
) -> Box<dyn DataPointSource> {
    // TODO: transform the rest and add more fetchers
    match predef_datasource {
        PredefinedDataPointSource::NanoErgUsd => Box::new(NanoErgUsd),
        PredefinedDataPointSource::NanoErgXau => kgau_nanoerg_aggregator(),
        PredefinedDataPointSource::NanoAdaUsd => Box::new(NanoAdaUsd),
    }
}