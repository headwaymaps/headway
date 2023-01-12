use std::error::Error as StdError;

#[derive(Debug, Clone, PartialEq, Eq)]
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
