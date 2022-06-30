//! Datapoint sources for oracle-core
mod ada_usd;
mod erg_usd;
mod erg_xau;
use derive_more::From;
use thiserror::Error;

pub trait DataPointSource: std::fmt::Debug {
    fn get_datapoint(&self) -> Result<i64, DataPointSourceError>;
}

#[derive(Debug, From, Error)]
pub enum DataPointSourceError {
    #[error("external script error: {0}")]
    ExternalScript(ExternalScriptError),
    #[error("Reqwest error: {0}")]
    Reqwest(reqwest::Error),
    #[error("JSON parse error: {0}")]
    JsonParse(json::Error),
    #[error("Missing JSON field")]
    JsonMissingField,
}

#[derive(Debug, From, Error)]
pub enum ExternalScriptError {
    #[error("external script child process error: {0}")]
    ChildProcess(std::io::Error),
    #[error("String from bytes error: {0}")]
    StringFromBytes(std::string::FromUtf8Error),
    #[error("Parse i64 from string error: {0}")]
    ParseInt(std::num::ParseIntError),
}

#[derive(Debug, Clone)]
pub struct ExternalScript(String);

impl ExternalScript {
    pub fn new(script_name: String) -> Self {
        ExternalScript(script_name)
    }
}

impl DataPointSource for ExternalScript {
    fn get_datapoint(&self) -> Result<i64, DataPointSourceError> {
        let script_output = std::process::Command::new(&self.0)
            .output()
            .map_err(ExternalScriptError::from)?;
        let datapoint_str =
            String::from_utf8(script_output.stdout).map_err(ExternalScriptError::from)?;
        datapoint_str
            .parse()
            .map_err(|e| DataPointSourceError::from(ExternalScriptError::from(e)))
    }
}

pub use ada_usd::NanoAdaUsd;
pub use erg_usd::NanoErgUsd;
pub use erg_xau::NanoErgXau;

#[derive(serde::Serialize, serde::Deserialize, Debug, Copy, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum PredefinedDataPointSource {
    NanoErgUsd,
    NanoErgXau,
    NanoAdaUsd,
}

impl DataPointSource for PredefinedDataPointSource {
    fn get_datapoint(&self) -> Result<i64, DataPointSourceError> {
        match self {
            PredefinedDataPointSource::NanoAdaUsd => NanoAdaUsd.get_datapoint(),
            PredefinedDataPointSource::NanoErgUsd => NanoErgUsd.get_datapoint(),
            PredefinedDataPointSource::NanoErgXau => NanoErgXau.get_datapoint(),
        }
    }
}
