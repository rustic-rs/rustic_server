use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber::filter::LevelFilter;

pub mod acl;
pub mod auth;
pub mod helpers;
pub mod storage;
pub mod web;

/// A REST server build in rust for use with restic
#[derive(Parser)]
#[command(name = "rustic-server")]
#[command(bin_name = "rustic-server")]
pub struct Opts {
    /// listen address
    #[arg(short, long, default_value = "localhost")]
    pub host: String,
    /// listen port https
    #[arg(short, long, default_value = 8000)]
    pub https_port: u16,
    /// listen port http
    #[arg(short, long, default_value = 8080)]
    pub http_port: u16,
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
    pub log: LevelFilter,
}
