use std::{sync::OnceLock, time::Instant};

use axum::{http::StatusCode, response::IntoResponse};
use axum_extra::json;

use crate::auth::BasicAuthFromRequest;

// Global that stores the current when the server started
// This is used to check if the server is running
pub static START_TIME: OnceLock<Instant> = OnceLock::new();

pub fn init_start_time() {
    let _ = START_TIME.get_or_init(Instant::now);
}

pub async fn live_check() -> impl IntoResponse {
    let start = START_TIME.get().expect("start time not initialized");
    let uptime = Instant::now().duration_since(*start);

    (
        StatusCode::OK,
        json!({
            "status": "ok",
            "version": env!("CARGO_PKG_VERSION"),
            "uptime": uptime.as_secs(),
            "timestamp": chrono::Local::now().timestamp(),
        }),
    )
        .into_response()
}

// /health/ready
//
// Example response as an idea of what to return:
//
// ```json
// {
//   "status": "ready",
//   "version": "1.2.3",
//   "uptime": 123456,
//   "error_count": 0,
//   "timestamp": "2024-11-16T12:34:56Z",
//   "last_backup_status": "success",
//   "last_backup_timestamp": "2024-11-15T23:59:59Z",
//   "backup_queue_length": 0,
//   "backup_size_last": "10GB",
//   "dependencies": {
//     "storage_status": "ok",
//     "available_disk_space": "120GB"
//   },
//   "performance": {
//     "cpu_usage": "15%",
//     "memory_usage": "300MB",
//     "active_tasks": 2,
//   }
// }
// ```
// TODO: Implement ready_check
#[allow(dead_code)]
pub async fn ready_check(_auth: BasicAuthFromRequest) -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}
