use std::net::SocketAddr;

use axum::routing::{delete, get, head, post};
use axum::{middleware, Router};
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
    log_opts: &LogSettings,
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
        .route("/:repo/config", head(has_config))
        .route("/:repo/config", post(add_config::<RepositoryConfigPath>))
        .route("/:repo/config", get(get_config::<RepositoryConfigPath>))
        .route(
            "/:repo/config",
            delete(delete_config::<RepositoryConfigPath>),
        );

    // /:tpe  --> note: NO trailing slash
    // we loop here over explicit types, to prevent the conflict with paths "/:repo/"
    for tpe in TYPES.into_iter() {
        let path = format!("/{}", &tpe);
        app = app.route(path.as_str(), get(list_files::<TpePath>));
    }

    // /:repo/ --> note: trailing slash
    app = app
        .route("/:repo/", post(create_repository::<RepositoryPath>))
        .route("/:repo/", delete(delete_repository::<RepositoryPath>));

    // /:tpe/:name
    // we loop here over explicit types, to prevent conflict with paths "/:repo/:tpe"
    for tpe in TYPES.into_iter() {
        let path = format!("/{}:name", &tpe);
        app = app
            .route(path.as_str(), head(file_length::<TpeNamePath>))
            .route(path.as_str(), get(get_file::<TpeNamePath>))
            .route(path.as_str(), post(add_file::<TpeNamePath>))
            .route(path.as_str(), delete(delete_file::<TpeNamePath>));
    }

    // /:repo/:tpe
    app = app.route("/:repo/:tpe", get(list_files::<RepositoryTpePath>));

    // /:repo/:tpe/:name
    app = app
        .route(
            "/:repo/:tpe/:name",
            head(file_length::<RepositoryTpeNamePath>),
        )
        .route("/:repo/:tpe/:name", get(get_file::<RepositoryTpeNamePath>))
        .route("/:repo/:tpe/:name", post(add_file::<RepositoryTpeNamePath>))
        .route(
            "/:repo/:tpe/:name",
            delete(delete_file::<RepositoryTpeNamePath>),
        );

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
