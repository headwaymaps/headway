use serde::{ser::SerializeStruct, Deserialize, Serialize, Serializer};
use std::error::Error as StdError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorType {
    /// Generic error for bad user input
    User = 400,
    /// Generic error for when something goes wrong on the server
    Server = 500,
    /// The requested trip area is not covered by any routing graph.
    NoCoverageForArea = 1701,
}

impl TryFrom<u32> for ErrorType {
    type Error = ();

    fn try_from(value: u32) -> std::result::Result<Self, Self::Error> {
        match value {
            400 => Ok(Self::User),
            500 => Ok(Self::Server),
            1701 => Ok(Self::NoCoverageForArea),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
#[allow(unused)]
pub struct Error {
    pub(crate) error_type: ErrorType,
    pub(crate) source: Box<dyn StdError>,
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
    /// An error with some logic or state on the service. There is no meaningful
    /// output for the user here except maybe "try again later".
    pub fn server(source: impl Into<Box<dyn StdError>> + 'static) -> Self {
        Self {
            source: source.into(),
            error_type: ErrorType::Server,
        }
    }

    /// An error with meaningful output for the user. It may mean the user
    /// gave invalid input or that they are doing something the server doesn't
    /// support.
    pub fn user(source: impl Into<Box<dyn StdError>> + 'static) -> Self {
        Self {
            source: source.into(),
            error_type: ErrorType::User,
        }
    }

    pub fn error_type(mut self, error_type: ErrorType) -> Self {
        self.error_type = error_type;
        self
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
