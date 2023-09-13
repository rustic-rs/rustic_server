// mod web
//
// implements a REST server as specified by
// https://restic.readthedocs.io/en/stable/REST_backend.html?highlight=Rest%20API
//
// uses the modules
// storage - to access the file system
// auth    - for user authentication
// acl     - for access control

use axum::{
    body::{Body, StreamBody},
    extract::{Path as PathExtract, Query, State, TypedHeader},
    handler::{Handler, HandlerWithoutStateExt},
    http::{header, Request, StatusCode},
    response::{AppendHeaders, IntoResponse, Response},
    routing::{get, head, post},
    Json, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use http_range::HttpRange;
use serde_derive::{Deserialize, Serialize};
use std::{convert::TryInto, marker::Unpin, path::Path as StdPath, sync::Arc};
use std::{net::SocketAddr, path::PathBuf};
use tokio::io::AsyncWrite;
use tokio::io::SeekFrom::Start;
use tokio::{fs::copy, io::AsyncSeekExt};
use tokio_util::io::ReaderStream;

use crate::{
    acl::{AccessType, Acl, AclChecker},
    auth::{Auth, AuthChecker},
    helpers::{Finalizer, IteratorAdapter},
    storage::{LocalStorage, Storage},
};

const API_V1: &str = "application/vnd.x.restic.rest.v1";
const API_V2: &str = "application/vnd.x.restic.rest.v2";
const TYPES: [&str; 5] = ["data", "keys", "locks", "snapshots", "index"];
const DEFAULT_PATH: &str = "";
const CONFIG_TYPE: &str = "config";
const CONFIG_NAME: &str = "";

#[derive(Clone)]
struct TpeState(pub String);

#[derive(Clone, Copy)]
pub struct Ports {
    pub http: u16,
    pub https: u16,
}

#[derive(Clone)]
pub struct AppState {
    auth: Arc<dyn AuthChecker>,
    acl: Arc<dyn AclChecker>,
    storage: Arc<dyn Storage>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            auth: Arc::new(Auth::default()),
            acl: Arc::new(Acl::default()),
            storage: Arc::new(LocalStorage::default()),
        }
    }
}

// TODO!
// #[async_trait::async_trait]
// impl tide_http_auth::Storage<String, BasicAuthRequest> for State {
//     async fn get_user(&self, request: BasicAuthRequest) -> Result<Option<String>> {
//         let user = request.username;
//         match self.auth.verify(&user, &request.password) {
//             true => Ok(Some(user)),
//             false => Ok(None),
//         }
//     }
// }

impl AppState {
    pub fn new(auth: impl AuthChecker, acl: impl AclChecker, storage: impl Storage) -> Self {
        Self {
            storage: Arc::new(storage),
            auth: Arc::new(auth),
            acl: Arc::new(acl),
        }
    }
}

fn check_string_sha256(name: &str) -> bool {
    if name.len() != 64 {
        return false;
    }
    for c in name.chars() {
        if !c.is_ascii_digit() && !('a'..='f').contains(&c) {
            return false;
        }
    }
    true
}

fn check_name(tpe: &str, name: &str) -> Result<impl IntoResponse, (StatusCode, String)> {
    match tpe {
        "config" => Ok(()),
        _ if check_string_sha256(name) => Ok(()),
        _ => Err((
            StatusCode::FORBIDDEN,
            format!("filename {} not allowed", name),
        )),
    }
}

fn check_auth_and_acl(
    state: &AppState,
    path: &StdPath,
    tpe: &str,
    append: AccessType,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // don't allow paths that includes any of the defined types
    for part in path.iter() {
        if let Some(part) = part.to_str() {
            for tpe in TYPES.iter() {
                if &part == tpe {
                    return Err((
                        StatusCode::FORBIDDEN,
                        format!("path {} not allowed", path.display()),
                    ));
                }
            }
        }
    }

    let empty = String::new();
    let user: &str = state.ext::<String>().unwrap_or(&empty);
    let Some(path) = path.to_str() else {
        return Err((
            StatusCode::FORBIDDEN,
            format!("path {} is non-unicode", path.display()),
        ));
    };
    let allowed = state.acl.allowed(user, path, tpe, append);
    tracing::debug!("[auth] user: {user}, path: {path}, tpe: {tpe}, allowed: {allowed}");

    match allowed {
        true => Ok(StatusCode::OK),
        false => Err((StatusCode::FORBIDDEN, format!("path {path} not allowed"))),
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct Create {
    create: bool,
}

async fn create_dirs(
    Query(params): Query<Create>,
    State(state): State<AppState>,
    path: Option<PathExtract<&str>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let path = if let Some(PathExtract(path_ext)) = path {
        StdPath::new(path_ext)
    } else {
        StdPath::new(DEFAULT_PATH)
    };

    tracing::debug!("[create_dirs] path: {path:?}");

    check_auth_and_acl(&state, path, "", AccessType::Append)?;
    let c: Create = params;
    match c.create {
        true => {
            for tpe in TYPES.iter() {
                match state.storage.create_dir(path, tpe) {
                    Ok(_) => (),
                    Err(e) => {
                        return Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("error creating dir: {:?}", e),
                        ))
                    }
                };
            }

            return Ok((
                StatusCode::OK,
                format!("Called create_files with path {:?}\n", path),
            ));
        }
        false => {
            return Ok((
                StatusCode::OK,
                format!("Called create_files with path {:?}, create=false\n", path),
            ))
        }
    }
}

#[derive(Serialize)]
struct RepoPathEntry {
    name: String,
    size: u64,
}

async fn list_files(
    State(tpe_state): State<TpeState>,
    State(state): State<AppState>,
    path: Option<PathExtract<&str>>,
    req: &Request<Body>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let tpe = &tpe_state.0;

    let path = if let Some(PathExtract(path_ext)) = path {
        StdPath::new(path_ext)
    } else {
        StdPath::new(DEFAULT_PATH)
    };

    tracing::debug!("[list_files] path: {path:?}, tpe: {tpe}");

    check_auth_and_acl(&state, path, tpe, AccessType::Read)?;

    let read_dir = state.storage.read_dir(path, tpe);

    // TODO: error handling
    let res = match req.headers().get("Accept") {
        Some(a)
            if match a.to_str() {
                Ok(s) => s == API_V2,
                Err(_) => false, // possibly not a String
            } =>
        {
            let read_dir_version = read_dir.map(|e| RepoPathEntry {
                name: e.file_name().to_str().unwrap().to_string(),
                size: e.metadata().unwrap().len(),
            });
            let mut response = Json(&IteratorAdapter::new(read_dir_version)).into_response();
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static(API_V2),
            );
            let status = response.status_mut();
            *status = StatusCode::OK;
            response
        }
        _ => {
            let read_dir_version = read_dir.map(|e| e.file_name().to_str().unwrap().to_string());
            let mut response = Json(&IteratorAdapter::new(read_dir_version)).into_response();
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static(API_V1),
            );
            let status = response.status_mut();
            *status = StatusCode::OK;
            response
        }
    };

    Ok(res)
}

async fn length(
    PathExtract(path): PathExtract<&str>,
    State(tpe_state): State<TpeState>,
    State(state): State<AppState>,
    PathExtract(name): PathExtract<&str>,
    req: &Request<Body>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let tpe = tpe_state.0.as_str();
    tracing::debug!("[length] path: {path}, tpe: {tpe}, name: {name}");
    let path = StdPath::new(&path);

    match check_name(tpe, name) {
        Ok(_) => (),
        Err(e) => return Err(e),
    };

    match check_auth_and_acl(&state, path, tpe, AccessType::Read) {
        Ok(_) => (),
        Err(e) => return Err(e),
    };

    let _file = state.storage.filename(path, tpe, name);
    Err((
        StatusCode::NOT_IMPLEMENTED,
        "not yet implemented".to_string(),
    ))
}

async fn get_file(
    State(tpe_state): State<TpeState>,
    State(state): State<AppState>,
    PathExtract(name): PathExtract<&str>,
    path: Option<PathExtract<&str>>,
    req: &Request<Body>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let tpe = &tpe_state.0;

    let path = if let Some(PathExtract(path_ext)) = path {
        StdPath::new(path_ext)
    } else {
        StdPath::new(DEFAULT_PATH)
    };
    tracing::debug!("[get_file] path: {path:?}, tpe: {tpe}, name: {name}");

    check_name(tpe, name)?;
    let path = StdPath::new(path);
    check_auth_and_acl(&state, path, tpe, AccessType::Read)?;

    let Ok(mut file) = state.storage.open_file(path, tpe, name).await else {
        return Err((StatusCode::NOT_FOUND, format!("file not found: {:?}", path)));
    };

    let mut len = match file.metadata().await {
        Ok(val) => val.len(),
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "error getting file metadata".to_string(),
            ))
        }
    };

    let status;
    match req.headers().get("Range") {
        None => {
            status = StatusCode::OK;
        }
        Some(header_value) => match HttpRange::parse(
            match header_value.to_str() {
                Ok(val) => val,
                Err(_) => return Err((StatusCode::BAD_REQUEST, "range not valid".to_string())),
            },
            len,
        ) {
            Ok(range) if range.len() == 1 => {
                let Ok(_) = file.seek(Start(range[0].start)).await else {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "error seeking file".to_string(),
                    ));
                };

                len = range[0].length;
                status = StatusCode::PARTIAL_CONTENT;
            }
            Ok(_) => {
                return Err((
                    StatusCode::NOT_IMPLEMENTED,
                    "multipart range not implemented".to_string(),
                ))
            }
            Err(_) => return Err((StatusCode::INTERNAL_SERVER_ERROR, "range error".to_string())),
        },
    };

    // From: https://github.com/tokio-rs/axum/discussions/608#discussioncomment-1789020
    let stream = ReaderStream::with_capacity(
        file,
        match len.try_into() {
            Ok(val) => val,
            Err(_) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "error converting length to u64".to_string(),
                ))
            }
        },
    );
    let body = StreamBody::new(stream);

    let headers = AppendHeaders([(header::CONTENT_TYPE, "application/octet-stream")]);
    Ok((status, headers, body))
}

async fn save_body(
    mut file: impl AsyncWrite + Unpin + Finalizer,
    req: &Request<Body>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let bytes_written = match copy(req, &mut file).await {
        Ok(val) => val,
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "error writing file".to_string(),
            ))
        }
    };
    tracing::debug!("[file written] bytes: {bytes_written}");
    let Ok(_) = file.finalize().await else {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "error finalizing file".to_string(),
        ));
    };
    Ok(StatusCode::OK)
}

async fn get_save_file(
    path: Option<PathExtract<&str>>,
    State(tpe_state): State<TpeState>,
    State(state): State<AppState>,
    PathExtract(name): PathExtract<&str>,
) -> Result<impl AsyncWrite + Unpin + Finalizer, (StatusCode, String)> {
    let tpe = tpe_state.0.as_str();
    let path = if let Some(PathExtract(path_ext)) = path {
        StdPath::new(path_ext)
    } else {
        StdPath::new(DEFAULT_PATH)
    };

    tracing::debug!("[get_save_file] path: {path:?}, tpe: {tpe}, name: {name}");

    let Ok(_) = check_name(tpe, name) else {
        return Err((
            StatusCode::FORBIDDEN,
            format!("filename {} not allowed", name),
        ));
    };

    let Ok(_) = check_auth_and_acl(&state, path, tpe, AccessType::Append) else {
        return Err((
            StatusCode::FORBIDDEN,
            format!("path {} not allowed", path.display()),
        ));
    };

    match state.storage.create_file(path, tpe, name).await {
        Ok(val) => Ok(val),
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "error getting file handle".to_string(),
            ))
        }
    }
}

async fn delete_file(
    path: Option<PathExtract<&str>>,
    State(tpe_state): State<TpeState>,
    State(state): State<AppState>,
    PathExtract(name): PathExtract<&str>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let tpe = tpe_state.0.as_str();
    let path = if let Some(PathExtract(path_ext)) = path {
        StdPath::new(path_ext)
    } else {
        StdPath::new(DEFAULT_PATH)
    };

    let Ok(_) = check_name(tpe, name) else {
        return Err((
            StatusCode::FORBIDDEN,
            format!("filename {} not allowed", name),
        ));
    };

    let Ok(_) = check_auth_and_acl(&state, path, tpe, AccessType::Modify) else {
        return Err((
            StatusCode::FORBIDDEN,
            format!("path {} not allowed", path.display()),
        ));
    };

    let Ok(_) = state.storage.remove_file(path, tpe, name) else {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "error removing file".to_string(),
        ));
    };

    Ok(StatusCode::OK)
}

// async fn auth_handler(AuthBasic((id, password)): AuthBasic) -> Result<String> {
//     tracing::debug!("[auth_handler] id: {id}, password: {password}");
//     match id.as_str() {
//         "user" if password == "password" => Ok(id),
//         _ => Err(axum::Error::from_str(StatusCode::Forbidden, "not allowed")),
//     }
// }

// TODO!: https://github.com/tokio-rs/axum/blob/main/examples/tls-rustls/src/main.rs
// TODO!: https://github.com/tokio-rs/axum/blob/main/examples/readme/src/main.rs
pub async fn main(
    state: AppState,
    addr: String,
    ports: Ports,
    tls: bool,
    cert: Option<String>,
    key: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    // TODO!
    // let mid = tide_http_auth::Authentication::new(BasicAuthScheme);

    // app.with(mid);

    let mut app = Router::new().with_state(state);

    app.route("/", post(create_dirs));
    app.route("/:path/", post(create_dirs));

    for tpe in TYPES.into_iter() {
        let path = &("/".to_string() + tpe + "/");
        tracing::debug!("add path: {path}");
        let tpe_state = Arc::new(TpeState(tpe.into()));

        app.route(path, get(list_files));

        let path = &("/".to_string() + tpe + "/:name");
        tracing::debug!("add path: {path}");
        app.route(
            path,
            head(length)
                .get(get_file)
                .post(
                    // TODO!: middle layer candidate
                    // let file = get_save_file(DEFAULT_PATH, tpe, req.param("name")?, &req).await?;
                    // save_body(&mut req, file).await
                )
                .delete( delete_file),
        );

        let path = &("/:path/".to_string() + tpe + "/");
        tracing::debug!("add path: {path}");
        app.route(path, get(list_files));

        let path = &("/:path/".to_string() + tpe + "/:name");
        tracing::debug!("add path: {path}");
        app.route(
            path,
            head(length)
            .get(get_file)
            .post(
                // TODO!: middle layer candidate
                // let file = get_save_file(req.param("path")?, tpe, req.param("name")?, &req).await?;
                // save_body(&mut req, file).await
            )
            .delete(delete_file),
        );
    }

    app.route(
        "config",
        get(get_file)
        .post(
            // TODO!: middle layer candidate
            // let file = get_save_file(DEFAULT_PATH, CONFIG_TYPE, CONFIG_NAME, &req).await?;
            // save_body(&mut req, file).await
        )
        .delete(
           delete_file
        ),
    );

    app.route(
        "/:path/config",
        get(get_file)
        .post(
            // TODO!: middle layer candidate
            // let file = get_save_file(req.param("path")?, CONFIG_TYPE, CONFIG_NAME, &req).await?;
            // save_body(&mut req, file).await
        )
        .delete(delete_file),
    );

    // configure certificate and private key used by https
    let config = match tls {
        true => {
            Some(
                RustlsConfig::from_pem_file(
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("self_signed_certs")
                        .join("cert.pem"),
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("self_signed_certs")
                        .join("key.pem"),
                )
                .await
                .unwrap(),
            );
        }
        false => None,
    };

    // match tls {
    //     false => app.listen(addr).await?,
    //     true => {
    //         app.listen(
    //             TlsListener::build()
    //                 .addrs(addr)
    //                 .cert(cert.expect("--cert not given"))
    //                 .key(key.expect("--key not given")),
    //         )
    //         .await?
    //     }
    // };

    // run https server
    let addr = SocketAddr::from(([127, 0, 0, 1], ports.https));
    tracing::debug!("listening on {}", addr);
    match config {
        Some(config) => axum_server::bind_rustls(addr, config)
            .serve(app.into_make_service())
            .await
            .unwrap(),
        None => axum_server::bind(addr),
    }

    Ok(())
}
