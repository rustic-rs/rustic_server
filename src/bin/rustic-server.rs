use std::net::SocketAddr;
use std::str::FromStr;
use clap::Parser;
use anyhow::Result;
use rustic_server::{acl::Acl, auth::Auth, storage::LocalStorage, web, Opts};
use rustic_server::log::init_tracing;

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();

    init_tracing();

    let storage = LocalStorage::try_new(&opts.path)?;
    let auth = Auth::from_file(opts.no_auth, &opts.path.join(".htpasswd"))?;
    let acl = Acl::from_file(opts.append_only, opts.private_repo, opts.acl)?;

    let sa = SocketAddr::from_str(&opts.listen)?;
    web::web_browser(acl, auth, storage, sa, opts.tls, opts.cert, opts.key).await.unwrap();
    Ok(())
}
