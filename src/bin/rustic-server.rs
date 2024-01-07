use axum::extract::State;
use clap::Parser;
use anyhow::Result;
use rustic_server::{acl::Acl, auth::Auth, storage::LocalStorage, web, Opts};

// #[async_std::main]
// async fn main() -> tide::Result<()> {
//     let opts = Opts::parse();
//
//     tide::log::with_level(opts.log);
//
//     let storage = LocalStorage::try_new(&opts.path)?;
//     let auth = Auth::from_file(opts.no_auth, &opts.path.join(".htpasswd"))?;
//     let acl = Acl::from_file(opts.append_only, opts.private_repo, opts.acl)?;
//
//     web::main(acl, auth, storage, opts.listen, opts.tls, opts.cert, opts.key).await
// }

fn main() -> Result<()> {
    println!("main: implement me");
    Ok(())
}