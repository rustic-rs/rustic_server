// mod web
//
// implements a REST server as specified by
// https://restic.readthedocs.io/en/stable/REST_backend.html?highlight=Rest%20API
//
// uses the modules
// storage - to access the file system
// auth    - for user authentication
// acl     - for access control


use std::net::SocketAddr;
use axum::routing::{get, head, post, delete};
use axum::{Router};
use axum_server::tls_rustls::RustlsConfig;
use tokio::net::TcpListener;

use crate::{
    error::{Result},
    storage::Storage,
};
use crate::acl::{Acl, init_acl};
use crate::auth::{Auth, init_auth};
use crate::handlers::file_config::{add_config, delete_config, get_config, has_config};
use crate::handlers::file_exchange::{add_file, delete_file, get_file};
use crate::handlers::file_length::file_length;
use crate::handlers::files_list::list_files;
use crate::handlers::path_analysis::TYPES;
use crate::handlers::repository::{create_repository, delete_repository};
use crate::storage::{init_storage};

/// FIXME: Routes are checked in order of adding them to the Router (right?)
pub async fn web_browser(
    acl:Acl,
    auth:Auth,
    storage: impl Storage,
    socket_address: SocketAddr,
    tls: bool,
    cert: Option<String>,
    key: Option<String>,
) -> Result<()> {

    init_acl(acl)?;
    init_auth(auth)?;
    init_storage(storage)?;

    // Create routing structure
    let mut app = Router::new()
        .route(
            "/:path/config",
            head(has_config)
                .post(add_config)
                .get(get_config)
                .delete(delete_config)

        );

    // Loop over types. Rationale:
    // We can not distinguish these 2 paths using :tpe in the router:
    // Like: "/:path/:tpe/:name
    // So we loop over the types, and create a route for each type.
    // The handlers will then analyse the path to determine (path, type, name/config)
    for tpe in TYPES.into_iter() {
        let path1 = format!("/:path/{}/", &tpe);
        let path2 = format!("/:path/{}/:name", &tpe);
        app = app
            .route(
                path1.as_str(),
                get(list_files)
            )
            .route(
                path2.as_str(),
                head(file_length)
                    .get(get_file)
                    .post(add_file)
                    .delete(delete_file)
            );
    }

    // FIXME: original restic interface does not have a delete repo (right?); rustic_server did ...
    // Creating the repositories is done by sending a path
    // This path can be anything for the router except the
    // paths defined in the previous routes, added above.
    app = app
        .route(
            "/:path/",
            post(create_repository)
                .delete(delete_repository)
                .post(create_repository)
        );

    match tls {
        false => {
            println!("rustic_server listening on {}", &socket_address);
            axum::serve(
                TcpListener::bind(socket_address).await.unwrap(),
                app.into_make_service(),
            )
                .await.unwrap();
        },
        true => {
            assert!(cert.is_some());
            assert!(key.is_some());
            let config = RustlsConfig::from_pem_file(
                cert.unwrap(),
                key.unwrap(),
            )
                .await
                .unwrap();

            println!("rustic_server listening on {}", &socket_address);
            axum_server::bind_rustls(socket_address, config)
                .serve(app.into_make_service())
                .await.unwrap();
        }
    }
    Ok(())
}
