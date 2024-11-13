use axum::{
    body::{Body, Bytes},
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use http_body_util::BodyExt;

use crate::error::ApiErrorKind;

/// router middleware function to print additional information on the request, and response.
/// Usage:
///       app = Router::new().layer(middleware::from_fn(print_request_response))
///
pub async fn print_request_response(
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, ApiErrorKind> {
    let (parts, body) = req.into_parts();

    tracing::debug!("request-method: {}", parts.method);

    for (k, v) in parts.headers.iter() {
        tracing::debug!("request-header: {k:?} -> {v:?} ");
    }
    tracing::debug!("request-uri: {}", parts.uri);
    let bytes = buffer_and_print("request", body).await?;
    let req = Request::from_parts(parts, Body::from(bytes));

    let res = next.run(req).await;

    let (parts, body) = res.into_parts();
    for (k, v) in parts.headers.iter() {
        tracing::debug!("reply-header: {k:?} -> {v:?} ");
    }
    let bytes = buffer_and_print("response", body).await?;
    let res = Response::from_parts(parts, Body::from(bytes));

    tracing::debug!("response-status: {}", res.status());

    Ok(res)
}

async fn buffer_and_print<B>(direction: &str, body: B) -> Result<Bytes, ApiErrorKind>
where
    B: axum::body::HttpBody<Data = Bytes>,
    B::Error: std::fmt::Display,
{
    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) => {
            return Err(ApiErrorKind::BadRequest(format!(
                "failed to read {direction} body: {err}"
            )));
        }
    };

    if let Ok(body) = std::str::from_utf8(&bytes) {
        tracing::debug!("{direction} body = {body:?}");
    }

    Ok(bytes)
}
