use std::error::Error;
use std::fmt::{Display, Formatter};
use crate::js::JsError;
use crate::mrc::UpgradeError;

#[derive(Debug)]
pub enum DeftError {
    InvalidState,
    InvalidParameter,
    Internal(String),
}

impl Display for DeftError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DeftError::InvalidState => f.write_str("invalid state"),
            DeftError::InvalidParameter => f.write_str("invalid parameter"),
            DeftError::Internal(message) => f.write_str(message),
        }
    }
}

impl Error for DeftError {

}

impl From<UpgradeError> for DeftError {
    fn from(_value: UpgradeError) -> Self {
        DeftError::InvalidState
    }
}

impl From<JsError> for DeftError {
    fn from(value: JsError) -> Self {
        DeftError::Internal(value.to_string())
    }
}

pub type DeftResult<T> = Result<T, DeftError>;
