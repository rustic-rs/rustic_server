use std::str::FromStr;

use axum::{
    body::{Body, Bytes},
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use http_body_util::BodyExt;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::error::ErrorKind;

pub fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "RUSTIC_SERVER_LOG_LEVEL=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

pub fn init_trace_from(level: &str) {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_str(level).unwrap())
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// router middleware function to print additional information on the request, and response.
/// Usage:
///       app = Router::new().layer(middleware::from_fn(print_request_response))
///
pub async fn print_request_response(
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, ErrorKind> {
    let (parts, body) = req.into_parts();
    for (k, v) in parts.headers.iter() {
        tracing::debug!("request-header: {k:?} -> {v:?} ");
    }
    let bytes = buffer_and_print("request", body).await?;
    let req = Request::from_parts(parts, Body::from(bytes));

    let res = next.run(req).await;

    let (parts, body) = res.into_parts();
    for (k, v) in parts.headers.iter() {
        tracing::debug!("reply-header: {k:?} -> {v:?} ");
    }
    let bytes = buffer_and_print("response", body).await?;
    let res = Response::from_parts(parts, Body::from(bytes));

    Ok(res)
}

async fn buffer_and_print<B>(direction: &str, body: B) -> Result<Bytes, ErrorKind>
where
    B: axum::body::HttpBody<Data = Bytes>,
    B::Error: std::fmt::Display,
{
    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) => {
            return Err(ErrorKind::BadRequest(format!(
                "failed to read {direction} body: {err}"
            )));
        }
    };

    if let Ok(body) = std::str::from_utf8(&bytes) {
        tracing::debug!("{direction} body = {body:?}");
    }

    Ok(bytes)
}
