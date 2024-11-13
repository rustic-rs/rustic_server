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
    acl::Acl,
    auth::Auth,
    config::RusticServerConfig,
    error::{AppResult, ErrorKind},
    prelude::RUSTIC_SERVER_APP,
    storage::LocalStorage,
    web::start_web_server,
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

        debug!("Successfully loaded configuration: {:?}", server_config);

        let Some(data_dir) = &server_config.storage.data_dir else {
            return Err(ErrorKind::MissingUserInput
                .context("No data directory specified".to_string())
                .into());
        };

        debug!("Data directory: {:?}", data_dir);

        if !data_dir.exists() {
            debug!("Creating data directory: {:?}", data_dir);

            std::fs::create_dir_all(&data_dir).map_err(|err| {
                ErrorKind::GeneralStorageError
                    .context(format!("Could not create data directory: {}", err))
            })?;
        }

        let storage = LocalStorage::try_new(&data_dir).map_err(|err| {
            ErrorKind::GeneralStorageError.context(format!("Could not create storage: {}", err))
        })?;

        debug!("Successfully created storage: {:?}", storage);

        let auth = Auth::from_config(&server_config.auth).map_err(|err| {
            ErrorKind::GeneralStorageError
                .context(format!("Could not create `htpasswd` due to {err}",))
        })?;

        debug!("Successfully created auth: {:?}", auth);

        let acl = Acl::from_config(&server_config.acl).map_err(|err| {
            ErrorKind::GeneralStorageError.context(format!("Could not create ACL due to {err}"))
        })?;

        debug!("Successfully created acl: {:?}", acl);

        let socket = server_config.server.listen.parse().map_err(|err| {
            ErrorKind::GeneralStorageError
                .context(format!("Could not create socket address: {err}"))
        })?;

        info!("[serve] Starting web server ...");

        let _ = tokio::spawn(async move {
            tokio::signal::ctrl_c().await.unwrap();
            info!("[serve] Shutting down ...");
            RUSTIC_SERVER_APP.shutdown(Shutdown::Graceful);
        });

        start_web_server(
            acl,
            auth,
            storage,
            socket,
            &server_config.tls,
            &server_config.log,
        )
        .await?;

        Ok(())
    }
}
