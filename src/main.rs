use clap::Parser;
use std::path::PathBuf;

pub mod acl;
pub mod auth;
pub mod helpers;
pub mod storage;
mod web;

/// A REST server build in rust for use with restic
#[derive(Parser)]
#[command(name = "rustic-server")]
#[command(bin_name = "rustic-server")]
struct Opts {
    /// listen adress
    #[arg(short, long, default_value = "localhost:8000")]
    listen: String,
    /// data directory
    #[arg(short, long, default_value = "/tmp/restic")]
    path: PathBuf,
    /// disable .htpasswd authentication
    #[arg(long)]
    no_auth: bool,
    /// file to read per-repo ACLs from
    #[arg(long)]
    acl: Option<PathBuf>,
    /// set standard acl to append only mode
    #[arg(long)]
    append_only: bool,
    /// set standard acl to only access private repos
    #[arg(long)]
    private_repo: bool,
    /// turn on TLS support
    #[arg(long)]
    tls: bool,
    /// TLS certificate path
    #[arg(long)]
    cert: Option<String>,
    /// TLS key path
    #[arg(long)]
    key: Option<String>,
    /// logging level (Off/Error/Warn/Info/Debug/Trace)
    #[arg(long, default_value = "Info")]
    log: tide::log::LevelFilter,
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    let opts = Opts::parse();

    tide::log::with_level(opts.log);

    let storage = storage::LocalStorage::try_new(&opts.path)?;
    let auth = auth::Auth::from_file(opts.no_auth, &opts.path.join(".htpasswd"))?;
    let acl = acl::Acl::from_file(opts.append_only, opts.private_repo, opts.acl)?;

    let new_state = web::State::new(auth, acl, storage);
    web::main(new_state, opts.listen, opts.tls, opts.cert, opts.key).await
}
