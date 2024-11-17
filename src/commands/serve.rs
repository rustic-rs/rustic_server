//! `serve` subcommand

use abscissa_core::{
    config::Override,
    status_err,
    tracing::{debug, info},
    Application, Command, FrameworkError, Runnable, Shutdown,
};
use anyhow::Result;
use clap::Parser;
use conflate::Merge;

use crate::{
    config::RusticServerConfig, context::ServerRuntimeContext, error::AppResult,
    prelude::RUSTIC_SERVER_APP, storage::LocalStorage, web::start_web_server,
};

/// `serve` subcommand
///
/// The `Parser` proc macro generates an option parser based on the struct
/// definition, and is defined in the `clap` crate. See their documentation
/// for a more comprehensive example:
///
/// <https://docs.rs/clap/>
#[derive(Command, Debug, Parser)]
pub struct ServeCmd {
    /// Server settings
    #[clap(flatten)]
    context: RusticServerConfig,
}

impl Override<RusticServerConfig> for ServeCmd {
    fn override_config(
        &self,
        mut config: RusticServerConfig,
    ) -> Result<RusticServerConfig, FrameworkError> {
        debug!(?config, "ServerConfig before merge.");
        debug!(?self.context, "Command context from CLI.");

        // Merge the command-line context into the config
        // This will override the config with the command-line values,
        // if they are present, because the command-line values have
        // precedence.
        config.merge(self.context.clone());

        Ok(config)
    }
}

impl Runnable for ServeCmd {
    /// Start the application.
    fn run(&self) {
        if let Err(tokio_err) = abscissa_tokio::run(&RUSTIC_SERVER_APP, async {
            if let Err(err) = self.inner_run().await {
                status_err!("{}", err);
                RUSTIC_SERVER_APP.shutdown(Shutdown::Crash);
            }
        }) {
            status_err!("{}", tokio_err);
            RUSTIC_SERVER_APP.shutdown(Shutdown::Crash);
        };
    }
}

impl ServeCmd {
    pub async fn inner_run(&self) -> AppResult<()> {
        let server_config = RUSTIC_SERVER_APP.config();

        debug!(?server_config, "Loaded ServerConfig.");

        let runtime_ctx: ServerRuntimeContext<LocalStorage> =
            ServerRuntimeContext::from_config(server_config.clone())?;

        _ = tokio::spawn(async move {
            // If we're running in test mode, we want to shutdown after
            // 10 seconds automatically, if the environment variable
            // `CI=1` is set.
            if std::env::var("CI").is_ok() {
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                info!("Shutting down gracefully ...");
                RUSTIC_SERVER_APP.shutdown(Shutdown::Graceful);
            }

            tokio::signal::ctrl_c().await.unwrap();
            info!("Shutting down gracefully ...");
            RUSTIC_SERVER_APP.shutdown(Shutdown::Graceful);
        });

        start_web_server(runtime_ctx).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_serve() {
        ServeCmd::command().debug_assert();
    }
}
