//! Defines the general error type for the crate and various conversions into it
use std::convert;
use std::fmt;

/// General error type for the crate
#[derive(Debug)]
pub enum Error {
    ArrayConversionError,
    DuplicateFileError(String),
    ElevationRequestError(reqwest::StatusCode, String),
    FileDoesNotExistError(String),
    FileIdMessageNotFound(String),
    FitParser(fitparser::ErrorKind),
    Io(std::io::Error),
    Other(String),
    Rusqlite(rusqlite::Error),
    SerdeYamlError(serde_yaml::Error),
    UnknownServiceHandler(String),
    InvalidConfigurationValue(String),
}

impl convert::From<fitparser::Error> for Error {
    fn from(err: fitparser::Error) -> Error {
        Error::FitParser(*err)
    }
}

impl convert::From<fitparser::ErrorKind> for Error {
    fn from(err: fitparser::ErrorKind) -> Error {
        Error::FitParser(err)
    }
}

impl convert::From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Error {
        Error::Rusqlite(err)
    }
}

impl convert::From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl convert::From<serde_yaml::Error> for Error {
    fn from(err: serde_yaml::Error) -> Error {
        Error::SerdeYamlError(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ArrayConversionError => {
                write!(f, "Cannot convert Value:Array into a SQL parameter")
            }
            Error::DuplicateFileError(uuid) => write!(
                f,
                "Attempted to import a file already in the database, UUID: {}",
                uuid
            ),
            Error::ElevationRequestError(code, msg) => write!(
                f,
                "Elevation data request failed with code: {} - {}",
                code, msg
            ),
            Error::FileDoesNotExistError(uuid) => {
                write!(f, "FIT File with UUID='{}' does not exist", uuid)
            }
            Error::FileIdMessageNotFound(uuid) => write!(
                f,
                "FIT File with UUID='{}' did not have a File_id message",
                uuid
            ),
            Error::FitParser(e) => write!(f, "{}", e),
            Error::Io(e) => write!(f, "{}", e),
            Error::Other(msg) => write!(f, "{}", msg),
            Error::Rusqlite(e) => write!(f, "{}", e),
            Error::SerdeYamlError(e) => write!(f, "{}", e),
            Error::UnknownServiceHandler(msg) => write!(f, "{}", msg),
            Error::InvalidConfigurationValue(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for Error {}
