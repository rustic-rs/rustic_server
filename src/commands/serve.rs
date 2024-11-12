//! `serve` subcommand

use std::path::PathBuf;

use abscissa_core::{
    config::Override, status_err, tracing::debug, Application, Command, FrameworkError, Runnable,
    Shutdown,
};
use anyhow::Result;
use clap::Parser;
use conflate::Merge;

use crate::{
    config::{
        AclSettings, ConnectionSettings, HtpasswdSettings, RusticServerConfig, StorageSettings,
        TlsSettings,
    },
    error::{AppResult, ErrorKind},
    prelude::RUSTIC_SERVER_APP,
    storage::LocalStorage,
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
    config: RusticServerConfig,
}

impl Override<RusticServerConfig> for ServeCmd {
    fn override_config(
        &self,
        mut config: RusticServerConfig,
    ) -> Result<RusticServerConfig, FrameworkError> {
        config.merge(self.config.clone());
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
            return Err(ErrorKind::MissingUserInput(
                "No data directory specified".to_string(),
            ));
        };

        debug!("Data directory: {:?}", data_dir);

        if !data_dir.exists() {
            debug!("Creating data directory: {:?}", data_dir);

            std::fs::create_dir_all(&data_dir).map_err(|err| {
                ErrorKind::GeneralStorageError(format!("Could not create data directory: {}", err))
            })?;
        }

        let storage = LocalStorage::try_new(&data_dir).map_err(|err| {
            ErrorKind::GeneralStorageError(format!("Could not create storage: {}", err))
        })?;

        debug!("Successfully created storage: {:?}", storage);

        // let auth_config = server_config.authorization;
        // let no_auth = !auth_config.use_auth;
        // let path = match auth_config.auth_path {
        //     None => PathBuf::new(),
        //     Some(p) => {
        //         if root.is_empty() {
        //             PathBuf::from(p)
        //         } else {
        //             assert!(!p.starts_with('/'));
        //             PathBuf::from(root.clone()).join(p)
        //         }
        //     }
        // };
        // let auth = Auth::from_file(no_auth, &path).map_err(|err| {
        //     WebErrorKind::InternalError(format!("Could not read file: {} at {:?}", err, path))
        // })?;

        // // Access control to the repositories
        // //-----------------------------------
        // let acl_config = server_config.access_control;
        // let path = acl_config.acl_path.map(|p| {
        //     if root.is_empty() {
        //         PathBuf::from(p)
        //     } else {
        //         assert!(!p.starts_with('/'));
        //         PathBuf::from(root.clone()).join(p)
        //     }
        // });
        // let acl = Acl::from_file(acl_config.append_only, acl_config.private_repo, path)?;

        // // Server definition
        // //-----------------------------------
        // let s_addr = server_config.server;
        // let s_str = format!("{}:{}", s_addr.host_dns_name, s_addr.port);
        // tracing::info!("[serve] Listening on: {}", &s_str);
        // let socket = s_str.to_socket_addrs().unwrap().next().unwrap();
        // start_web_server(acl, auth, storage, socket, false, None, self.key).await?;

        // without config
        // let storage = LocalStorage::try_new(&self.path).map_err(|err| {
        //     WebErrorKind::GeneralStorageError(format!("Could not create storage: {}", err))
        // })?;

        // let auth = Auth::from_file(self.no_auth, &self.path.join(".htpasswd")).map_err(|err| {
        //     WebErrorKind::InternalError(format!(
        //         "Could not read auth file: {} at {:?}",
        //         err, self.path
        //     ))
        // })?;
        // let acl = Acl::from_file(self.append_only, self.private_repo, self.acl)?;

        // start_web_server(
        //     acl,
        //     auth,
        //     storage,
        //     SocketAddr::from_str(&self.listen).unwrap(),
        //     false,
        //     None,
        //     self.key,
        // )
        // .await?;

        Ok(())
    }
}
