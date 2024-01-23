use crate::acl::Acl;
use crate::auth::Auth;
use crate::config::server_config::ServerConfig;
use crate::log::{init_trace_from, init_tracing};
use crate::storage::LocalStorage;
use crate::web::start_web_server;
use clap::Parser;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::str::FromStr;

use crate::error::{ErrorKind, Result};

pub async fn serve(opts: Opts) -> Result<()> {
    match &opts.config {
        Some(config) => {
            let config_path = PathBuf::new().join(config);
            let server_config =
                ServerConfig::from_file(&config_path).unwrap_or_else(|e| panic!("{}", e));

            if let Some(level) = server_config.log_level {
                init_trace_from(&level);
            } else {
                init_tracing();
            }

            // Repository storage
            let storage_path = PathBuf::new().join(server_config.repos.storage_path);
            let storage = LocalStorage::try_new(&storage_path).map_err(|err| {
                ErrorKind::GeneralStorageError(format!("Could not create storage: {}", err))
            })?;

            // Authorization user/password
            let auth_config = server_config.authorization;
            let no_auth = !auth_config.use_auth;
            let path = match auth_config.auth_path {
                None => PathBuf::new(),
                Some(p) => PathBuf::new().join(p),
            };
            let auth = Auth::from_file(no_auth, &path)
                .map_err(|err| ErrorKind::InternalError(format!("Could not read file: {}", err)))?;

            // Access control to the repositories
            let acl_config = server_config.accesscontrol;
            let path = acl_config.acl_path.map(|p| PathBuf::new().join(p));
            let acl = Acl::from_file(acl_config.append_only, acl_config.private_repo, path)?;

            // Server definition
            let s_addr = server_config.server;
            let s_str = format!("{}:{}", s_addr.host_dns_name, s_addr.port);
            tracing::debug!("[serve] Serving address: {}", &s_str);
            let socket = s_str.to_socket_addrs().unwrap().next().unwrap();
            start_web_server(acl, auth, storage, socket, false, None, opts.key).await?;
        }
        None => {
            init_trace_from(&opts.log);

            let storage = LocalStorage::try_new(&opts.path).map_err(|err| {
                ErrorKind::GeneralStorageError(format!("Could not create storage: {}", err))
            })?;

            let auth = Auth::from_file(opts.no_auth, &opts.path.join(".htpasswd"))
                .map_err(|err| ErrorKind::InternalError(format!("Could not read file: {}", err)))?;
            let acl = Acl::from_file(opts.append_only, opts.private_repo, None)?;

            start_web_server(
                acl,
                auth,
                storage,
                SocketAddr::from_str(&opts.listen).unwrap(),
                false,
                None,
                opts.key,
            )
            .await?;
        }
    }

    Ok(())
}

/// A REST server build in rust for use with restic
#[derive(Parser)]
#[command(name = "rustic-server")]
#[command(bin_name = "rustic-server")]
pub struct Opts {
    /// Server configuration file
    #[arg(short, long)]
    pub config: Option<String>,
    /// listen address
    #[arg(short, long, default_value = "localhost:8000")]
    pub listen: String,
    /// data directory
    #[arg(short, long, default_value = "/tmp/restic")]
    pub path: PathBuf,
    /// disable .htpasswd authentication
    #[arg(long)]
    pub no_auth: bool,
    /// file to read per-repo ACLs from
    #[arg(long)]
    pub acl: Option<PathBuf>,
    /// set standard acl to append only mode
    #[arg(long)]
    pub append_only: bool,
    /// set standard acl to only access private repos
    #[arg(long)]
    pub private_repo: bool,
    /// turn on TLS support
    #[arg(long)]
    pub tls: bool,
    /// TLS certificate path
    #[arg(long)]
    pub cert: Option<String>,
    /// TLS key path
    #[arg(long)]
    pub key: Option<String>,
    /// logging level (Off/Error/Warn/Info/Debug/Trace)
    #[arg(long, default_value = "Info")]
    pub log: String,
}
