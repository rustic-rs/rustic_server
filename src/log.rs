use axum::{
    body::{Body, Bytes},
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_macros::debug_middleware;
use http_body_util::BodyExt;
use tracing::Instrument;

use crate::error::ApiErrorKind;

/// Router middleware function to print additional information on the request and response.
///
/// # Usage
///
/// Add this middleware to the router to print the request and response information.
///
/// ```rust
/// use axum::Router;
/// 
/// app = Router::new()
///         .layer(middleware::from_fn(print_request_response))
/// ```
#[debug_middleware]
pub async fn print_request_response(
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, ApiErrorKind> {
    let (parts, body) = req.into_parts();

    let span = tracing::span!(
        tracing::Level::DEBUG,
        "request",
        method = %parts.method,
        uri = %parts.uri,
    );

    let _enter = span.enter();

    tracing::debug!(headers = ?parts.headers, "[new request]");

    let bytes = buffer_and_print(body).instrument(span.clone()).await?;

    let req = Request::from_parts(parts, Body::from(bytes));

    let res = next.run(req).instrument(span.clone()).await;
    let (parts, body) = res.into_parts();

    let span = tracing::span!(
        tracing::Level::DEBUG,
        "response",
        headers = ?parts.headers,
        status = %parts.status,
    );

    let _enter = span.enter();

    let bytes = buffer_and_print(body).instrument(span.clone()).await?;
    let res = Response::from_parts(parts, Body::from(bytes));

    Ok(res)
}

async fn buffer_and_print<B>(body: B) -> Result<Bytes, ApiErrorKind>
where
    B: axum::body::HttpBody<Data = Bytes>,
    B::Error: std::fmt::Display,
{
    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) => {
            return Err(ApiErrorKind::BadRequest(format!(
                "failed to read body: {err}"
            )));
        }
    };

    if let Ok(body) = std::str::from_utf8(&bytes) {
        tracing::debug!(body = %body);
    }

    Ok(bytes)
}
