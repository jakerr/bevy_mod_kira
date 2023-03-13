use std::error::Error;
use std::fmt::{self, Display};

#[derive(Debug)]
pub enum KiraPluginError {
    FailedToInitialize,
}

impl Display for KiraPluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KiraPluginError::FailedToInitialize => write!(f, "failed to initialize kira"),
        }
    }
}

impl Error for KiraPluginError {
    /*
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            KiraPluginError::FailedToInitialize => None,
        }
    }
    */
}
