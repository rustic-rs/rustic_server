use axum::{middleware, Router};
use axum_extra::routing::RouterExt;
use axum_server::tls_rustls::RustlsConfig;
use tokio::net::TcpListener;
use tracing::{info, level_filters::LevelFilter};

use crate::{
    acl::init_acl,
    auth::init_auth,
    context::ServerRuntimeContext,
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
    typed_path::{RepositoryConfigPath, RepositoryPath, RepositoryTpeNamePath, RepositoryTpePath},
};

/// Start the web server
///
/// # Arguments
///
/// * `runtime_ctx` - The server runtime context
pub async fn start_web_server<S>(runtime_ctx: ServerRuntimeContext<S>) -> AppResult<()>
where
    S: Storage + Clone + std::fmt::Debug,
{
    let ServerRuntimeContext {
        socket_address,
        acl,
        auth,
        storage,
        tls,
        ..
    } = runtime_ctx;

    init_acl(acl)?;
    init_auth(auth)?;
    init_storage(storage)?;

    let mut app = Router::new();

    // /:repo/:tpe/:name
    app = app
        // Returns “200 OK” if the blob with the given name and type is stored in the repository,
        // “404 not found” otherwise. If the blob exists, the HTTP header Content-Length
        // is set to the file size.
        .typed_head(file_length::<RepositoryTpeNamePath>)
        // Returns the content of the blob with the given name and type if it is stored
        // in the repository, “404 not found” otherwise.
        // If the request specifies a partial read with a Range header field, then the
        // status code of the response is 206 instead of 200 and the response only contains
        // the specified range.
        //
        // Response format: binary/octet-stream
        .typed_get(get_file::<RepositoryTpeNamePath>)
        // Saves the content of the request body as a blob with the given name and type,
        // an HTTP error otherwise.
        //
        // Request format: binary/octet-stream
        .typed_post(add_file::<RepositoryTpeNamePath>)
        // Returns “200 OK” if the blob with the given name and type has been deleted from
        // the repository, an HTTP error otherwise.
        .typed_delete(delete_file::<RepositoryTpeNamePath>);

    // /:repo/config
    app = app
        // Returns “200 OK” if the repository has a configuration, an HTTP error otherwise.
        .typed_head(has_config)
        // Returns the content of the configuration file if the repository has a configuration,
        // an HTTP error otherwise.
        //
        // Response format: binary/octet-stream
        .typed_get(get_config::<RepositoryConfigPath>)
        // Returns “200 OK” if the configuration of the request body has been saved,
        // an HTTP error otherwise.
        .typed_post(add_config::<RepositoryConfigPath>)
        // Returns “200 OK” if the configuration of the repository has been deleted,
        // an HTTP error otherwise.
        // Note: This is not part of the API documentation, but it is implemented
        // to allow for the deletion of the configuration file during testing.
        .typed_delete(delete_config::<RepositoryConfigPath>);

    // /:repo/:tpe/
    // # API version 1
    //
    // Returns a JSON array containing the names of all the blobs stored for a given type, example:
    //
    // ```json
    // [
    //   "245bc4c430d393f74fbe7b13325e30dbde9fb0745e50caad57c446c93d20096b",
    //   "85b420239efa1132c41cea0065452a40ebc20c6f8e0b132a5b2f5848360973ec",
    //   "8e2006bb5931a520f3c7009fe278d1ebb87eb72c3ff92a50c30e90f1b8cf3e60",
    //   "e75c8c407ea31ba399ab4109f28dd18c4c68303d8d86cc275432820c42ce3649"
    // ]
    // ```
    //
    // # API version 2
    //
    // Returns a JSON array containing an object for each file of the given type.
    // The objects have two keys: name for the file name, and size for the size in bytes.
    //
    // [
    //    {
    //        "name": "245bc4c430d393f74fbe7b13325e30dbde9fb0745e50caad57c446c93d20096b",
    //        "size": 2341058
    //    },
    //    {
    //        "name": "85b420239efa1132c41cea0065452a40ebc20c6f8e0b132a5b2f5848360973ec",
    //        "size": 2908900
    //    },
    //    {
    //        "name": "8e2006bb5931a520f3c7009fe278d1ebb87eb72c3ff92a50c30e90f1b8cf3e60",
    //        "size": 3030712
    //    },
    //    {
    //        "name": "e75c8c407ea31ba399ab4109f28dd18c4c68303d8d86cc275432820c42ce3649",
    //        "size": 2804
    //    }
    // ]
    app = app.typed_get(list_files::<RepositoryTpePath>);

    // /:repo/ --> note: trailing slash
    app = app
        // This request is used to initially create a new repository.
        // The server responds with “200 OK” if the repository structure was created
        // successfully or already exists, otherwise an error is returned.
        .typed_post(create_repository::<RepositoryPath>)
        // Deletes the repository on the server side. The server responds with “200 OK”
        // if the repository was successfully removed. If this function is not implemented
        // the server returns “501 Not Implemented”, if this it is denied by the server it
        // returns “403 Forbidden”.
        .typed_delete(delete_repository::<RepositoryPath>);

    // TODO: This is not reflected in the API documentation?
    // TODO: Decide if we want to keep this or not!
    // // /:tpe/:name
    // // we loop here over explicit types, to prevent conflict with paths "/:repo/:tpe"
    // for tpe in constants::TYPES.into_iter() {
    //     let path = format!("/{}:name", &tpe);
    //     app = app
    //         .route(path.as_str(), head(file_length::<TpeNamePath>))
    //         .route(path.as_str(), get(get_file::<TpeNamePath>))
    //         .route(path.as_str(), post(add_file::<TpeNamePath>))
    //         .route(path.as_str(), delete(delete_file::<TpeNamePath>));
    // }
    //
    // /:tpe  --> note: NO trailing slash
    // we loop here over explicit types, to prevent the conflict with paths "/:repo/"
    // for tpe in constants::TYPES.into_iter() {
    //     let path = format!("/{}", &tpe);
    //     app = app.route(path.as_str(), get(list_files::<TpePath>));
    // }

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

    info!("Starting web server ...");

    if let Some(tls) = tls {
        // Start server with or without TLS
        let config = RustlsConfig::from_pem_file(tls.tls_cert, tls.tls_key)
            .await
        .map_err(|err|
            ErrorKind::Io.context(
                format!("Failed to load TLS certificate/key. Please make sure the paths are correct. `{err}`")
            )
        )?;

        info!("[serve] Listening on: `https://{socket_address}`");

        axum_server::bind_rustls(socket_address, config)
            .serve(app.into_make_service())
            .await
            .expect("Failed to start server. Is the address already in use?");
    } else {
        info!("Listening on: `http://{socket_address}`");

        axum::serve(
            TcpListener::bind(socket_address)
                .await
                .expect("Failed to bind to socket. Please make sure the address is correct."),
            app.into_make_service(),
        )
        .await
        .expect("Failed to start server. Is the address already in use?");
    };

    Ok(())
}
