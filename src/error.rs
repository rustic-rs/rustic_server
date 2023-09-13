use axum::http::StatusCode;
use std::borrow::Cow;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub type StatusResult<T> = std::result::Result<T, StatusError<'static>>;

pub struct StatusError<'a> {
    pub status: StatusCode,
    pub message: Cow<'a, str>,
}
