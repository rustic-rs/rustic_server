use std::{
    io,
    path::{Path, PathBuf},
};

use axum::{
    body::Bytes,
    extract::{Path as AxumPath, Request},
    response::IntoResponse,
    BoxError,
};
use axum_extra::{headers::Range, TypedHeader};
use axum_range::{KnownSize, Ranged};
use futures::{Stream, TryStreamExt};
use futures_util::pin_mut;
use tokio::io::AsyncWrite;
use tokio_util::io::StreamReader;

use crate::{
    acl::AccessType,
    auth::AuthFromRequest,
    error::{ErrorKind, Result},
    handlers::{access_check::check_auth_and_acl, file_helpers::Finalizer},
    storage::STORAGE,
};

/// add_file
/// Interface: POST {path}/{type}/{name}
/// Background info: https://github.com/tokio-rs/axum/blob/main/examples/stream-to-file/src/main.rs
/// Future on ranges: https://www.rfc-editor.org/rfc/rfc9110.html#name-partial-put
pub(crate) async fn add_file(
    AxumPath((path, tpe, name)): AxumPath<(Option<String>, String, String)>,
    auth: AuthFromRequest,
    request: Request,
) -> Result<impl IntoResponse> {
    tracing::debug!("[get_file] path: {path:?}, tpe: {tpe}, name: {name}");
    let path_str = path.unwrap_or_default();

    //credential & access check executed in get_save_file()
    let path = std::path::PathBuf::from(&path_str);
    let file = get_save_file(auth.user, path, tpe.as_str(), name).await?;

    let stream = request.into_body().into_data_stream();
    save_body(file, stream).await?;

    //FIXME: Do we need to check if the file exists here? (For now it seems we should get an error if NOK)
    Ok(())
}

/// delete_file
/// Interface: DELETE {path}/{type}/{name}
pub(crate) async fn delete_file(
    AxumPath((path, tpe, name)): AxumPath<(Option<String>, String, String)>,
    auth: AuthFromRequest,
) -> Result<impl IntoResponse> {
    tracing::debug!("[delete_file] path: {path:?}, tpe: {tpe}, name: {name}");
    let path_str = path.unwrap_or_default();
    let path = Path::new(&path_str);

    check_name(tpe.as_str(), name.as_str())?;
    check_auth_and_acl(auth.user, tpe.as_str(), path, AccessType::Append)?;

    let storage = STORAGE.get().unwrap();

    if let Err(e) = storage.remove_file(path, tpe.as_str(), name.as_str()) {
        tracing::debug!("[delete_file] IO error: {e:?}");
        return Err(ErrorKind::RemovingFileFailed(path_str));
    }
    Ok(())
}

/// get_file
/// Interface: GET {path}/{type}/{name}
pub(crate) async fn get_file(
    AxumPath((path, tpe, name)): AxumPath<(Option<String>, String, String)>,
    auth: AuthFromRequest,
    range: Option<TypedHeader<Range>>,
) -> Result<impl IntoResponse> {
    tracing::debug!("[get_file] path: {path:?}, tpe: {tpe}, name: {name}");

    check_name(tpe.as_str(), name.as_str())?;
    let path_str = path.unwrap_or_default();
    let path = Path::new(&path_str);

    check_auth_and_acl(auth.user, tpe.as_str(), path, AccessType::Read)?;

    let storage = STORAGE.get().unwrap();
    let file = match storage.open_file(path, &tpe, &name).await {
        Ok(file) => file,
        Err(_) => {
            return Err(ErrorKind::FileNotFound(path_str));
        }
    };

    let body = KnownSize::file(file).await.unwrap();
    let range = range.map(|TypedHeader(range)| range);
    Ok(Ranged::new(range, body).into_response())
}

//==============================================================================
// Support functions:
//
//==============================================================================

/// Returns a stream for the given path in the repository.
pub(crate) async fn get_save_file(
    user: String,
    path: PathBuf,
    tpe: &str,
    name: String,
) -> Result<impl AsyncWrite + Unpin + Finalizer> {
    tracing::debug!("[get_save_file] path: {path:?}, tpe: {tpe}, name: {name}");

    check_name(tpe, name.as_str())?;
    check_auth_and_acl(user, tpe, path.as_path(), AccessType::Append)?;

    let storage = STORAGE.get().unwrap();
    let file_writer = match storage.create_file(&path, tpe, &name).await {
        Ok(w) => w,
        Err(_) => {
            return Err(ErrorKind::GettingFileHandleFailed);
        }
    };

    Ok(file_writer)
}

/// saves the content in the HTML request body to a file stream.
pub(crate) async fn save_body<S, E>(
    mut write_stream: impl AsyncWrite + Unpin + Finalizer,
    stream: S,
) -> Result<impl IntoResponse>
where
    S: Stream<Item = std::result::Result<Bytes, E>>,
    E: Into<BoxError>,
{
    // Convert the stream into an `AsyncRead`.
    let body_with_io_error = stream.map_err(|err| io::Error::new(io::ErrorKind::Other, err));
    let body_reader = StreamReader::new(body_with_io_error);
    pin_mut!(body_reader);
    let byte_count = match tokio::io::copy(&mut body_reader, &mut write_stream).await {
        Ok(b) => b,
        Err(err) => return Err(ErrorKind::FinalizingFileFailed(format!("{:?}", err))),
    };

    tracing::debug!("[file written] bytes: {byte_count}");
    write_stream
        .finalize()
        .await
        .map_err(|err| ErrorKind::FinalizingFileFailed(format!("Could not finalize file: {}", err)))
}

#[cfg(test)]
fn check_string_sha256(_name: &str) -> bool {
    true
}

#[cfg(not(test))]
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

///FIXME Move to support functoin file
pub(crate) fn check_name(tpe: &str, name: &str) -> Result<impl IntoResponse> {
    match tpe {
        "config" => Ok(()),
        _ if check_string_sha256(name) => Ok(()),
        _ => Err(ErrorKind::FilenameNotAllowed(name.to_string())),
    }
}

#[cfg(test)]
mod test {
    use crate::handlers::file_exchange::{add_file, delete_file, get_file};
    use crate::log::print_request_response;
    use crate::test_helpers::{
        basic_auth_header_value, init_test_environment, request_uri_for_test,
    };
    use axum::http::{header, Method};
    use axum::routing::{delete, get, put};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use axum::{middleware, Router};
    use http_body_util::BodyExt;
    use std::path::PathBuf;
    use std::{env, fs};
    use tower::ServiceExt;

    #[tokio::test]
    async fn server_add_delete_file_tester() {
        init_test_environment();

        let file_name = "__add_file_test_adds_this_one__";

        //Start with a clean slate ...
        let cwd = env::current_dir().unwrap();
        let path = PathBuf::new()
            .join(cwd)
            .join("tests")
            .join("fixtures")
            .join("test_data")
            .join("test_repos")
            .join("test_repo")
            .join("keys")
            .join(file_name);
        if path.exists() {
            fs::remove_file(&path).unwrap();
            assert!(!path.exists());
        }

        //----------------------------------------------
        // Write a complete file
        //----------------------------------------------
        let app = Router::new()
            .route("/*path", put(add_file))
            .layer(middleware::from_fn(print_request_response));

        let test_vec = "Hello World".to_string();
        let body = Body::new(test_vec.clone());
        let uri = ["/test_repo/keys/", file_name].concat();
        let request = Request::builder()
            .uri(uri)
            .method(Method::PUT)
            .header(
                "Authorization",
                basic_auth_header_value("test", Some("test_pw")),
            )
            .body(body)
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert!(path.exists());

        let body = fs::read_to_string(&path).unwrap();
        assert_eq!(body, test_vec);

        //----------------------------------------------
        // Delete a complete file
        //----------------------------------------------
        let app = Router::new()
            .route("/*path", delete(delete_file))
            .layer(middleware::from_fn(print_request_response));

        let uri = ["/test_repo/keys/", file_name].concat();
        let request = Request::builder()
            .uri(uri)
            .method(Method::DELETE)
            .header(
                "Authorization",
                basic_auth_header_value("test", Some("test_pw")),
            )
            .body(body)
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert!(!path.exists());

        // // Just to be sure ...
        // fs::remove_file(&path).unwrap();
        // assert!( !path.exists() );
    }

    #[tokio::test]
    async fn server_get_file_tester() {
        init_test_environment();

        let file_name = "__get_file_test_adds_this_two__";
        //Start with a clean slate ...
        let cwd = env::current_dir().unwrap();
        let path = PathBuf::new()
            .join(cwd)
            .join("tests")
            .join("fixtures")
            .join("test_data")
            .join("test_repos")
            .join("test_repo")
            .join("keys")
            .join(file_name);
        if path.exists() {
            tracing::debug!("[server_get_file_tester] test file found and removed");
            fs::remove_file(&path).unwrap();
            assert!(!path.exists());
        }

        // Start with creating the file before we can test
        let app = Router::new()
            .route("/*path", put(add_file))
            .layer(middleware::from_fn(print_request_response));

        let test_vec = "Hello Sweet World".to_string();
        let body = Body::new(test_vec.clone());
        let uri = ["/test_repo/keys/", file_name].concat();
        let request = Request::builder()
            .uri(uri)
            .method(Method::PUT)
            .header(
                "Authorization",
                basic_auth_header_value("test", Some("test_pw")),
            )
            .body(body)
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert!(path.exists());
        let body = fs::read_to_string(&path).unwrap();
        assert_eq!(body, test_vec);

        // Now we can start to test
        //----------------------------------------
        // Fetch the complete file
        //----------------------------------------
        let app = Router::new()
            .route("/*path", get(get_file))
            .layer(middleware::from_fn(print_request_response));

        let uri = ["/test_repo/keys/", file_name].concat();
        let request = request_uri_for_test(&uri, Method::GET);
        let resp = app.clone().oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let (_parts, body) = resp.into_parts();
        let byte_vec = body.collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(byte_vec.to_vec()).unwrap();
        assert_eq!(body_str, test_vec);

        //----------------------------------------
        // Read a partial file
        //----------------------------------------
        //  let test_vec = "Hello Sweet World".to_string();

        let uri = ["/test_repo/keys/", file_name].concat();
        let request = Request::builder()
            .uri(uri)
            .method(Method::GET)
            .header(header::RANGE, "bytes=6-12")
            .header(
                "Authorization",
                basic_auth_header_value("test", Some("test_pw")),
            )
            .body(Body::empty())
            .unwrap();

        let resp = app.clone().oneshot(request).await.unwrap();

        let test_vec = "Sweet W".to_string(); // bytes 6 - 13 from in the file

        assert_eq!(resp.status(), StatusCode::PARTIAL_CONTENT);
        let (_parts, body) = resp.into_parts();
        let byte_vec = body.collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(byte_vec.to_vec()).unwrap();
        assert_eq!(body_str, test_vec);

        //----------------------------------------------
        // Clean up -> Delete test file
        //----------------------------------------------
        // fs::remove_file(&path).unwrap();
        // assert!( !path.exists() );
        let app = Router::new()
            .route("/*path", delete(delete_file))
            .layer(middleware::from_fn(print_request_response));

        let uri = ["/test_repo/keys/", file_name].concat();
        let request = request_uri_for_test(&uri, Method::DELETE);
        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn test_get_config() {
        init_test_environment();

        let cwd = env::current_dir().unwrap();
        let path = PathBuf::new()
            .join(cwd)
            .join("tests")
            .join("fixtures")
            .join("test_data")
            .join("test_repos")
            .join("test_repo")
            .join("config");
        let test_vec = fs::read(path).unwrap();

        let app = Router::new()
            .route("/*path", get(get_file))
            .layer(middleware::from_fn(print_request_response));

        let uri = "/test_repo/config";
        let request = request_uri_for_test(uri, Method::GET);
        let resp = app.clone().oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let (_parts, body) = resp.into_parts();
        let byte_vec = body.collect().await.unwrap().to_bytes();
        let body_str = byte_vec.to_vec();
        assert_eq!(body_str, test_vec);
    }
}
