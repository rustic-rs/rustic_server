use std::net::SocketAddr;

use axum::{
    middleware,
    routing::{get, head, post},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use tokio::net::TcpListener;
use tracing::level_filters::LevelFilter;

use crate::{
    acl::{init_acl, Acl},
    auth::{init_auth, Auth},
    error::Result,
    handlers::{
        file_config::{add_config, delete_config, get_config, has_config},
        file_exchange::{add_file, delete_file, get_file},
        file_length::file_length,
        files_list::list_files,
        path_analysis::constants::TYPES,
        repository::{create_repository, delete_repository},
    },
    log::print_request_response,
    storage::{init_storage, Storage},
};

pub async fn start_web_server(
    acl: Acl,
    auth: Auth,
    storage: impl Storage,
    socket_address: SocketAddr,
    tls: bool,
    cert: Option<String>,
    key: Option<String>,
) -> Result<()> {
    init_acl(acl)?;
    init_auth(auth)?;
    init_storage(storage)?;

    // -------------------------------------
    // Create routing structure
    // -------------------------------------
    let mut app = Router::new().route(
        "/:path/config",
        head(has_config)
            .post(add_config)
            .get(get_config)
            .delete(delete_config),
    );

    app = app.route("/:path/:tpe", get(list_files)).route(
        "/:path/:tpe/:name",
        head(file_length)
            .get(get_file)
            .post(add_file)
            .delete(delete_file),
    );

    app = app.route("/:path/", post(create_repository).delete(delete_repository));

    // -----------------------------------------------
    // Extra logging requested. Handlers will log too
    // ----------------------------------------------
    let level_filter = LevelFilter::current();
    match level_filter {
        LevelFilter::TRACE | LevelFilter::DEBUG | LevelFilter::INFO => {
            app = app.layer(middleware::from_fn(print_request_response));
        }
        _ => {}
    };

    // -----------------------------------------------
    // Start server with or without TLS
    // -----------------------------------------------
    match tls {
        false => {
            println!("rustic_server listening on {}", &socket_address);
            axum::serve(
                TcpListener::bind(socket_address).await.unwrap(),
                app.into_make_service(),
            )
            .await
            .unwrap();
        }
        true => {
            assert!(cert.is_some());
            assert!(key.is_some());
            let config = RustlsConfig::from_pem_file(cert.unwrap(), key.unwrap())
                .await
                .unwrap();

            println!("rustic_server listening on {}", &socket_address);
            axum_server::bind_rustls(socket_address, config)
                .serve(app.into_make_service())
                .await
                .unwrap();
        }
    }
    Ok(())
}
