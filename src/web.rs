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
    body::Body,
    extract::{Path as PathExtract, Query},
    http::{header, Request, StatusCode},
    response::{AppendHeaders, IntoResponse},
    routing::{get, head, post},
    Json, Router,
};
use axum_macros::TypedPath;
use axum_macros::debug_handler;
use futures_util::StreamExt;
use http_range::HttpRange;
use serde_derive::{Deserialize, Serialize};
use std::{convert::TryInto, marker::Unpin, path::Path as StdPath};
use std::path::{Path, PathBuf};
use axum_extra::headers::HeaderMap;
use tokio::io::AsyncWrite;
use tokio::io::SeekFrom::Start;
use tokio::{io::copy, io::AsyncSeekExt};
use tokio_util::io::ReaderStream;
use crate::{
    acl::{AccessType, AclChecker},
    error::{ErrorKind, Result},
    handlers::file_helpers::{Finalizer, IteratorAdapter},
    storage::Storage,
};
use crate::acl::{Acl, ACL, init_acl};
use crate::auth::{Auth, AUTH, AuthChecker, AuthFromRequest, init_auth};
use crate::storage::{init_storage, STORAGE};

pub(crate) const API_V1:&str = "application/vnd.x.restic.rest.v1";
pub(crate) const API_V2:&str = "application/vnd.x.restic.rest.v2";

// TPE_LOCKS is is defined, but outside this types[] array.
// This allow us to loop over the types[] when generating "routes"
pub(crate) const TPE_DATA:&str = "data";
pub(crate) const TPE_KEYS:&str = "keys";
pub(crate) const TPE_LOCKS:&str = "locks";
pub(crate) const TPE_SNAPSHOTS:&str = "snapshots";
pub(crate) const TPE_INDEX:&str = "index";
pub(crate) const TPE_CONFIG: &str = "config";
pub(crate) const TYPES: [&str; 5] = [TPE_DATA, TPE_KEYS, TPE_LOCKS, TPE_SNAPSHOTS, TPE_INDEX];

pub(crate) const DEFAULT_PATH: &str = "";
pub(crate) const CONFIG_NAME: &str = "";


// #[derive(Serialize)]
// struct RepoPathEntry {
//     name: String,
//     size: u64,
// }
//
// fn check_string_sha256(name: &str) -> bool {
//     if name.len() != 64 {
//         return false;
//     }
//     for c in name.chars() {
//         if !c.is_ascii_digit() && !('a'..='f').contains(&c) {
//             return false;
//         }
//     }
//     true
// }
//
// fn check_name(tpe: &str, name: &str) -> Result<impl IntoResponse> {
//     match tpe {
//         "config" => Ok(()),
//         _ if check_string_sha256(name) => Ok(()),
//         _ => Err(ErrorKind::FilenameNotAllowed(name.to_string())),
//     }
// }

// pub(crate) fn check_auth_and_acl(
//     user:String,
//     tpe:&str,
//     path: &StdPath,
//     append: AccessType,
// ) -> Result<impl IntoResponse> {
//     // don't allow paths that includes any of the defined types
//     for part in path.iter() {
//         if let Some(part) = part.to_str() {
//             for tpe_i in TYPES.iter() {
//                 if &part == tpe_i {
//                     return Err(ErrorKind::PathNotAllowed(path.display().to_string()));
//                 }
//             }
//         }
//     }
//
//     let acl = ACL.get().unwrap();
//     let path = if let Some(path) = path.to_str() {
//         path
//     }
//     else {
//         return Err(ErrorKind::NonUnicodePath(path.display().to_string()));
//     };
//     let allowed = acl.allowed(user.as_str(), path, tpe, append);
//     tracing::debug!(
//         "[auth] user: {user}, path: {path}, tpe: {tpe}, allowed: {allowed}"
//     );
//
//     match allowed {
//         true => Ok(StatusCode::OK),
//         false => Err(ErrorKind::PathNotAllowed(path.to_string())),
//     }
// }

// //==============================================================================
// // Create_repository
// // Interface: POST {path}?create=true
// //==============================================================================
// #[derive(Default, Deserialize)]
// #[serde(default)]
// struct Create {
//     create: bool,
// }
//
// #[debug_handler]
// async fn create_repository(
//     a: AuthFromRequest,
//     Query(params): Query<Create>,
//     path: Option<PathExtract<String>>,
// ) -> Result<impl IntoResponse> {
//
//     let tpe = "";
//     let unpacked_path = path.map_or(DEFAULT_PATH.to_string(), |PathExtract(path_ext)| path_ext);
//     let path = StdPath::new(&unpacked_path);
//
//     tracing::debug!("[create_dirs] path: {path:?}");
//
//     check_auth_and_acl( a.user, &tpe, path, AccessType::Append)?;
//
//     let storage = STORAGE.get().unwrap();
//     let c: Create = params;
//     return match c.create {
//         true => {
//             for tpe_i in TYPES.iter() {
//                 match storage.create_dir(path, tpe_i) {
//                     Ok(_) => (),
//                     Err(e) => return Err(ErrorKind::CreatingDirectoryFailed(e.to_string()).into()),
//                 };
//             }
//
//             Ok((
//                 StatusCode::OK,
//                 format!("Called create_files with path {:?}\n", path),
//             ))
//         }
//         false => {
//             Ok((
//                 StatusCode::OK,
//                 format!("Called create_files with path {:?}, create=false\n", path),
//             ))
//         }
//     }
// }
//
// //==============================================================================
// // Delete_repository
// // Interface: Delete {path}
// //==============================================================================
// #[debug_handler]
// async fn delete_repository(
//     a: AuthFromRequest,
//     path:Option<PathExtract<String>>) -> Result<impl IntoResponse> {
//
//     let unpacked_path = path.map_or(DEFAULT_PATH.to_string(), |PathExtract(path_ext)| path_ext);
//     let path = StdPath::new(&unpacked_path);
//     tracing::debug!("[delete_repository] path: {path:?}");
//
//     check_auth_and_acl(a.user, "", path, AccessType::Append)?;
//
//     let storage = STORAGE.get().unwrap();
//     match storage.remove_repository(path) {
//         Ok(_) => Ok(()),
//         Err(_) => return Err(ErrorKind::CreatingDirectoryFailed(path.to_string_lossy().into()).into()),
//     }
// }

// //==============================================================================
// // List files
// // Interface: GET {path}/config
// //==============================================================================
// #[derive(TypedPath, Deserialize)]
// #[typed_path("/:path/:tpe/")]
// struct ListFilesPar {
//     pub path: String,
//     pub tpe: String,
// }
//
// #[debug_handler]
// async fn list_files(
//     a: AuthFromRequest,
//     param: ListFilesPar,
//     headers: HeaderMap,
// ) -> Result<impl IntoResponse> {
//
//     // let path_o = param.path;
//     // let p_str = match path_o {
//     //     Some(p) => p,
//     //     None => DEFAULT_PATH.to_string()
//     // };
//     let p_str = param.path;
//     let tpe = param.tpe;
//     tracing::debug!("[list_files] path: {p_str}, tpe: {tpe}");
//
//
//     let pth = Path::new(&p_str);
//     check_auth_and_acl(a.user, tpe.as_str(), &pth, AccessType::Read)?;
//
//     let storage = STORAGE.get().unwrap();
//     let read_dir = storage.read_dir(&pth, tpe.as_str());
//
//     let version = match headers.get(header::ACCEPT)
//         .and_then(|header| header.to_str().ok())
//         {
//         Some(API_V2) => {2},
//         _ => {1}
//     };
//
//     let res = if version == 2 {
//         let read_dir_version = read_dir.map(|e|{let a = 1;  RepoPathEntry {
//             name: e.file_name().to_str().unwrap().to_string(),
//             size: e.metadata().unwrap().len(),
//             // FIXME:  return Err(ErrorKind::GettingFileMetadataFailed.into());
//         }});
//         let mut response = Json(&IteratorAdapter::new(read_dir_version)).into_response();
//         response.headers_mut().insert(
//             header::CONTENT_TYPE,
//             header::HeaderValue::from_static(API_V2),
//         );
//         let status = response.status_mut();
//         *status = StatusCode::OK;
//         response
//     } else {
//         let read_dir_version = read_dir.map(|e| e.file_name().to_str().unwrap().to_string());
//         let mut response = Json(&IteratorAdapter::new(read_dir_version)).into_response();
//         response.headers_mut().insert(
//             header::CONTENT_TYPE,
//             header::HeaderValue::from_static(API_V1),
//         );
//         let status = response.status_mut();
//         *status = StatusCode::OK;
//         response
//     };
//     Ok(res)
// }

// //==============================================================================
// // Length
// // Interface: HEAD {path}/{type}/{name}
// //==============================================================================
// //FIXME
// //#[derive(TypedPath, Deserialize)]
// //#[typed_path("/AAP/:path/NOOT/:tpe/MIES/:name")]
// struct LengthPar {
//     AAP: AuthFromRequest,
//     pub path: String,
//     pub tpe: String,
//     pub name: String,
// }
//
// async fn length(a: AuthFromRequest, param: AddFilePar) -> Result<impl IntoResponse> {
//     let path_o = param.path;
//     let p_str = match path_o {
//         Some(p) => p,
//         None => DEFAULT_PATH.to_string()
//     };
//     let tpe = param.tpe;
//     let name = param.name;
//     tracing::debug!("[length] path: {p_str}, tpe: {tpe}, name: {name}");
//
//     check_name(tpe.as_str(), name.as_str())?;
//     let path = Path::new(&p_str);
//     check_auth_and_acl(a.user, tpe.as_str(), path, AccessType::Read)?;
//
//     let storage = STORAGE.get().unwrap();
//     let file = storage.filename(path, &tpe, &name);
//     return if file.exists() {
//         let storage = STORAGE.get().unwrap();
//         let file = match storage.open_file(&path, &tpe, &name).await {
//             Ok(file) => {file}
//             Err(_) => {
//                 return Err(ErrorKind::FileNotFound(p_str)).into();
//             }
//         };
//         let length = match file.metadata().await {
//             Ok(meta) => meta.len(),
//             Err(_) => {
//                 return Err(ErrorKind::GettingFileMetadataFailed.into());
//             }
//         };
//         let mut headers = HeaderMap::new();
//         headers.insert(
//             header::CONTENT_LENGTH,
//             length.into()
//         );
//         Ok(headers)
//     } else {
//         Err(ErrorKind::FileNotFound(p_str).into())
//     }
// }

// //==============================================================================
// // add_file
// // Interface: POST {path}/{type}/{name}
// //==============================================================================
// //FIXME
// // #[derive(TypedPath, Deserialize)]
// // #[typed_path("/:path/:tpe/:name")]
// struct AddFilePar {
//     pub path: Option<String>,
//     pub tpe: String,
//     pub name: String,
// }
//
// // //FIXME
// //#[debug_handler]
// async fn add_file(a: AuthFromRequest, param: AddFilePar, header:HeaderMap)
//     -> Result<impl IntoResponse> {
//     let path_o = param.path;
//     let p_str = match path_o {
//         Some(p) => p,
//         None => DEFAULT_PATH.to_string()
//     };
//     let tpe = param.tpe;
//     let name = param.name;
//     tracing::debug!("[get_file] path: {p_str}, tpe: {tpe}, name: {name}");
//
//     let pth = PathBuf::new().join(&p_str);
//     let file = get_save_file(a, pth, tpe.as_str(), name).await?;
//
//     //FIXME
//     // let mut async_body: Body;
//     // save_body(file, &mut async_body).await
//     Ok(())
// }

//==============================================================================
// add_config
// Interface: POST {path}/config
//==============================================================================
//FIXME
// #[derive(TypedPath, Deserialize)]
// #[typed_path("/:path/:name")]
struct AddConfigPar {
    pub path: Option<String>,
    pub name: String,
}

// //FIXME
// //#[debug_handler]
// async fn add_config(a: AuthFromRequest, param: AddConfigPar) -> Result<impl IntoResponse> {
//     let path_o = param.path;
//     let p_str = match path_o {
//         Some(p) => p,
//         None => DEFAULT_PATH.to_string()
//     };
//     let tpe = TPE_CONFIG.to_string();
//     let name = param.name;
//     tracing::debug!("[add_config] path: {p_str}, tpe: {tpe}, name: {name}");
//
//     let pth = PathBuf::new().join(&p_str);
//     //let file = get_save_file(a, pth, tpe.as_str(), name).await?;
//
//     //FIXME
//     // let mut async_body: Body;
//     // save_body(file, &mut async_body).await
//     Ok(())
// }


// //==============================================================================
// // get_file
// // Interface: GET {path}/{type}/{name}
// //==============================================================================
// //FIXME
// // #[derive(TypedPath, Deserialize)]
// // #[typed_path("/:path/:tpe/:name")]
// struct GetPathPar {
//     pub path: Option<String>,
//     pub tpe: String,
//     pub name: String,
// }
//
// //FIXME
// //#[debug_handler]
// async fn get_file(a: AuthFromRequest, param:GetPathPar, header:HeaderMap)
//     -> Result<impl IntoResponse> {
//     let path_o = param.path;
//     let p_str = match path_o {
//         Some(p) => p,
//         None => DEFAULT_PATH.to_string()
//     };
//     let tpe = param.tpe;
//     let name = param.name;
//     tracing::debug!("[get_file] path: {p_str}, tpe: {tpe}, name: {name}");
//
//     check_name(tpe.as_str(), name.as_str())?;
//     let pth = Path::new(&p_str);
//     check_auth_and_acl(a.user, tpe.as_str(), pth, AccessType::Read)?;
//
//     let storage = STORAGE.get().unwrap();
//     let mut file = match storage.open_file(&pth, &tpe, &name).await {
//         Ok(file) => {file}
//         Err(_) => {
//             return Err(ErrorKind::FileNotFound(p_str));
//         }
//     };
//     let mut len = match file.metadata().await {
//         Ok(val) => val.len(),
//         Err(_) => {
//             return Err(ErrorKind::GettingFileMetadataFailed.into());
//         }
//     };
//
//     let status;
//     match header.get(header::RANGE) {
//         None => {
//             status = StatusCode::OK;
//         }
//         Some(header_value) => match HttpRange::parse(
//             match header_value.to_str() {
//                 Ok(val) => val,
//                 Err(_) => return Err(ErrorKind::RangeNotValid.into()),
//             },
//             len,
//         ) {
//             Ok(range) if range.len() == 1 => {
//                 match file.seek(Start(range[0].start)).await {
//                     Ok(_) => {},
//                     Err(_) => {
//                         return Err(ErrorKind::SeekingFileFailed.into());
//                     }
//                 };
//
//                 len = range[0].length;
//                 status = StatusCode::PARTIAL_CONTENT;
//             }
//             Ok(_) => return Err(ErrorKind::MultipartRangeNotImplemented.into()),
//             Err(_) => return Err(ErrorKind::GeneralRange.into()),
//         },
//     };
//
//     // From: https://github.com/tokio-rs/axum/discussions/608#discussioncomment-1789020
//     let stream = ReaderStream::with_capacity(
//         file,
//         match len.try_into() {
//             Ok(val) => val,
//             Err(_) => return Err(ErrorKind::ConversionToU64Failed.into()),
//         },
//     );
//
//     let body = Body::from_stream(stream);
//
//     let headers = AppendHeaders([(header::CONTENT_TYPE, "application/octet-stream")]);
//     Ok((status, headers, body))
// }

// //==============================================================================
// // get_config
// // Interface: GET {path}/config
// //==============================================================================
// //FIXME
// // #[derive(TypedPath, Deserialize)]
// // #[typed_path("/:path/config")]
// struct GetConfigPar {
//     pub path: Option<String>,
// }
//
// //FIXME
// //#[debug_handler]
// async fn get_config(a: AuthFromRequest, param:GetConfigPar, header:HeaderMap)
//     -> Result<impl IntoResponse> {
//     let path_o = param.path;
//     let tpe = TPE_CONFIG.to_string();
//     tracing::debug!("[get_config] path: {path_o:?}, tpe: {tpe}, name: config");
//
//     let pp = GetPathPar{ path:path_o, tpe, name: "config".to_string() };
//     return get_file( a,pp, header).await;
// }

// //==============================================================================
// // delete_file
// // Interface: DELETE {path}/{type}/{name}
// //==============================================================================
// //FIXME
// // #[derive(TypedPath, Deserialize)]
// // #[typed_path("/:path/:tpe/:name")]
// struct DeletePathPar {
//     pub path: Option<String>,
//     pub tpe: String,
//     pub name: String,
// }
//
//
// //FIXME
// //#[debug_handler]
// async fn delete_file(a: AuthFromRequest, param: DeletePathPar) -> Result<impl IntoResponse> {
//     let path_o = param.path;
//     let p_str = match path_o {
//         Some(p) => p,
//         None => DEFAULT_PATH.to_string()
//     };
//     let tpe = param.tpe;
//     let name = param.name;
//     tracing::debug!("[delete_file] path: {p_str}, tpe: {tpe}, name: {name}");
//
//     check_name(tpe.as_str(), name.as_str())?;
//     let pth = Path::new(&p_str);
//     check_auth_and_acl(a.user, tpe.as_str(), pth, AccessType::Append)?;
//
//     let storage= STORAGE.get().unwrap();
//     let pth = Path::new(&p_str);
//     match storage.remove_file(pth, tpe.as_str(), name.as_str()) {
//         Ok(_) => Ok(()),
//         Err(_) => return Err(ErrorKind::RemovingFileFailed(p_str))
//     }
// }

// //==============================================================================
// // delete_config
// // Interface: DELETE {path}/config
// // FIXME: The original restic spec does not define delete_config --> but rustic did ??
// //==============================================================================
// //FIXME
// // #[derive(TypedPath, Deserialize)]
// // #[typed_path("/:path/config")]
// struct DeleteConfigPar {
//     pub path: Option<String>,
// }
//
//
// //FIXME
// //#[debug_handler]
// async fn delete_config(a: AuthFromRequest, param: DeleteConfigPar) -> Result<impl IntoResponse> {
//     let path_o = param.path;
//     let p_str = match path_o {
//         Some(p) => p,
//         None => DEFAULT_PATH.to_string()
//     };
//     let tpe = TPE_CONFIG.to_string();
//     let name = "config";
//     tracing::debug!("[delete_config] path: {p_str}, tpe: {tpe}, name: {name}");
//
//     check_name(tpe.as_str(), name)?;
//     let path = Path::new(&p_str);
//     let pth = Path::new(path);
//     check_auth_and_acl(a.user, tpe.as_str(), pth, AccessType::Append)?;
//
//     let storage= STORAGE.get().unwrap();
//     match storage.remove_file(path, tpe.as_str(), name) {
//         Ok(_) => Ok(()),
//         Err(_) => return Err(ErrorKind::RemovingFileFailed(p_str))
//     }
// }



// //==============================================================================
// // Support function: save_body
// //
// //==============================================================================
// async fn save_body(
//     mut file: impl AsyncWrite + Unpin + Finalizer,
//     stream: &mut Body,
// ) -> Result<impl IntoResponse> {
//     let mut bytes_written_overall = 0_u64;
//
//     // while let Some(chunk) = stream.into_data_stream().next().await {
//     //     let mut bytes = match chunk {
//     //         Ok(b) => b,
//     //         Err(_) => return Err(ErrorKind::ReadingFromStreamFailed.into())
//     //     };
//     //
//     //     //FIXME
//     //     // let bytes_written = match copy(bytes, &mut file).await {
//     //     //     Ok(val) => val,
//     //     //     Err(_) => return Err(ErrorKind::WritingToFileFailed.into()),
//     //     // };
//     //     // bytes_written_overall += bytes_written;
//     // }
//
//     tracing::debug!("[file written] bytes: {bytes_written_overall}");
//     match file.finalize().await {
//         Ok(_) => {},
//         Err(_) => return Err(ErrorKind::FinalizingFileFailed.into())
//     };
//     Ok(StatusCode::OK)
// }

// //==============================================================================
// // Support function: get_save_file
// //
// //==============================================================================
// async fn get_save_file(a: AuthFromRequest, path: PathBuf, tpe: &str, name: String, )
//     -> std::result::Result<impl AsyncWrite + Unpin + Finalizer, ErrorKind>
// {
//     tracing::debug!("[get_save_file] path: {path:?}, tpe: {tpe}, name: {name}");
//
//     match check_name(tpe, name.as_str()) {
//         Ok(_) => {}
//         Err(_) => {
//             return Err(ErrorKind::FilenameNotAllowed(name));
//         }
//     };
//
//     match check_auth_and_acl(a.user, tpe, path.as_path(), AccessType::Append) {
//         Ok(_) => {}
//         Err(_) => {
//             return Err(ErrorKind::PathNotAllowed(path.display().to_string()));
//         }
//     };
//
//     let storage = STORAGE.get().unwrap();
//     match storage.create_file(&path, &tpe, &name).await {
//         Ok(val) => Ok(val),
//         Err(_) => return Err(ErrorKind::GettingFileHandleFailed.into()),
//     }
// }

//==============================================================================
// MAIN                                                                   MAIN
//==============================================================================
pub async fn main(
    acl:Acl,
    auth:Auth,
    storage: impl Storage,
    addr: String,
    tls: bool,
    cert: Option<String>,
    key: Option<String>,
) -> Result<()> {

    // Initializing static authentication data and storage location
    init_acl(acl)?;
    init_auth(auth)?;
    init_storage(storage)?;

    // FIXME: TLS

    // // Create routing structure
    // let mut app = Router::new()
    //     .route(
    //         "/",
    //         post(create_repository)
    //             .get(list_files)
    //     );
    //
    // // Loop over types. Rationale:
    // // We can not distinguish these 2 paths using :tpe in the router:
    // //  - /a/path/to/file
    // //  - /a/path/locks/a_name
    // // This would confuse with the get(list_files) route "/:path/"
    // //
    // // Further some path parameters are made optional, so that we can apply the same
    // // function also for files in the root of the repository.
    // //
    // // FIXME: It seems important in which order the paths are checked in the routes ...
    // for tpe in TYPES.into_iter() {
    //     let path1 = format!("/:path/{}/", &tpe);
    //     let path2 = format!("/:path/{}/:name", &tpe);
    //     let mut app = app
    //         .route(
    //             path1.as_str(),
    //             get(list_files)
    //         )
    //         .route(
    //             path2.as_str(),
    //             head(length)
    //                 .get(get_file)
    //                 .post(add_file)
    //                 .delete(delete_file)
    //         );
    // }
    // let mut app = app
    //     .route(
    //         "/:path/config",
    //             //head()
    //                 get(get_config)
    //                 .post(add_config)
    //                 // FIXME: original restic interface does not have this; rustic_server did ...?
    //                 .delete(delete_config)
    //
    //     ).route(
    //         "/:path/",
    //         post(create_repository)
    //             .delete(delete_repository)
    //     );

// /// FIXME hier was ik gebleven ...
//     match tls {
//         false => app.listen(addr).await?,
//         true => {
//             app.listen(
//                 TlsListener::build()
//                     .addrs(addr)
//                     .cert(cert.expect("--cert not given"))
//                     .key(key.expect("--key not given")),
//             )
//             .await?
//         }
//     };
    Ok(())
}
