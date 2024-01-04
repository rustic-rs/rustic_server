use crate::acl::Acl;
use crate::auth::Auth;
use crate::config::server_config::ServerConfig;
use crate::storage::LocalStorage;
use crate::web;
use crate::web::State;
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

pub async fn serve(opts: Opts) -> Result<()> {
    tide::log::with_level(opts.log);

    return match &opts.config {
        Some(config) => {
            let config_path = PathBuf::new().join(&config);
            let server_config = ServerConfig::from_file(&config_path).expect(&format!(
                "Can not load server configuration file: {}",
                &config
            ));

            // Repository storage
            let storage_path = PathBuf::new().join(server_config.repos.storage_path);
            let storage = LocalStorage::try_new(&storage_path)?;

            // Authorization user/password
            let auth_config = server_config.authorization;
            let no_auth = !auth_config.use_auth;
            let path = match auth_config.auth_path {
                None => PathBuf::new(),
                Some(p) => PathBuf::new().join(p),
            };
            let auth = Auth::from_file(no_auth, &path)?;

            // Access control to the repositories
            let acl_config = server_config.accesscontrol;
            let path = match acl_config.acl_path {
                None => None,
                Some(p) => Some(PathBuf::new().join(p)),
            };
            let acl = Acl::from_file(acl_config.append_only, acl_config.private_repo, path)?;

            // Server definition
            let new_state = State::new(auth, acl, storage);
            let server = server_config.server;
            let s = format!(
                "{}://{}:{}",
                server.protocol, server.host_dns_name, server.port
            );
            println!("Starting server. Listening on: {}", &s);
            match web::main(new_state, s, false, None, opts.key).await {
                Ok(_) => Ok(()),
                Err(e) => Err(e.into_inner()),
            }
        }
        None => {
            let storage = LocalStorage::try_new(&opts.path)?;
            let auth = Auth::from_file(opts.no_auth, &opts.path.join(".htpasswd"))?;
            let acl = Acl::from_file(opts.append_only, opts.private_repo, None)?;

            let new_state = State::new(auth, acl, storage);
            match web::main(new_state, opts.listen.into(), false, None, opts.key).await {
                Ok(_) => Ok(()),
                Err(e) => Err(e.into_inner()),
            }
        }
    };
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
    pub log: tide::log::LevelFilter,
}
