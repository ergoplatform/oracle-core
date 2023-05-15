use thiserror::Error;

use crate::oracle_types::Rate;

use super::DataPointSource;
use super::DataPointSourceError;

#[derive(Debug, Error)]
pub enum ExternalScriptError {
    #[error("external script child process error: {0}")]
    ChildProcess(#[from] std::io::Error),
    #[error("String from bytes error: {0}")]
    StringFromBytes(#[from] std::string::FromUtf8Error),
    #[error("Parse i64 from string error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),
}

#[derive(Debug, Clone)]
pub struct ExternalScript(String);

impl ExternalScript {
    pub fn new(script_name: String) -> Self {
        ExternalScript(script_name)
    }
}

impl DataPointSource for ExternalScript {
    fn get_datapoint(&self) -> Result<Rate, DataPointSourceError> {
        let script_output = std::process::Command::new(&self.0)
            .output()
            .map_err(ExternalScriptError::from)?;
        let datapoint_str =
            String::from_utf8(script_output.stdout).map_err(ExternalScriptError::from)?;
        datapoint_str
            .parse::<i64>()
            .map_err(|e| DataPointSourceError::from(ExternalScriptError::from(e)))
            .map(Into::into)
    }
}
