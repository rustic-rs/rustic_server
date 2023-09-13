use tracing_subscriber::Registry;

pub fn init_tracing() -> Registry {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "RUSTIC_SERVER_LOG_LEVEL=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init()
}
