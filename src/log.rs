use once_cell::sync::OnceCell;
use std::sync::Mutex;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

//FIXME: The MUTEX is only here for the test environment. --> move to test code
// , and execute without mutex here

//When starting a server we fetch the mutex to force serial testing
pub(crate) static TRACER: OnceCell<Mutex<usize>> = OnceCell::new();
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
