use axum::{extract::Request, middleware::Next, response::Response};

// Add the `#[debug_middleware]` attribute to the function to make debugging easier.
// use axum_macros::debug_middleware;
//
// #[debug_middleware]
/// Router middleware function to print additional information on the request and response.
///
/// # Note
///
/// Request and response bodies are deliberately *not* logged. They carry pack,
/// index and key files, which are encrypted blobs of arbitrary size, so logging
/// them would mean holding a whole file in memory for every request in flight
/// only to find out it is not printable text.
pub async fn print_request_response(req: Request, next: Next) -> Response {
    let uuid = uuid::Uuid::new_v4();

    tracing::debug!(
        id = %uuid,
        method = %req.method(),
        uri = %req.uri(),
        headers = ?req.headers(),
        "[REQUEST]",
    );

    let res = next.run(req).await;

    tracing::debug!(
        id = %uuid,
        status = %res.status(),
        headers = ?res.headers(),
        "[RESPONSE]",
    );

    res
}
