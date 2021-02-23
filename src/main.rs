use clap::Clap;
use std::path::PathBuf;

pub mod acl;
pub mod auth;
pub mod helpers;
pub mod storage;
mod web;

/// A REST server build in rust for use with restic
#[derive(Clap)]
struct Opts {
    /// listen adress
    #[clap(short, long, default_value = "localhost:8000")]
    listen: String,
    /// data directory
    #[clap(short, long, default_value = "/tmp/restic")]
    path: PathBuf,
    /// disable .htpasswd authentication
    #[clap(long)]
    no_auth: bool,
    /// file to read per-repo ACLs from
    #[clap(long)]
    acl: Option<PathBuf>,
    /// set standard acl to append only mode
    #[clap(long)]
    append_only: bool,
    /// set standard acl to only access private repos
    #[clap(long)]
    private_repo: bool,
    /// turn on TLS support
    #[clap(long)]
    tls: bool,
    /// TLS certificate path
    #[clap(long)]
    cert: Option<String>,
    /// TLS key path
    #[clap(long)]
    key: Option<String>,
    /// logging level (Off/Error/Warn/Info/Debug/Trace)
    #[clap(long, default_value = "Info")]
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
