// mod web
//
// implements a REST server as specified by
// https://restic.readthedocs.io/en/stable/REST_backend.html?highlight=Rest%20API
//
// uses the modules
// storage - to access the file system
// auth    - for user authentication
// acl     - for access control

use anyhow::Error;
use axum::{
    body::{Body, StreamBody},
    extract::{FromRef, Path as PathExtract, Query, State, TypedHeader},
    handler::{Handler, HandlerWithoutStateExt},
    http::{header, Request, StatusCode},
    response::{AppendHeaders, IntoResponse, Response},
    routing::{get, head, post},
    Json, Router,
};

use axum_macros::debug_handler;

use axum_server::tls_rustls::RustlsConfig;
use http_range::HttpRange;
use serde_derive::{Deserialize, Serialize};
use std::{convert::TryInto, marker::Unpin, path::Path as StdPath, sync::Arc};
use std::{net::SocketAddr, path::PathBuf};
use tokio::io::AsyncWrite;
use tokio::io::SeekFrom::Start;
use tokio::{io::copy, io::AsyncSeekExt};
use tokio_util::io::ReaderStream;

use crate::{
    acl::{AccessType, Acl, AclChecker},
    auth::{Auth, AuthChecker},
    error::{ErrorKind, Result},
    helpers::{Finalizer, IteratorAdapter},
    storage::{LocalStorage, Storage},
};

use crate::state::AppState;

const API_V1: &str = "application/vnd.x.restic.rest.v1";
const API_V2: &str = "application/vnd.x.restic.rest.v2";
const TYPES: [&str; 5] = ["data", "keys", "locks", "snapshots", "index"];
const DEFAULT_PATH: &str = "";
const CONFIG_TYPE: &str = "config";
const CONFIG_NAME: &str = "";

#[derive(Clone)]
struct TpeState(pub String);

#[derive(Serialize)]
struct RepoPathEntry {
    name: String,
    size: u64,
}

#[derive(Clone, Copy)]
pub struct Ports {
    pub http: u16,
    pub https: u16,
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

fn check_name(tpe: &str, name: &str) -> Result<impl IntoResponse> {
    match tpe {
        "config" => Ok(()),
        _ if check_string_sha256(name) => Ok(()),
        _ => Err(ErrorKind::FilenameNotAllowed(name.to_string()).into()),
    }
}

fn check_auth_and_acl(
    state: &AppState,
    path: &StdPath,
    tpe: &str,
    append: AccessType,
) -> Result<impl IntoResponse> {
    // don't allow paths that includes any of the defined types
    for part in path.iter() {
        if let Some(part) = part.to_str() {
            for tpe in TYPES.iter() {
                if &part == tpe {
                    return Err(ErrorKind::PathNotAllowed(path.display().to_string()).into());
                }
            }
        }
    }

    let empty = String::new();
    // TODO!: How to get extension value?
    let user: &str = state.ext::<String>().unwrap_or(&empty);
    let Some(path) = path.to_str() else {
        return Err(ErrorKind::NonUnicodePath(path.display().to_string()).into());
    };
    let allowed = state.acl().allowed(user, path, tpe, append);
    tracing::debug!("[auth] user: {user}, path: {path}, tpe: {tpe}, allowed: {allowed}");

    match allowed {
        true => Ok(StatusCode::OK),
        false => Err(ErrorKind::PathNotAllowed(path.to_string()).into()),
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct Create {
    create: bool,
}

#[debug_handler]
async fn create_dirs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<Create>,
    path: Option<PathExtract<String>>,
) -> Result<impl IntoResponse> {
    let unpacked_path = path.map_or(DEFAULT_PATH.to_string(), |PathExtract(path_ext)| path_ext);
    let path = StdPath::new(&unpacked_path);

    tracing::debug!("[create_dirs] path: {path:?}");

    check_auth_and_acl(&state, path, "", AccessType::Append)?;
    let c: Create = params;
    match c.create {
        true => {
            for tpe in TYPES.iter() {
                match state.storage().create_dir(path, tpe) {
                    Ok(_) => (),
                    Err(e) => return Err(ErrorKind::CreatingDirectoryFailed(e.to_string()).into()),
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

#[debug_handler]
async fn list_files(
    State(state): State<Arc<AppState>>,
    path: Option<PathExtract<String>>,
    req: Request<Body>,
) -> Result<impl IntoResponse> {
    let tpe = state.tpe();
    let unpacked_path = path.map_or(DEFAULT_PATH.to_string(), |PathExtract(path_ext)| path_ext);
    let path = StdPath::new(&unpacked_path);

    tracing::debug!("[list_files] path: {path:?}, tpe: {tpe}");

    check_auth_and_acl(&state, path, tpe, AccessType::Read)?;

    let read_dir = state.storage().read_dir(path, tpe);

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

#[debug_handler]
async fn length(
    PathExtract(path): PathExtract<String>,
    State(state): State<Arc<AppState>>,
    PathExtract(name): PathExtract<String>,
    _req: Request<Body>,
) -> Result<()> {
    let tpe = state.tpe();
    tracing::debug!("[length] path: {path}, tpe: {tpe}, name: {name}");
    let path = StdPath::new(&path);

    check_name(tpe, name.as_str())?;
    check_auth_and_acl(&state, path, tpe, AccessType::Read)?;

    let _file = state.storage().filename(path, tpe, name.as_str());
    Err(ErrorKind::NotImplemented.into())
}

#[debug_handler]
async fn get_file(
    State(state): State<Arc<AppState>>,
    PathExtract(name): PathExtract<String>,
    path: Option<PathExtract<String>>,
    req: Request<Body>,
) -> Result<impl IntoResponse> {
    let tpe = state.tpe();
    let unpacked_path = path.map_or(DEFAULT_PATH.to_string(), |PathExtract(path_ext)| path_ext);
    let path = StdPath::new(&unpacked_path);

    tracing::debug!("[get_file] path: {path:?}, tpe: {tpe}, name: {name}");

    check_name(tpe, name.as_str())?;
    let path = StdPath::new(path);
    check_auth_and_acl(&state, path, tpe, AccessType::Read)?;

    let Ok(mut file) = state.storage().open_file(path, tpe, name.as_str()).await else {
        return Err(ErrorKind::FileNotFound(path.display().to_string()).into());
    };

    let mut len = match file.metadata().await {
        Ok(val) => val.len(),
        Err(_) => {
            return Err(ErrorKind::GettingFileMetadataFailed.into());
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
                Err(_) => return Err(ErrorKind::RangeNotValid.into()),
            },
            len,
        ) {
            Ok(range) if range.len() == 1 => {
                let Ok(_) = file.seek(Start(range[0].start)).await else {
                    return Err(ErrorKind::SeekingFileFailed.into());
                };

                len = range[0].length;
                status = StatusCode::PARTIAL_CONTENT;
            }
            Ok(_) => return Err(ErrorKind::MultipartRangeNotImplemented.into()),
            Err(_) => return Err(ErrorKind::GeneralRange.into()),
        },
    };

    // From: https://github.com/tokio-rs/axum/discussions/608#discussioncomment-1789020
    let stream = ReaderStream::with_capacity(
        file,
        match len.try_into() {
            Ok(val) => val,
            Err(_) => return Err(ErrorKind::ConversionToU64Failed.into()),
        },
    );
    let body = StreamBody::new(stream);

    let headers = AppendHeaders([(header::CONTENT_TYPE, "application/octet-stream")]);
    Ok((status, headers, body))
}

async fn save_body(
    mut file: impl AsyncWrite + Unpin + Finalizer,
    req: &mut Request<Body>,
) -> Result<impl IntoResponse> {
    let bytes_written = match copy(req, &mut file).await {
        Ok(val) => val,
        Err(_) => return Err(ErrorKind::WritingToFileFailed.into()),
    };
    tracing::debug!("[file written] bytes: {bytes_written}");
    let Ok(_) = file.finalize().await else {
        return Err(ErrorKind::FinalizingFileFailed.into());
    };
    Ok(StatusCode::OK)
}

async fn get_save_file(
    path: Option<String>,
    state: Arc<AppState>,
    name: String,
) -> std::result::Result<impl AsyncWrite + Unpin + Finalizer, ErrorKind> {
    let tpe = state.tpe();
    let unpacked_path = path.map_or(DEFAULT_PATH.to_string(), |path_ext| path_ext);
    let path = StdPath::new(&unpacked_path);

    tracing::debug!("[get_save_file] path: {path:?}, tpe: {tpe}, name: {name}");

    let Ok(_) = check_name(tpe, name.as_str()) else {
        return Err(ErrorKind::FilenameNotAllowed(name).into());
    };

    let Ok(_) = check_auth_and_acl(&state, path, tpe, AccessType::Append) else {
        return Err(ErrorKind::PathNotAllowed(path.display().to_string()).into());
    };

    match state.storage().create_file(path, tpe, name.as_str()).await {
        Ok(val) => Ok(val),
        Err(_) => return Err(ErrorKind::GettingFileHandleFailed.into()),
    }
}

#[debug_handler]
async fn delete_file(
    State(state): State<Arc<AppState>>,
    PathExtract(name): PathExtract<String>,
    path: Option<PathExtract<String>>,
) -> Result<impl IntoResponse> {
    let tpe = state.tpe();
    let unpacked_path = path.map_or(DEFAULT_PATH.to_string(), |PathExtract(path_ext)| path_ext);
    let path = StdPath::new(&unpacked_path);

    let Ok(_) = check_name(tpe, name.as_str()) else {
        return Err(ErrorKind::FilenameNotAllowed(name).into());
    };

    let Ok(_) = check_auth_and_acl(&state, path, tpe, AccessType::Modify) else {
        return Err(ErrorKind::PathNotAllowed(path.display().to_string()).into());
    };

    let Ok(_) = state.storage().remove_file(path, tpe, name.as_str()) else {
        return Err(ErrorKind::RemovingFileFailed.into());
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
    mut state: AppState,
    addr: String,
    ports: Ports,
    tls: bool,
    cert: Option<String>,
    key: Option<String>,
) -> Result<()> {
    // TODO!
    // let mid = tide_http_auth::Authentication::new(BasicAuthScheme);

    // app.with(mid);

    let shared_state = Arc::new(state);

    let mut app = Router::new();

    app.route("/", post(create_dirs).with_state(shared_state));
    app.route("/:path/", post(create_dirs).with_state(shared_state));

    for tpe in TYPES.into_iter() {
        let path = &("/".to_string() + tpe + "/");
        tracing::debug!("add path: {path}");

        shared_state.set_tpe(tpe.to_string());

        app.route(path, get(list_files).with_state(shared_state));

        let path = &("/".to_string() + tpe + "/:name");
        tracing::debug!("add path: {path}");
        app.route(
            path,
            head(length)
                .get(get_file)
                .post(move |mut req: Request<Body>| async move {
                    let PathExtract(name): PathExtract<String>;
                    let file = get_save_file(None, shared_state, name).await?;

                    save_body(file, &mut req).await
                })
                .delete(delete_file),
        )
        .with_state(shared_state);

        let path = &("/:path/".to_string() + tpe + "/");
        tracing::debug!("add path: {path}");
        app.route(path, get(list_files)).with_state(shared_state);

        let path = &("/:path/".to_string() + tpe + "/:name");
        tracing::debug!("add path: {path}");
        app.route(
            path,
            head(length)
                .get(get_file)
                .post(move |mut req: Request<Body>| async move {
                    let PathExtract(name): PathExtract<String>;
                    let PathExtract(path): PathExtract<String>;
                    let file = get_save_file(Some(path), shared_state, name).await?;

                    save_body(file, &mut req).await
                })
                .delete(delete_file),
        )
        .with_state(shared_state);
    }

    app.route(
        "config",
        get(get_file)
            .post(move |mut req: Request<Body>| async move {
                shared_state.set_tpe(CONFIG_TYPE.to_string());
                let file = get_save_file(None, shared_state, CONFIG_NAME.to_string()).await?;

                save_body(file, &mut req).await
            })
            .delete(delete_file),
    );

    app.route(
        "/:path/config",
        get(get_file)
            .post(move |mut req: Request<Body>| async move {
                let PathExtract(name): PathExtract<String>;
                let PathExtract(path): PathExtract<String>;
                shared_state.set_tpe(CONFIG_TYPE.to_string());
                let file = get_save_file(Some(path), shared_state, CONFIG_NAME.to_string()).await?;

                save_body(file, &mut req).await
            })
            .delete(delete_file),
    );

    // configure certificate and private key used by https
    let config = match tls {
        true => Some(
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
        ),
        false => None,
    };

    // run https server
    let addr = SocketAddr::from(([127, 0, 0, 1], ports.https));
    tracing::debug!("listening on {}", addr);
    match config {
        Some(config) => axum_server::bind_rustls(addr, config)
            .serve(app.into_make_service())
            .await
            .unwrap(),
        None => axum_server::bind(addr)
            .serve(app.into_make_service())
            .await
            .unwrap(),
    }

    Ok(())
}
