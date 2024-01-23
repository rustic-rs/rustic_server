use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use displaydoc::Display;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, ErrorKind>;

#[derive(Debug, Error, Display)]
pub enum ErrorKind {
    /// Internal server error: {0}
    InternalError(String),
    /// Bad request: {0}
    BadRequest(String),
    /// Filename {0} not allowed
    FilenameNotAllowed(String),
    /// Path {0} not allowed
    PathNotAllowed(String),
    /// Path {0} is not valid
    InvalidPath(String),
    /// Path {0} is not valid unicode
    NonUnicodePath(String),
    /// Creating directory failed: {0}
    CreatingDirectoryFailed(String),
    /// Not yet implemented
    NotImplemented,
    /// File not found: {0}
    FileNotFound(String),
    /// Fetting file metadata failed
    GettingFileMetadataFailed,
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
    /// Writing file failed
    WritingToFileFailed,
    /// Finalizing file failed
    FinalizingFileFailed,
    /// Getting file handle failed
    GettingFileHandleFailed,
    /// Removing file failed: {0}
    RemovingFileFailed(String),
    /// Reading from stream failed
    ReadingFromStreamFailed,
    /// Removing repository folder failed: {0}
    RemovingRepositoryFailed(String),
    /// Bad authentication header
    AuthenticationHeaderError,
    /// Failed to authenticate user: {0}
    UserAuthenticationError(String),
    /// General Storage error: {0}
    GeneralStorageError(String),
}

impl IntoResponse for ErrorKind {
    fn into_response(self) -> Response {
        match self {
            ErrorKind::InternalError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal server error: {}", err),
            )
                .into_response(),
            ErrorKind::BadRequest(err) => (
                StatusCode::BAD_REQUEST,
                format!("Internal server error: {}", err),
            )
                .into_response(),
            ErrorKind::FilenameNotAllowed(filename) => (
                StatusCode::FORBIDDEN,
                format!("filename {filename} not allowed"),
            )
                .into_response(),
            ErrorKind::PathNotAllowed(path) => {
                (StatusCode::FORBIDDEN, format!("path {path} not allowed")).into_response()
            }
            ErrorKind::NonUnicodePath(path) => (
                StatusCode::BAD_REQUEST,
                format!("path {path} is not valid unicode"),
            )
                .into_response(),
            ErrorKind::InvalidPath(_) => todo!(),
            ErrorKind::CreatingDirectoryFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error creating dir: {:?}", err),
            )
                .into_response(),
            ErrorKind::NotImplemented => (
                StatusCode::NOT_IMPLEMENTED,
                "not yet implemented".to_string(),
            )
                .into_response(),
            ErrorKind::FileNotFound(path) => {
                (StatusCode::NOT_FOUND, format!("file not found: {path}")).into_response()
            }
            ErrorKind::GettingFileMetadataFailed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error getting file metadata".to_string(),
            )
                .into_response(),
            ErrorKind::RangeNotValid => {
                (StatusCode::BAD_REQUEST, "range not valid".to_string()).into_response()
            }
            ErrorKind::SeekingFileFailed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error seeking file".to_string(),
            )
                .into_response(),
            ErrorKind::MultipartRangeNotImplemented => (
                StatusCode::NOT_IMPLEMENTED,
                "multipart range not implemented".to_string(),
            )
                .into_response(),
            ErrorKind::ConversionToU64Failed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error converting length to u64".to_string(),
            )
                .into_response(),
            ErrorKind::WritingToFileFailed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error writing file".to_string(),
            )
                .into_response(),
            ErrorKind::FinalizingFileFailed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error finalizing file".to_string(),
            )
                .into_response(),
            ErrorKind::GettingFileHandleFailed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error getting file handle".to_string(),
            )
                .into_response(),
            ErrorKind::RemovingFileFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error removing file: {:?}", err),
            )
                .into_response(),
            ErrorKind::GeneralRange => {
                (StatusCode::INTERNAL_SERVER_ERROR, "range error".to_string()).into_response()
            }
            ErrorKind::ReadingFromStreamFailed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error reading from stream".to_string(),
            )
                .into_response(),
            ErrorKind::RemovingRepositoryFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error removing repository folder: {:?}", err),
            )
                .into_response(),
            ErrorKind::AuthenticationHeaderError => {
                (StatusCode::FORBIDDEN, "Bad authentication header").into_response()
            }
            ErrorKind::UserAuthenticationError(err) => (
                StatusCode::FORBIDDEN,
                format!("Failed to authenticate user: {:?}", err),
            )
                .into_response(),
            ErrorKind::GeneralStorageError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Storage error: {:?}", err),
            )
                .into_response(),
        }
    }
}
