use std::{
    fs::create_dir_all,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

use abscissa_core::prelude::{debug, info};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{
    acl::Acl,
    auth::Auth,
    config::{
        default_data_dir, default_socket_address, AclSettings, HtpasswdSettings, LogSettings,
        RusticServerConfig, TlsSettings,
    },
    error::{AppResult, ErrorKind},
    storage::Storage,
};

#[derive(Clone, Serialize, Deserialize, Default, Debug)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct TlsOptions {
    /// Optional path to the TLS key file
    pub tls_key: PathBuf,

    /// Optional path to the TLS certificate file
    pub tls_cert: PathBuf,
}

#[derive(Clone, Debug)]
pub struct ServerRuntimeContext<S>
where
    S: Storage + Clone + std::fmt::Debug,
{
    pub(crate) acl: Acl,
    pub(crate) auth: Auth,
    pub(crate) _quota: usize,
    pub(crate) socket_address: SocketAddr,
    pub(crate) storage: S,
    pub(crate) tls: Option<TlsOptions>,
}

impl<S> ServerRuntimeContext<S>
where
    S: Storage + Clone + std::fmt::Debug,
{
    pub fn from_config(config: Arc<RusticServerConfig>) -> AppResult<Self> {
        let storage_dir = Self::data_dir(
            config
                .storage
                .data_dir
                .clone()
                .unwrap_or_else(default_data_dir),
        )?;

        let socket_address =
            Self::socket_address(config.server.listen.unwrap_or_else(default_socket_address))?;

        let quota = Self::quota(config.storage.quota);

        let acl = Self::acl(config.acl.clone(), storage_dir.clone())?;

        let auth = Self::auth(config.auth.clone(), storage_dir.clone())?;

        let tls = Self::tls(config.tls.clone())?;

        let storage = Self::storage(storage_dir)?;

        Ok(Self {
            acl,
            auth,
            _quota: quota,
            socket_address,
            storage,
            tls,
        })
    }

    fn quota(quota: Option<usize>) -> usize {
        quota.unwrap_or(0)
    }

    fn storage(data_dir: PathBuf) -> AppResult<S> {
        let storage = S::init(&data_dir).map_err(|err| {
            ErrorKind::GeneralStorageError.context(format!("Could not create storage: {}", err))
        })?;

        debug!(?storage, "Loaded Storage.");

        Ok(storage)
    }

    fn tls(tls_settings: TlsSettings) -> AppResult<Option<TlsOptions>> {
        // TODO: Do we need to validate the TLS settings?
        let tls = if tls_settings.is_disabled() {
            info!("TLS is disabled.");
            None
        } else {
            let (Some(tls_key), Some(tls_cert)) = (tls_settings.tls_key, tls_settings.tls_cert)
            else {
                return Err(ErrorKind::GeneralStorageError
                    .context("TLS is enabled but no key or certificate was provided.")
                    .into());
            };
            info!("TLS is enabled.");

            Some(TlsOptions { tls_key, tls_cert })
        };

        debug!(?tls, "Loaded TLS settings.");

        Ok(tls)
    }

    #[allow(clippy::cognitive_complexity)]
    fn auth(htpasswd_settings: HtpasswdSettings, data_dir: PathBuf) -> AppResult<Auth> {
        let auth = if htpasswd_settings.is_disabled() {
            info!("Authentication is disabled.");
            warn!("This allows anyone to push to your repositories. This should be considered insecure and is not recommended for production use.");
            Auth::default()
        } else {
            info!(
                "Authentication is enabled by default. If you want to disable it, add `--no-auth`."
            );

            let valid_htpasswd_path = htpasswd_settings.htpasswd_file_or_default(data_dir)?;

            Auth::from_config(&htpasswd_settings, valid_htpasswd_path).map_err(|err| {
                ErrorKind::GeneralStorageError
                    .context(format!("Could not create authentication due to `{err}`",))
            })?
        };

        debug!(?auth, "Loaded Auth.");

        Ok(auth)
    }

    fn data_dir(data_dir: impl Into<PathBuf>) -> AppResult<PathBuf> {
        let data_dir = data_dir.into();

        if !data_dir.exists() {
            debug!("Creating data directory: `{:?}`", data_dir);

            create_dir_all(&data_dir).map_err(|err| {
                ErrorKind::GeneralStorageError
                    .context(format!("Could not create data directory: `{}`", err))
            })?;
        }

        info!(
            "Using directory for storing repositories: `{}`",
            data_dir.display()
        );

        Ok(data_dir)
    }

    fn socket_address(address: SocketAddr) -> AppResult<SocketAddr> {
        debug!(?address, "Parsed socket address.");

        Ok(address)
    }

    fn acl(acl_settings: AclSettings, data_dir: PathBuf) -> AppResult<Acl> {
        let acl = if acl_settings.is_disabled() {
            info!("ACL is disabled.");
            Acl::default().set_append_only(acl_settings.append_only)
        } else {
            info!("ACL is enabled.");

            let valid_acl_path = acl_settings.acl_file_or_default(data_dir)?;

            Acl::from_config(&acl_settings, Some(valid_acl_path)).map_err(|err| {
                ErrorKind::GeneralStorageError
                    .context(format!("Could not create ACL due to `{err}`"))
            })?
        };

        debug!(?acl, "Loaded Access Control List.");

        Ok(acl)
    }

    fn _log(log_settings: LogSettings) -> AppResult<LogSettings> {
        let log = if log_settings.is_disabled() {
            info!("Logging is set to default.");
            LogSettings::default()
        } else {
            log_settings
        };

        debug!(?log, "Loaded LogSettings.");

        Ok(log)
    }

    pub fn storage_path(&self) -> &Path {
        self.storage.path()
    }
}
