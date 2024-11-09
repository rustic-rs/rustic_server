use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub type Result<T> = std::result::Result<T, ErrorKind>;

#[derive(Debug, thiserror::Error, displaydoc::Display)]
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
    /// Fetting file metadata failed: {0}
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
    /// Opening file failed: {0}
    OpeningFileFailed(String),
    /// Writing file failed: {0}
    WritingToFileFailed(String),
    /// Finalizing file failed: {0}
    FinalizingFileFailed(String),
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
        let response = match self {
            ErrorKind::InternalError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal server error: {}", err),
            ),
            ErrorKind::BadRequest(err) => (
                StatusCode::BAD_REQUEST,
                format!("Internal server error: {}", err),
            ),
            ErrorKind::FilenameNotAllowed(filename) => (
                StatusCode::FORBIDDEN,
                format!("filename {filename} not allowed"),
            ),
            ErrorKind::PathNotAllowed(path) => {
                (StatusCode::FORBIDDEN, format!("path {path} not allowed"))
            }
            ErrorKind::NonUnicodePath(path) => (
                StatusCode::BAD_REQUEST,
                format!("path {path} is not valid unicode"),
            ),
            ErrorKind::InvalidPath(path) => {
                (StatusCode::BAD_REQUEST, format!("path {path} is not valid"))
            }
            ErrorKind::CreatingDirectoryFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error creating dir: {:?}", err),
            ),
            ErrorKind::NotImplemented => (
                StatusCode::NOT_IMPLEMENTED,
                "not yet implemented".to_string(),
            ),
            ErrorKind::FileNotFound(path) => {
                (StatusCode::NOT_FOUND, format!("file not found: {path}"))
            }
            ErrorKind::GettingFileMetadataFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error getting file metadata: {err}"),
            ),
            ErrorKind::RangeNotValid => (StatusCode::BAD_REQUEST, "range not valid".to_string()),
            ErrorKind::SeekingFileFailed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error seeking file".to_string(),
            ),
            ErrorKind::MultipartRangeNotImplemented => (
                StatusCode::NOT_IMPLEMENTED,
                "multipart range not implemented".to_string(),
            ),
            ErrorKind::ConversionToU64Failed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error converting length to u64".to_string(),
            ),
            ErrorKind::OpeningFileFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error opening file: {err}"),
            ),
            ErrorKind::WritingToFileFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error writing file: {err}"),
            ),
            ErrorKind::FinalizingFileFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error finalizing file: {err}"),
            ),
            ErrorKind::GettingFileHandleFailed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error getting file handle".to_string(),
            ),
            ErrorKind::RemovingFileFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error removing file: {err}"),
            ),
            ErrorKind::GeneralRange => {
                (StatusCode::INTERNAL_SERVER_ERROR, "range error".to_string())
            }
            ErrorKind::ReadingFromStreamFailed => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error reading from stream".to_string(),
            ),
            ErrorKind::RemovingRepositoryFailed(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("error removing repository folder: {:?}", err),
            ),
            ErrorKind::AuthenticationHeaderError => (
                StatusCode::FORBIDDEN,
                "Bad authentication header".to_string(),
            ),
            ErrorKind::UserAuthenticationError(err) => (
                StatusCode::FORBIDDEN,
                format!("Failed to authenticate user: {:?}", err),
            ),
            ErrorKind::GeneralStorageError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Storage error: {:?}", err),
            ),
        };

        response.into_response()
    }
}
