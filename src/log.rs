use std::sync::{Mutex};
use once_cell::sync::OnceCell;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

//When starting a server we fetch the mutex to force serial testing
pub(crate) static TRACER:OnceCell<Mutex<usize>> = OnceCell::new();
pub(crate) fn init_mutex() {
    TRACER.get_or_init(|| {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "RUSTIC_SERVER_LOG_LEVEL=debug".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
        Mutex::new(0)
    });
}

pub fn init_tracing() {
    init_mutex();
}


