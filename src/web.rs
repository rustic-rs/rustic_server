use std::net::SocketAddr;

use axum::{middleware, Router};
use axum_extra::routing::RouterExt;
use axum_server::tls_rustls::RustlsConfig;
use tokio::net::TcpListener;
use tracing::{info, level_filters::LevelFilter};

use crate::{
    acl::{init_acl, Acl},
    auth::{init_auth, Auth},
    config::LogSettings,
    error::{AppResult, ErrorKind},
    handlers::{
        file_config::{add_config, delete_config, get_config, has_config},
        file_exchange::{add_file, delete_file, get_file},
        file_length::file_length,
        files_list::list_files,
        repository::{create_repository, delete_repository},
    },
    log::print_request_response,
    storage::{init_storage, Storage},
    typed_path::{RepositoryTpeNamePath, RepositoryTpePath, TpeNamePath, TpePath},
};
use crate::{
    config::TlsSettings,
    typed_path::{RepositoryConfigPath, RepositoryPath},
};

// TPE_LOCKS is defined, but outside the types[] array.
// This allows us to loop over the types[] when generating "routes"
pub(crate) const TPE_DATA: &str = "data";
pub(crate) const TPE_KEYS: &str = "keys";
pub(crate) const TPE_LOCKS: &str = "locks";
pub(crate) const TPE_SNAPSHOTS: &str = "snapshots";
pub(crate) const TPE_INDEX: &str = "index";
pub(crate) const _TPE_CONFIG: &str = "config";
pub(crate) const TYPES: [&str; 5] = [TPE_DATA, TPE_KEYS, TPE_LOCKS, TPE_SNAPSHOTS, TPE_INDEX];

/// Start the web server
///
/// # Arguments
///
/// * `acl` - The ACL configuration
/// * `auth` - The Auth configuration
/// * `storage` - The Storage configuration
/// * `socket_address` - The socket address to bind to
/// * `tls` - Enable TLS
/// * `cert` - The certificate file
/// * `key` - The key file
pub async fn start_web_server(
    acl: Acl,
    auth: Auth,
    storage: impl Storage,
    socket_address: SocketAddr,
    tls_opts: &TlsSettings,
    _log_opts: &LogSettings,
) -> AppResult<()> {
    init_acl(acl)?;
    init_auth(auth)?;
    init_storage(storage)?;

    // -------------------------------------
    // Create routing structure
    // -------------------------------------
    let mut app = Router::new();

    // /:repo/config
    app = app
        .typed_head(has_config)
        .typed_post(add_config::<RepositoryConfigPath>)
        .typed_get(get_config::<RepositoryConfigPath>)
        .typed_delete(delete_config::<RepositoryConfigPath>);

    // /:repo/
    app = app
        .typed_post(create_repository::<RepositoryPath>)
        .typed_delete(delete_repository::<RepositoryPath>);

    // /:tpe
    app = app.typed_get(list_files::<TpePath>);

    // /:tpe/:name
    app = app
        .typed_head(file_length::<TpeNamePath>)
        .typed_get(get_file::<TpeNamePath>)
        .typed_post(add_file::<TpeNamePath>)
        .typed_delete(delete_file::<TpeNamePath>);

    // /:repo/:tpe
    app = app.typed_get(list_files::<RepositoryTpePath>);

    // /:repo/:tpe/:name
    app = app
        .typed_head(file_length::<RepositoryTpeNamePath>)
        .typed_get(get_file::<RepositoryTpeNamePath>)
        .typed_post(add_file::<RepositoryTpeNamePath>)
        .typed_delete(delete_file::<RepositoryTpeNamePath>);

    // Extra logging requested. Handlers will log too
    // TODO: Use LogSettings here, this should be set from the cli by `--log`
    // TODO: and then needs to go to a file
    // e.g. log_opts.is_disabled() or other checks
    match LevelFilter::current() {
        LevelFilter::TRACE | LevelFilter::DEBUG | LevelFilter::INFO => {
            app = app.layer(middleware::from_fn(print_request_response));
        }
        _ => {}
    };

    let TlsSettings {
        tls,
        tls_cert,
        tls_key,
    } = tls_opts;

    // Start server with or without TLS
    if !tls {
        info!("[serve] Listening on: http://{}", socket_address);

        axum::serve(
            TcpListener::bind(socket_address)
                .await
                .expect("Failed to bind to socket. Please make sure the address is correct."),
            app.into_make_service(),
        )
        .await
        .expect("Failed to start server. Is the address already in use?");
    } else {
        let (Some(cert), Some(key)) = (tls_cert.as_ref(), tls_key.as_ref()) else {
            return Err(ErrorKind::MissingUserInput
                .context("TLS certificate or key not specified".to_string())
                .into());
        };

        let config = RustlsConfig::from_pem_file(cert, key)
            .await
            .expect("Failed to load TLS certificate/key. Please make sure the paths are correct.");

        info!("[serve] Listening on: https://{}", socket_address);

        axum_server::bind_rustls(socket_address, config)
            .serve(app.into_make_service())
            .await
            .expect("Failed to start server. Is the address already in use?");
    }
    Ok(())
}
