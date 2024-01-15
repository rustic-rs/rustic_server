use std::fmt::{Display, Formatter};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub type Result<T> = std::result::Result<T, ErrorKind>;

#[derive(Debug)]
pub enum ErrorKind {
    InternalError(String),
    BadRequest(String),
    FilenameNotAllowed(String),
    PathNotAllowed(String),
    InvalidPath(String),
    NonUnicodePath(String),
    CreatingDirectoryFailed(String),
    NotImplemented,
    FileNotFound(String),
    GettingFileMetadataFailed,
    RangeNotValid,
    SeekingFileFailed,
    MultipartRangeNotImplemented,
    GeneralRange,
    ConversionToU64Failed,
    WritingToFileFailed,
    FinalizingFileFailed,
    GettingFileHandleFailed,
    RemovingFileFailed(String),
    ReadingFromStreamFailed,
    RemovingRepositoryFailed(String),
    AuthenticationHeaderError,
    UserAuthenticationError(String)
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
            ErrorKind::AuthenticationHeaderError => (
                StatusCode::FORBIDDEN,
                format!("Bad authentication header"),
            )
                .into_response(),
            ErrorKind::UserAuthenticationError(err) => (
                StatusCode::FORBIDDEN,
                format!("Failed to authenticate user: {:?}", err),
            )
                .into_response(),
        }
    }
}

// impl Display for ErrorKind {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         match self {
//             ErrorKind::InternalError(e) => {
//                 write!(f, "Error( code=InternalError, Message={e}")
//             }
//             ErrorKind::BadRequest(e) => {
//                 write!(f, "Error( code=BadRequest, Message={e}")
//             }
//             ErrorKind::FilenameNotAllowed(e) => {
//                 write!(f, "Error( code=FilenameNotAllowed, Message={e}")
//             }
//             ErrorKind::PathNotAllowed(e) => {
//                 write!(f, "Error( code=PathNotAllowed, Message={e}")
//             }
//             ErrorKind::InvalidPath(e) => {
//                 write!(f, "Error( code=InvalidPath, Message={e}")
//             }
//             ErrorKind::NonUnicodePath(e) => {
//                 write!(f, "Error( code=InternalError, Message={e}")
//             }
//             ErrorKind::CreatingDirectoryFailed(e) => {
//                 write!(f, "Error( code=InternalError, Message={e}")
//             }
//             ErrorKind::NotImplemented => {
//                 write!(f, "Error( code=InternalError")
//             }
//             ErrorKind::FileNotFound(e) => {
//                 write!(f, "Error( code=InternalError, Message={e}")
//             }
//             ErrorKind::GettingFileMetadataFailed => {
//                 write!(f, "Error( code=InternalError")
//             }
//             ErrorKind::RangeNotValid => {
//                 write!(f, "Error( code=InternalError")
//             }
//             ErrorKind::SeekingFileFailed => {
//                 write!(f, "Error( code=InternalError")
//             }
//             ErrorKind::MultipartRangeNotImplemented => {
//                 write!(f, "Error( code=InternalError")
//             }
//             ErrorKind::GeneralRange => {
//                 write!(f, "Error( code=InternalError")
//             }
//             ErrorKind::ConversionToU64Failed => {
//                 write!(f, "Error( code=InternalError")
//             }
//             ErrorKind::WritingToFileFailed => {
//                 write!(f, "Error( code=InternalError")
//             }
//             ErrorKind::FinalizingFileFailed => {
//                 write!(f, "Error( code=InternalError")
//             }
//             ErrorKind::GettingFileHandleFailed => {
//                 write!(f, "Error( code=InternalError")
//             }
//             ErrorKind::RemovingFileFailed(e) => {
//                 write!(f, "Error( code=InternalError, Message={e}")
//             }
//             ErrorKind::ReadingFromStreamFailed => {
//                 write!(f, "Error( code=InternalError")
//             }
//             ErrorKind::RemovingRepositoryFailed(e) => {
//                 write!(f, "Error( code=InternalError, Message={e}")
//             }
//             ErrorKind::AuthenticationHeaderError => {
//                 write!(f, "Error( code=InternalError")
//             }
//             ErrorKind::UserAuthenticationError(e) => {
//                 write!(f, "Error( code=InternalError, Message={e}")
//             }
//         }
//     }
// }
