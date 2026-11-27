use thiserror::Error;

use crate::oracle_types::Rate;

use super::DataPointSource;
use super::DataPointSourceError;

#[derive(Debug, Error)]
pub enum ExternalScriptError {
    #[error("external script child process error: {0}")]
    ChildProcess(#[from] std::io::Error),
    #[error("external script exited with {status}: {stderr}")]
    NonZeroExitStatus {
        status: std::process::ExitStatus,
        stderr: String,
    },
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
        if !script_output.status.success() {
            return Err(ExternalScriptError::NonZeroExitStatus {
                status: script_output.status,
                stderr: String::from_utf8_lossy(&script_output.stderr)
                    .trim()
                    .to_string(),
            }
            .into());
        }
        let datapoint_str =
            String::from_utf8(script_output.stdout).map_err(ExternalScriptError::from)?;
        datapoint_str
            .trim()
            .parse::<i64>()
            .map_err(|e| DataPointSourceError::from(ExternalScriptError::from(e)))
            .map(Into::into)
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    // Serialize script creation and execution in tests to avoid Linux ETXTBSY races
    // when another test thread forks while a newly written script descriptor is open.
    static SCRIPT_EXEC_LOCK: Mutex<()> = Mutex::new(());

    struct TempScript {
        path: PathBuf,
    }

    impl TempScript {
        fn new(body: &str) -> Self {
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "oracle_core_ext_script_test_{}_{}.sh",
                std::process::id(),
                id
            ));
            fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
            let mut perms = fs::metadata(&path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&path, perms).unwrap();
            Self { path }
        }

        fn path_str(&self) -> String {
            self.path.to_string_lossy().into_owned()
        }
    }

    impl Drop for TempScript {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
        }
    }

    #[test]
    fn test_external_script_success() {
        let _guard = SCRIPT_EXEC_LOCK.lock().unwrap();
        let script = TempScript::new("printf '424242\\n'");
        let src = ExternalScript::new(script.path_str());
        assert_eq!(src.get_datapoint().unwrap(), Rate::from(424242));
    }

    #[test]
    fn test_external_script_nonzero_exit_rejected() {
        let _guard = SCRIPT_EXEC_LOCK.lock().unwrap();
        let script = TempScript::new("printf '424242\\n'; printf 'upstream failed\\n' >&2; exit 2");
        let src = ExternalScript::new(script.path_str());
        let err = src.get_datapoint().unwrap_err();
        assert!(
            matches!(
                err,
                DataPointSourceError::ExternalScript(ExternalScriptError::NonZeroExitStatus { .. })
            ),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn test_external_script_invalid_integer_rejected() {
        let _guard = SCRIPT_EXEC_LOCK.lock().unwrap();
        let script = TempScript::new("printf 'not-an-int\\n'");
        let src = ExternalScript::new(script.path_str());
        let err = src.get_datapoint().unwrap_err();
        assert!(
            matches!(
                err,
                DataPointSourceError::ExternalScript(ExternalScriptError::ParseInt(_))
            ),
            "unexpected error: {err:?}"
        );
    }
}
