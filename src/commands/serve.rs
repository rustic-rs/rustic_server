use crate::acl::Acl;
use crate::auth::Auth;
use crate::config::server_config::ServerConfig;
use crate::error::{ErrorKind, Result};
use crate::log::init_tracing;
use crate::storage::LocalStorage;
use crate::web::start_web_server;
use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;

// FIXME: we should not return crate::error::Result here; maybe anyhow::Result,

pub async fn serve(opts: Opts) -> Result<()> {
    init_tracing();

    match &opts.config {
        Some(config) => {
            let config_path = PathBuf::new().join(&config);
            let server_config = ServerConfig::from_file(&config_path)
                .unwrap_or_else(|_| panic!("Can not load server configuration file: {}", &config));

            // Repository storage
            let storage_path = PathBuf::new().join(server_config.repos.storage_path);
            let storage = match LocalStorage::try_new(&storage_path) {
                Ok(s) => s,
                Err(e) => return Err(ErrorKind::InternalError(e.to_string())),
            };

            // Authorization user/password
            let auth_config = server_config.authorization;
            let no_auth = !auth_config.use_auth;
            let path = match auth_config.auth_path {
                None => PathBuf::new(),
                Some(p) => PathBuf::new().join(p),
            };
            let auth = match Auth::from_file(no_auth, &path) {
                Ok(s) => s,
                Err(e) => return Err(ErrorKind::InternalError(e.to_string())),
            };

            // Access control to the repositories
            let acl_config = server_config.accesscontrol;
            let path = acl_config.acl_path.map(|p| PathBuf::new().join(p));
            let acl = match Acl::from_file(acl_config.append_only, acl_config.private_repo, path) {
                Ok(s) => s,
                Err(e) => return Err(ErrorKind::InternalError(e.to_string())),
            };

            // Server definition
            let socket = SocketAddr::from_str(&opts.listen).unwrap();
            start_web_server(acl, auth, storage, socket, false, None, opts.key).await
        }
        None => {
            let storage = match LocalStorage::try_new(&opts.path) {
                Ok(s) => s,
                Err(e) => return Err(ErrorKind::InternalError(e.to_string())),
            };
            let auth = match Auth::from_file(opts.no_auth, &opts.path.join(".htpasswd")) {
                Ok(s) => s,
                Err(e) => return Err(ErrorKind::InternalError(e.to_string())),
            };
            let acl = match Acl::from_file(opts.append_only, opts.private_repo, None) {
                Ok(s) => s,
                Err(e) => return Err(ErrorKind::InternalError(e.to_string())),
            };

            start_web_server(
                acl,
                auth,
                storage,
                SocketAddr::from_str(&opts.listen).unwrap(),
                false,
                None,
                opts.key,
            )
            .await
        }
    }
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
