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
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
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
    /// Fetting file metadata failed: `{0}`
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
}

impl IntoResponse for ApiErrorKind {
    fn into_response(self) -> Response {
        let response = match self {
            ApiErrorKind::InternalError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal server error: {}", err),
            ),
            ApiErrorKind::BadRequest(err) => (
                StatusCode::BAD_REQUEST,
                format!("Internal server error: {}", err),
            ),
            ApiErrorKind::FilenameNotAllowed(filename) => (
                StatusCode::FORBIDDEN,
                format!("filename {filename} not allowed"),
            ),
            ApiErrorKind::AmbiguousPath(path) => (
                StatusCode::FORBIDDEN,
                format!("path {path} is ambiguous with internal types and not allowed"),
            ),
            ApiErrorKind::PathNotAllowed(path) => {
                (StatusCode::FORBIDDEN, format!("path {path} not allowed"))
            }
            ApiErrorKind::NonUnicodePath(path) => (
                StatusCode::BAD_REQUEST,
                format!("path {path} is not valid unicode"),
            ),
            ApiErrorKind::InvalidPath(path) => {
                (StatusCode::BAD_REQUEST, format!("path {path} is not valid"))
            }
            ApiErrorKind::CreatingDirectoryFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error creating dir: {:?}", err),
            ),
            ApiErrorKind::NotImplemented => (
                StatusCode::NOT_IMPLEMENTED,
                "not yet implemented".to_string(),
            ),
            ApiErrorKind::FileNotFound(path) => {
                (StatusCode::NOT_FOUND, format!("file not found: {path}"))
            }
            ApiErrorKind::GettingFileMetadataFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error getting file metadata: {err}"),
            ),
            ApiErrorKind::RangeNotValid => (StatusCode::BAD_REQUEST, "range not valid".to_string()),
            ApiErrorKind::SeekingFileFailed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error seeking file".to_string(),
            ),
            ApiErrorKind::MultipartRangeNotImplemented => (
                StatusCode::NOT_IMPLEMENTED,
                "multipart range not implemented".to_string(),
            ),
            ApiErrorKind::ConversionToU64Failed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error converting length to u64".to_string(),
            ),
            ApiErrorKind::OpeningFileFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error opening file: {err}"),
            ),
            ApiErrorKind::WritingToFileFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error writing file: {err}"),
            ),
            ApiErrorKind::FinalizingFileFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error finalizing file: {err}"),
            ),
            ApiErrorKind::GettingFileHandleFailed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error getting file handle".to_string(),
            ),
            ApiErrorKind::RemovingFileFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error removing file: {err}"),
            ),
            ApiErrorKind::GeneralRange => {
                (StatusCode::INTERNAL_SERVER_ERROR, "range error".to_string())
            }
            ApiErrorKind::ReadingFromStreamFailed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error reading from stream".to_string(),
            ),
            ApiErrorKind::RemovingRepositoryFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error removing repository folder: {:?}", err),
            ),
            ApiErrorKind::AuthenticationHeaderError => (
                StatusCode::FORBIDDEN,
                "Bad authentication header".to_string(),
            ),
            ApiErrorKind::UserAuthenticationError(err) => (
                StatusCode::FORBIDDEN,
                format!("Failed to authenticate user: {:?}", err),
            ),
            ApiErrorKind::GeneralStorageError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Storage error: {:?}", err),
            ),
        };

        response.into_response()
    }
}

impl ErrorKind {
    /// Create an error context from this error
    pub fn context(self, source: impl Into<BoxError>) -> Context<ErrorKind> {
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
        Error(Box::new(context))
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        ErrorKind::Io.context(err).into()
    }
}
