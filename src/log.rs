use axum::{
    body::{Body, Bytes},
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_macros::debug_middleware;
use http_body_util::BodyExt;

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
    let uuid = uuid::Uuid::new_v4();

    tracing::debug!(
        id = %uuid,
        method = %parts.method,
        uri = %parts.uri,
        "[REQUEST]",
    );

    tracing::debug!(id = %uuid, headers = ?parts.headers, "[HEADERS]");

    let bytes = buffer_and_print(&uuid, body).await?;

    let req = Request::from_parts(parts, Body::from(bytes));

    let res = next.run(req).await;
    let (parts, body) = res.into_parts();

    tracing::debug!(
        id = %uuid,
        headers = ?parts.headers,
        status = %parts.status,
        "[RESPONSE]",
    );

    let bytes = buffer_and_print(&uuid, body).await?;
    let res = Response::from_parts(parts, Body::from(bytes));

    Ok(res)
}

async fn buffer_and_print<B>(uuid: &uuid::Uuid, body: B) -> Result<Bytes, ApiErrorKind>
where
    B: axum::body::HttpBody<Data = Bytes> + Send,
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
        tracing::debug!(id = %uuid, body = %body, "[BODY]");
    }

    Ok(bytes)
}
