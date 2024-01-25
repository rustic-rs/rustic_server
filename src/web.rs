use std::net::SocketAddr;

use axum::{middleware, Router};
use axum_extra::routing::{
    RouterExt, // for `Router::typed_*`
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
        repository::{create_repository, delete_repository},
    },
    log::print_request_response,
    storage::{init_storage, Storage},
    typed_path::{RepositoryTpeNamePath, RepositoryTpePath, TpeNamePath, TpePath},
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
    let mut app = Router::new();

    // /:repo/config
    app = app
        .typed_head(has_config)
        .typed_post(add_config)
        .typed_get(get_config)
        .typed_delete(delete_config);

    // /:repo/
    app = app
        .typed_post(create_repository)
        .typed_delete(delete_repository);

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
