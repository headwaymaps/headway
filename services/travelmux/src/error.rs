use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};
use std::error::Error as StdError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum ErrorType {
    User,
    Server,
}

#[derive(Debug)]
#[allow(unused)]
pub struct Error {
    error_type: ErrorType,
    source: Box<dyn StdError>,
}

impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut my_struct = serializer.serialize_struct("Error", 2)?;
        my_struct.serialize_field("error_type", &self.error_type)?;
        my_struct.serialize_field("message", &self.source.to_string())?;
        my_struct.end()
    }
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn server(source: impl Into<Box<dyn StdError>> + 'static) -> Self {
        Self {
            source: source.into(),
            error_type: ErrorType::Server,
        }
    }

    pub fn user(source: impl Into<Box<dyn StdError>> + 'static) -> Self {
        Self {
            source: source.into(),
            error_type: ErrorType::User,
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::server(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::server(err)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{self:?}")
    }
}

impl actix_web::ResponseError for Error {}
