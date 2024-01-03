use clap::Parser;
use rustic_server::config::server_config::ServerConfig;
use rustic_server::{acl::Acl, auth::Auth, storage::LocalStorage, web, web::State, Opts};
use std::path::PathBuf;

#[async_std::main]
async fn main() -> tide::Result<()> {
    let opts = Opts::parse();

    tide::log::with_level(opts.log);

    match opts.config {
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
            web::main(new_state, s, false, None, opts.key).await
        }
        None => {
            let storage = LocalStorage::try_new(&opts.path)?;
            let auth = Auth::from_file(opts.no_auth, &opts.path.join(".htpasswd"))?;
            let acl = Acl::from_file(opts.append_only, opts.private_repo, None)?;

            let new_state = State::new(auth, acl, storage);
            web::main(new_state, "2222".into(), false, None, opts.key).await
        }
    }
}
