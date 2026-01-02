use tokio::{runtime::Runtime, task};

use super::error::AppRunError;

pub fn build_runtime() -> Result<(Runtime, task::LocalSet), AppRunError> {
    let rt = Runtime::new().map_err(|e| AppRunError::RuntimeInit(std::io::Error::other(e)))?;

    let local = task::LocalSet::new();
    Ok((rt, local))
}
