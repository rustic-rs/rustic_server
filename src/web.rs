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

// FIXME: decide either to keep, or change. Then remove the remarks below
// During the rout table creation, we loop over types. Rationale:
// We can not distinguish paths using `:tpe` matching in the router.
// The routing path would then become "/:path/:tpe/:name
// This seems not supported by the Serde parser (I assume that is what is used under the hood)
//
// So, instead, we loop over the types, and create a route path for each "explicit" type.
// The handlers will then analyse the path to determine (path, type, name/config).

// An alternative design might be that we create helper functions like so:
//   - get_file_data()  --> calls get_file( ..., "data")
//   - get_file_config() --> calls get_file( ..., "config")
//   - get_file_keys()  --> calls get_file( ..., "keys")
//      etc, etc,
// When adding these to the router, we can use the Axum::Path to get the path without having
// to re-analyse the URI like we do now. TBI: does this speed up the server?

/// FIXME: original Restic interface seems not to provide a "delete repository" interface.
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

    // Fixme: Are we faster by creating a "function" per type and skip analysing the path in each call?
    for tpe in TYPES.into_iter() {
        let path1 = format!("/:path/{}/", &tpe);
        let path2 = format!("/:path/{}/:name", &tpe);
        app = app.route(path1.as_str(), get(list_files)).route(
            path2.as_str(),
            head(file_length)
                .get(get_file)
                .post(add_file)
                .delete(delete_file),
        );
    }

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
