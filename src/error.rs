//! Error types

use abscissa_core::error::{BoxError, Context};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::{
    fmt::{self, Display},
    io,
    ops::Deref,
    result::Result,
};

pub type AppResult<T> = Result<T, Error>;
pub type ApiResult<T> = Result<T, ApiErrorKind>;

/// Kinds of errors
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq, Copy)]
pub enum ErrorKind {
    /// Error in configuration file
    #[error("config error")]
    Config,

    /// Input/output error
    #[error("I/O error")]
    Io,

    /// General storage error
    #[error("storage error")]
    GeneralStorageError,

    /// Missing user input
    #[error("missing user input")]
    MissingUserInput,
}

#[derive(Debug, thiserror::Error, displaydoc::Display)]
pub enum ApiErrorKind {
    /// Internal server error: `{0}`
    InternalError(String),
    /// Bad request: `{0}`
    BadRequest(String),
    /// Filename `{0}` not allowed
    FilenameNotAllowed(String),
    /// Path `{0}` is ambiguous with internal types and not allowed
    AmbiguousPath(String),
    /// Path `{0}` not allowed
    PathNotAllowed(String),
    /// Path `{0}` is not valid
    InvalidPath(String),
    /// Path `{0}` is not valid unicode
    NonUnicodePath(String),
    /// Creating directory failed: `{0}`
    CreatingDirectoryFailed(String),
    /// Not yet implemented
    NotImplemented,
    /// File not found: `{0}`
    FileNotFound(String),
    /// Getting file metadata failed: `{0}`
    GettingFileMetadataFailed(String),
    /// Range not valid
    RangeNotValid,
    /// Seeking file failed
    SeekingFileFailed,
    /// Multipart range not implemented
    MultipartRangeNotImplemented,
    /// General range error
    GeneralRange,
    /// Conversion from length to u64 failed
    ConversionToU64Failed,
    /// Opening file failed: `{0}`
    OpeningFileFailed(String),
    /// Writing file failed: `{0}`
    WritingToFileFailed(String),
    /// Finalizing file failed: `{0}`
    FinalizingFileFailed(String),
    /// Getting file handle failed
    GettingFileHandleFailed,
    /// Removing file failed: `{0}`
    RemovingFileFailed(String),
    /// Reading from stream failed
    ReadingFromStreamFailed,
    /// Removing repository folder failed: `{0}`
    RemovingRepositoryFailed(String),
    /// Bad authentication header
    AuthenticationHeaderError,
    /// Failed to authenticate user: `{0}`
    UserAuthenticationError(String),
    /// General Storage error: `{0}`
    GeneralStorageError(String),
    /// Invalid API version: `{0}`
    InvalidApiVersion(String),
}

impl IntoResponse for ApiErrorKind {
    fn into_response(self) -> Response {
        let response = match self {
            Self::InvalidApiVersion(err) => (
                StatusCode::BAD_REQUEST,
                format!("Invalid API version: {err}"),
            ),
            Self::InternalError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal server error: {}", err),
            ),
            Self::BadRequest(err) => (
                StatusCode::BAD_REQUEST,
                format!("Internal server error: {}", err),
            ),
            Self::FilenameNotAllowed(filename) => (
                StatusCode::FORBIDDEN,
                format!("filename {filename} not allowed"),
            ),
            Self::AmbiguousPath(path) => (
                StatusCode::FORBIDDEN,
                format!("path {path} is ambiguous with internal types and not allowed"),
            ),
            Self::PathNotAllowed(path) => {
                (StatusCode::FORBIDDEN, format!("path {path} not allowed"))
            }
            Self::NonUnicodePath(path) => (
                StatusCode::BAD_REQUEST,
                format!("path {path} is not valid unicode"),
            ),
            Self::InvalidPath(path) => {
                (StatusCode::BAD_REQUEST, format!("path {path} is not valid"))
            }
            Self::CreatingDirectoryFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error creating dir: {:?}", err),
            ),
            Self::NotImplemented => (
                StatusCode::NOT_IMPLEMENTED,
                "not yet implemented".to_string(),
            ),
            Self::FileNotFound(path) => (StatusCode::NOT_FOUND, format!("file not found: {path}")),
            Self::GettingFileMetadataFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error getting file metadata: {err}"),
            ),
            Self::RangeNotValid => (StatusCode::BAD_REQUEST, "range not valid".to_string()),
            Self::SeekingFileFailed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error seeking file".to_string(),
            ),
            Self::MultipartRangeNotImplemented => (
                StatusCode::NOT_IMPLEMENTED,
                "multipart range not implemented".to_string(),
            ),
            Self::ConversionToU64Failed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error converting length to u64".to_string(),
            ),
            Self::OpeningFileFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error opening file: {err}"),
            ),
            Self::WritingToFileFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error writing file: {err}"),
            ),
            Self::FinalizingFileFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error finalizing file: {err}"),
            ),
            Self::GettingFileHandleFailed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error getting file handle".to_string(),
            ),
            Self::RemovingFileFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error removing file: {err}"),
            ),
            Self::GeneralRange => (StatusCode::INTERNAL_SERVER_ERROR, "range error".to_string()),
            Self::ReadingFromStreamFailed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error reading from stream".to_string(),
            ),
            Self::RemovingRepositoryFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error removing repository folder: {:?}", err),
            ),
            Self::AuthenticationHeaderError => (
                StatusCode::FORBIDDEN,
                "Bad authentication header".to_string(),
            ),
            Self::UserAuthenticationError(err) => (
                StatusCode::FORBIDDEN,
                format!("Failed to authenticate user: {:?}", err),
            ),
            Self::GeneralStorageError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Storage error: {:?}", err),
            ),
        };

        response.into_response()
    }
}

impl ErrorKind {
    /// Create an error context from this error
    pub fn context(self, source: impl Into<BoxError>) -> Context<Self> {
        Context::new(self, Some(source.into()))
    }
}

/// Error type
#[derive(Debug)]
pub struct Error(Box<Context<ErrorKind>>);

impl Deref for Error {
    type Target = Context<ErrorKind>;

    fn deref(&self) -> &Context<ErrorKind> {
        &self.0
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Context::new(kind, None).into()
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(context: Context<ErrorKind>) -> Self {
        Self(Box::new(context))
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        ErrorKind::Io.context(err).into()
    }
}
