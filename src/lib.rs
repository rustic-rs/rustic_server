use clap::Parser;
use std::path::PathBuf;

pub mod acl;
pub mod auth;
pub mod error;
pub mod log;
pub mod storage;
pub mod web;
pub mod config;
pub mod handlers;

#[cfg(test)]
pub mod test_server;

/// A REST server build in rust for use with restic
#[derive(Parser)]
#[command(name = "rustic-server")]
#[command(bin_name = "rustic-server")]
pub struct Opts {
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
