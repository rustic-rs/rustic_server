// mod web
//
// implements a REST server as specified by
// https://restic.readthedocs.io/en/stable/REST_backend.html?highlight=Rest%20API
//
// uses the modules
// storage - to access the file system
// auth    - for user authentication
// acl     - for access control

use std::convert::TryInto;
use std::marker::Unpin;
use std::path::Path;
use std::sync::Arc;

use http_range::HttpRange;

use axum::{
    body::Body,
    extract::Host,
    handler::HandlerWithoutStateExt,
    http::{Request, StatusCode, Uri},
    response::{IntoResponse, Redirect, Response},
    routing::get,
    BoxError, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use serde_derive::{Deserialize, Serialize};
use std::io;
use std::{net::SocketAddr, path::PathBuf};

use super::acl::{AccessType, AclChecker};
use super::auth::AuthChecker;
use super::helpers::IteratorAdapter;
use super::storage::Storage;

#[derive(Clone, Copy)]
pub struct Ports {
    http: u16,
    https: u16,
}

#[derive(Clone)]
pub struct State {
    auth: Arc<dyn AuthChecker>,
    acl: Arc<dyn AclChecker>,
    storage: Arc<dyn Storage>,
}

#[async_trait::async_trait]
impl tide_http_auth::Storage<String, BasicAuthRequest> for State {
    async fn get_user(&self, request: BasicAuthRequest) -> Result<Option<String>> {
        let user = request.username;
        match self.auth.verify(&user, &request.password) {
            true => Ok(Some(user)),
            false => Ok(None),
        }
    }
}

impl State {
    pub fn new(auth: impl AuthChecker, acl: impl AclChecker, storage: impl Storage) -> Self {
        Self {
            storage: Arc::new(storage),
            auth: Arc::new(auth),
            acl: Arc::new(acl),
        }
    }
}

const TYPES: [&str; 5] = ["data", "keys", "locks", "snapshots", "index"];
const DEFAULT_PATH: &str = "";
const CONFIG_TYPE: &str = "config";
const CONFIG_NAME: &str = "";

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

fn check_name(tpe: &str, name: &str) -> Result<(), axum::Error> {
    match tpe {
        "config" => Ok(()),
        _ if check_string_sha256(name) => Ok(()),
        _ => Err(axum::Error::from_str(
            StatusCode::Forbidden,
            format!("filename {} not allowed", name),
        )),
    }
}

fn check_auth_and_acl(
    req: &Request<State>,
    path: &Path,
    tpe: &str,
    append: AccessType,
) -> Result<(), axum::Error> {
    let state = req.state();

    // don't allow paths that includes any of the defined types
    for part in path.iter() {
        if let Some(part) = part.to_str() {
            for tpe in TYPES.iter() {
                if &part == tpe {
                    return Err(axum::Error::from_str(StatusCode::Forbidden, "not allowed"));
                }
            }
        }
    }

    let empty = String::new();
    let user: &str = req.ext::<String>().unwrap_or(&empty);
    let path = path.to_str().ok_or(axum::Error::from_str(
        StatusCode::Forbidden,
        "path is non-unicode",
    ))?;
    let allowed = state.acl.allowed(user, path, tpe, append);
    tracing::debug!("[auth] user: {user}, path: {path}, tpe: {tpe}, allowed: {allowed}");

    match allowed {
        true => Ok(()),
        false => Err(axum::Error::from_str(StatusCode::Forbidden, "not allowed")),
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct Create {
    create: bool,
}

async fn create_dirs(path: &str, req: &Request<State>) -> Result {
    tracing::debug!("[create_dirs] path: {path}");

    let path = Path::new(path);
    check_auth_and_acl(req, path, "", AccessType::Append)?;
    let c: Create = req.query()?;
    match c.create {
        true => {
            for tpe in TYPES.iter() {
                req.state().storage.create_dir(path, tpe)?;
            }
            Ok(format!("Called create_files with path {:?}\n", path).into())
        }
        false => Ok(format!("Called create_files with path {:?}, create=false\n", path).into()),
    }
}

const API_V1: &str = "application/vnd.x.restic.rest.v1";
const API_V2: &str = "application/vnd.x.restic.rest.v2";

#[derive(Serialize)]
struct RepoPathEntry {
    name: String,
    size: u64,
}

async fn list_files(path: &str, tpe: &str, req: &Request<State>) -> Result {
    tracing::debug!("[list_files] path: {path}, tpe: {tpe}");

    let path = Path::new(path);
    check_auth_and_acl(req, path, tpe, AccessType::Read)?;

    let read_dir = req.state().storage.read_dir(path, tpe);
    let mut res = Response::new(StatusCode::Ok);

    // TODO: error handling
    match req.header("Accept") {
        Some(a) if a.as_str() == API_V2 => {
            res.set_content_type(API_V2);
            let read_dir_version = read_dir.map(|e| RepoPathEntry {
                name: e.file_name().to_str().unwrap().to_string(),
                size: e.metadata().unwrap().len(),
            });
            res.set_body(Body::from_json(&IteratorAdapter::new(read_dir_version))?);
        }
        _ => {
            res.set_content_type(API_V1);
            let read_dir_version = read_dir.map(|e| e.file_name().to_str().unwrap().to_string());
            res.set_body(Body::from_json(&IteratorAdapter::new(read_dir_version))?);
        }
    };
    Ok(res)
}

async fn length(path: &str, tpe: &str, name: &str, req: &Request<State>) -> Result {
    tracing::debug!("[length] path: {path}, tpe: {tpe}, name: {name}");

    check_name(tpe, name)?;
    let path = Path::new(path);
    check_auth_and_acl(req, path, tpe, AccessType::Read)?;

    let _file = req.state().storage.filename(path, tpe, name);
    Err(axum::Error::from_str(
        StatusCode::NotImplemented,
        "not yet implemented",
    ))
}

async fn get_file(path: &str, tpe: &str, name: &str, req: &Request<State>) -> Result {
    tracing::debug!("[get_file] path: {path}, tpe: {tpe}, name: {name}");

    check_name(tpe, name)?;
    let path = Path::new(path);
    check_auth_and_acl(req, path, tpe, AccessType::Read)?;

    let mut file = req.state().storage.open_file(path, tpe, name).await?;
    let mut len = file.metadata().await?.len();

    let mut res;
    match req.header("Range") {
        None => {
            res = Response::new(StatusCode::Ok);
        }
        Some(r) => match HttpRange::parse(r.as_str(), len) {
            Ok(range) if range.len() == 1 => {
                file.seek(Start(range[0].start)).await?;
                len = range[0].length;
                res = Response::new(StatusCode::PartialContent);
            }
            Ok(_) => {
                return Err(axum::Error::from_str(
                    StatusCode::NotImplemented,
                    "multipart range not implemented",
                ))
            }
            Err(_) => {
                return Err(axum::Error::from_str(
                    StatusCode::InternalServerError,
                    "range error",
                ))
            }
        },
    };

    let file = io::BufReader::new(file);
    res.set_body(Body::from_reader(file, Some(len.try_into()?)));
    Ok(res)
}

#[async_trait::async_trait]
pub trait Finalizer {
    async fn finalize(&mut self) -> Result<(), io::Error>;
}

async fn save_body(
    req: &mut Request<State>,
    mut file: impl io::Write + Unpin + Finalizer,
) -> Result {
    let bytes_written = io::copy(req, &mut file).await?;
    tracing::debug!("[file written] bytes: {bytes_written}");
    file.finalize().await?;
    Ok(Response::new(StatusCode::Ok))
}

async fn get_save_file(
    path: &str,
    tpe: &str,
    name: &str,
    req: &Request<State>,
) -> Result<impl io::Write + Unpin + Finalizer, axum::Error> {
    tracing::debug!("[get_save_file] path: {path}, tpe: {tpe}, name: {name}");

    check_name(tpe, name)?;
    let path = Path::new(path);
    check_auth_and_acl(req, path, tpe, AccessType::Append)?;

    Ok(req.state().storage.create_file(path, tpe, name).await?)
}

async fn delete_file(path: &str, tpe: &str, name: &str, req: &Request<State>) -> Result {
    check_name(tpe, name)?;
    let path = Path::new(path);
    check_auth_and_acl(req, path, tpe, AccessType::Modify)?;
    req.state().storage.remove_file(path, tpe, name)?;
    Ok(Response::new(StatusCode::Ok))
}
// TODO!: https://github.com/tokio-rs/axum/blob/main/examples/tls-rustls/src/main.rs
// TODO!: https://github.com/tokio-rs/axum/blob/main/examples/readme/src/main.rs
pub async fn main(
    state: State,
    addr: String,
    tls: bool,
    cert: Option<String>,
    key: Option<String>,
) -> Result<()> {
    let mid = tide_http_auth::Authentication::new(BasicAuthScheme);
    let mut app = tide::with_state(state);
    app.with(mid);

    app.at("/:path/")
        .post(|req: Request<State>| async move { create_dirs(req.param("path")?, &req).await });
    app.at("/")
        .post(|req| async move { create_dirs(DEFAULT_PATH, &req).await });

    for tpe in TYPES.iter() {
        let path = &("/".to_string() + tpe + "/");
        tracing::debug!("add path: {path}");
        app.at(path)
            .get(move |req| async move { list_files(DEFAULT_PATH, tpe, &req).await });

        let path = &("/".to_string() + tpe + "/:name");
        tracing::debug!("add path: {path}");
        app.at(path)
            .head(move |req: Request<State>| async move {
                length(DEFAULT_PATH, tpe, req.param("name")?, &req).await
            })
            .get(move |req: Request<State>| async move {
                get_file(DEFAULT_PATH, tpe, req.param("name")?, &req).await
            })
            .post(move |mut req: Request<State>| async move {
                let file = get_save_file(DEFAULT_PATH, tpe, req.param("name")?, &req).await?;
                save_body(&mut req, file).await
            })
            .delete(move |req: Request<State>| async move {
                delete_file(DEFAULT_PATH, tpe, req.param("name")?, &req).await
            });

        let path = &("/:path/".to_string() + tpe + "/");
        tracing::debug!("add path: {path}");
        app.at(path).get(move |req: Request<State>| async move {
            list_files(req.param("path")?, tpe, &req).await
        });

        let path = &("/:path/".to_string() + tpe + "/:name");
        tracing::debug!("add path: {path}");
        app.at(path)
            .head(move |req: Request<State>| async move {
                length(req.param("path")?, tpe, req.param("name")?, &req).await
            })
            .get(move |req: Request<State>| async move {
                get_file(req.param("path")?, tpe, req.param("name")?, &req).await
            })
            .post(move |mut req: Request<State>| async move {
                let file = get_save_file(req.param("path")?, tpe, req.param("name")?, &req).await?;
                save_body(&mut req, file).await
            })
            .delete(move |req: Request<State>| async move {
                delete_file(req.param("path")?, tpe, req.param("name")?, &req).await
            });
    }

    app.at("config")
        .get(|req| async move { get_file(DEFAULT_PATH, CONFIG_TYPE, CONFIG_NAME, &req).await })
        .post(|mut req| async move {
            let file = get_save_file(DEFAULT_PATH, CONFIG_TYPE, CONFIG_NAME, &req).await?;
            save_body(&mut req, file).await
        })
        .delete(
            |req| async move { delete_file(DEFAULT_PATH, CONFIG_TYPE, CONFIG_NAME, &req).await },
        );

    app.at("/:path/config")
        .get(|req: Request<State>| async move {
            get_file(req.param("path")?, CONFIG_TYPE, CONFIG_NAME, &req).await
        })
        .post(|mut req: Request<State>| async move {
            let file = get_save_file(req.param("path")?, CONFIG_TYPE, CONFIG_NAME, &req).await?;
            save_body(&mut req, file).await
        })
        .delete(|req: Request<State>| async move {
            delete_file(req.param("path")?, CONFIG_TYPE, CONFIG_NAME, &req).await
        });

    match tls {
        false => app.listen(addr).await?,
        true => {
            app.listen(
                TlsListener::build()
                    .addrs(addr)
                    .cert(cert.expect("--cert not given"))
                    .key(key.expect("--key not given")),
            )
            .await?
        }
    };
    Ok(())
}
