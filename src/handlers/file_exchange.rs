use axum::{
    body::Bytes,
    extract::{Path as PathExtract},
    extract::{Request},
    http::StatusCode,
    response::{Html, Redirect,IntoResponse},
    routing::{get, post},
    BoxError, Router,
};
use futures_util::pin_mut;
use ::futures::{Stream, TryStreamExt};
use std::io;
use std::io::SeekFrom::Start;
use std::path::{Path, PathBuf};
use axum::body::Body;
use axum::http::{header, HeaderMap};
use axum::response::AppendHeaders;
use axum_auth::AuthBasic;
use http_range::HttpRange;
use tokio::io::{AsyncSeekExt, AsyncWrite};
use tokio_util::io::{ReaderStream, StreamReader};
use crate::error::ErrorKind;
use crate::handlers::path_analysis::{ArchivePathEnum, decompose_path};
use crate::helpers::Finalizer;
use crate::storage::{STORAGE};
use crate::web::{check_auth_and_acl, DEFAULT_PATH};
use crate::{
    acl::{AccessType},
    error::{Result},
};

//==============================================================================
// add_file
// Interface: POST {path}/{type}/{name}
// Background info: https://github.com/tokio-rs/axum/blob/main/examples/stream-to-file/src/main.rs
//==============================================================================

async fn add_file(
    AuthBasic((user, _password)): AuthBasic,
    path: Option<PathExtract<String>>,
    request: Request,
)    -> Result<impl IntoResponse>
{
    let path_string = path.map_or(DEFAULT_PATH.to_string(), |PathExtract(path_ext)| path_ext);
    let archive_path = decompose_path(path_string);
    let p_str = archive_path.path;
    let tpe = archive_path.tpe;
    let name = archive_path.name;
    assert_ne!( archive_path.path_type, ArchivePathEnum::CONFIG);
    assert_ne!( &name, "");
    tracing::debug!("[get_file] path: {p_str}, tpe: {tpe}, name: {name}");

    //credential & access check executed in get_save_file()
    let pth = PathBuf::new().join(&p_str);
    let file = get_save_file(user, pth, tpe.as_str(), name).await?;

    let stream = request.into_body().into_data_stream();
    save_body(file, stream).await?;

    //FIXME: Do we need to check if the file exists here? (For now it seems we should get an error if NOK)
    Ok(())
}

//==============================================================================
// get_file
// Interface: GET {path}/{type}/{name}
//==============================================================================

async fn get_file(
    AuthBasic((user, _password)): AuthBasic,
    path: Option<PathExtract<String>>,
    headers: HeaderMap,
)  -> Result<impl IntoResponse>
{
    let path_string = path.map_or(DEFAULT_PATH.to_string(), |PathExtract(path_ext)| path_ext);
    let archive_path = decompose_path(path_string);
    let p_str = archive_path.path;
    let tpe = archive_path.tpe;
    let name = archive_path.name;
    assert_ne!( archive_path.path_type, ArchivePathEnum::CONFIG);
    assert_ne!( &name, "");
    tracing::debug!("[get_file] path: {p_str}, tpe: {tpe}, name: {name}");

    check_name(tpe.as_str(), name.as_str())?;
    let pth = Path::new(&p_str);

    check_auth_and_acl(user, tpe.as_str(), pth, AccessType::Read)?;

    let storage = STORAGE.get().unwrap();
    let mut file = match storage.open_file(&pth, &tpe, &name).await {
        Ok(file) => {file}
        Err(_) => {
            return Err(ErrorKind::FileNotFound(p_str));
        }
    };

    let mut len = match file.metadata().await {
        Ok(val) => val.len(),
        Err(_) => {
            return Err(ErrorKind::GettingFileMetadataFailed);
        }
    };

    let mut status = StatusCode::OK;
    if let Some(header_value) = headers.get(header::RANGE) {
        let header_value = match header_value.to_str() {
            Ok(val) => val,
            Err(_) => return Err(ErrorKind::RangeNotValid)
        };
        match HttpRange::parse(header_value, len, ) {
            Ok(range) if range.len() == 1 => {
                if file.seek(Start(range[0].start)).await.is_err() {
                    return Err(ErrorKind::SeekingFileFailed);
                };
                len = range[0].length;
                status = StatusCode::PARTIAL_CONTENT;
            }
            Ok(_) => return Err(ErrorKind::MultipartRangeNotImplemented),
            Err(_) => return Err(ErrorKind::GeneralRange),
        }
    };


    // From: https://github.com/tokio-rs/axum/discussions/608#discussioncomment-1789020
    let stream = ReaderStream::with_capacity(
        file,
        match len.try_into() {
            Ok(val) => val,
            Err(_) => return Err(ErrorKind::ConversionToU64Failed),
        },
    );

    let body = Body::from_stream(stream);

    let headers = AppendHeaders([(header::CONTENT_TYPE, "application/octet-stream")]);
    Ok((status, headers, body))
}

//==============================================================================
// Support functions:
//
//==============================================================================

async fn get_save_file(user:String, path: PathBuf, tpe: &str, name: String, )
                       -> Result<impl AsyncWrite + Unpin + Finalizer>
{
    tracing::debug!("[get_save_file] path: {path:?}, tpe: {tpe}, name: {name}");

    if check_name(tpe, name.as_str()).is_err() {
        return Err(ErrorKind::FilenameNotAllowed(name));
    }

    if check_auth_and_acl(user, tpe, path.as_path(), AccessType::Append).is_err() {
        return Err(ErrorKind::PathNotAllowed(path.display().to_string()));
    }

    let storage = STORAGE.get().unwrap();
    let file_writer = match storage.create_file(&path, tpe, &name).await{
        Ok(w) => w,
        Err(_) => {
            return Err(ErrorKind::GettingFileHandleFailed);
        }
    };

    Ok(file_writer)
}

async fn save_body<S, E>(mut file: impl AsyncWrite + Unpin + Finalizer, stream: S, )
                         -> Result<impl IntoResponse>
    where
        S: Stream<Item = std::result::Result<Bytes, E>>,
        E: Into<BoxError>,
{
    // Convert the stream into an `AsyncRead`.
    let body_with_io_error = stream
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err));
    let body_reader = StreamReader::new(body_with_io_error);
    pin_mut!(body_reader);
    let byte_count = match tokio::io::copy(&mut body_reader, &mut file).await {
        Ok(b) => {b},
        Err(_) => return Err(ErrorKind::FinalizingFileFailed)
    };

    tracing::debug!("[file written] bytes: {byte_count}");
    if file.finalize().await.is_err() {
        return Err(ErrorKind::FinalizingFileFailed);
    };

    Ok(())
}

#[cfg(test)]
fn check_string_sha256(name: &str) -> bool {true}

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

fn check_name(tpe: &str, name: &str) -> Result<impl IntoResponse> {
    match tpe {
        "config" => Ok(()),
        _ if check_string_sha256(name) => Ok(()),
        _ => Err(ErrorKind::FilenameNotAllowed(name.to_string())),
    }
}

#[cfg(test)]
mod test {
    use std::{env, fs};
    use std::path::PathBuf;
    use http_body_util::{BodyExt, StreamBody};
    use axum::{ middleware, Router};
    use axum::routing::{put, get,};
    use crate::test_server::{basic_auth, init_test_environment, print_request_response};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use axum::http::{header, Method};
    use axum::http::header::{ACCEPT, CONTENT_TYPE};
    use tower::{ServiceExt};
    use crate::handlers::file_exchange::{add_file, get_file};

    #[tokio::test]
    async fn server_add_file_tester() {
        init_test_environment();

        let file_name = "__add_file_test_adds_this_one__";

        //Start with a clean slate ...
        let cwd = env::current_dir().unwrap();
        let path = PathBuf::new()
            .join(cwd)
            .join("test_data")
            .join("test_repos")
            .join( "test_repo")
            .join( "keys" )
            .join( file_name );
        if path.exists() {
            fs::remove_file(&path).unwrap();
            assert!(!path.exists());
        }

        // Write a complete file
        let app = Router::new()
            .route( "/*path",put(add_file) )
            .layer(middleware::from_fn(print_request_response));

        let test_vec = "Hello World".to_string();
        let body = Body::new( test_vec.clone() );
        let uri = ["/test_repo/keys/", file_name].concat();
        let request = Request::builder()
            .uri(uri)
            .method(Method::PUT)
            .header("Authorization",  basic_auth("test", Some("test_pw")))
            .body(body).unwrap();

        let resp = app
            .oneshot(request)
            .await
            .unwrap();

        assert_eq!( resp.status(), StatusCode::OK );
        assert!( path.exists() );

        let body = fs::read_to_string(&path).unwrap();
        assert_eq!( body, test_vec );

        fs::remove_file(&path).unwrap();
        assert!( !path.exists() );
    }

    #[tokio::test]
    async fn server_get_file_tester() {
        init_test_environment();

        let file_name = "__add_file_test_adds_this_two__";
        //Start with a clean slate ...
        let cwd = env::current_dir().unwrap();
        let path = PathBuf::new()
            .join(cwd)
            .join("test_data")
            .join("test_repos")
            .join( "test_repo")
            .join( "keys" )
            .join( file_name );
        if path.exists() {
            fs::remove_file(&path).unwrap();
            assert!(!path.exists());
        }

        // Start with creating the file before we can test
        let app = Router::new()
            .route( "/*path",put(add_file) )
            .layer(middleware::from_fn(print_request_response));

        let test_vec = "Hello World".to_string();
        let body = Body::new( test_vec.clone() );
        let uri = ["/test_repo/keys/", file_name].concat();
        let request = Request::builder()
            .uri(uri)
            .method(Method::PUT)
            .header("Authorization",  basic_auth("test", Some("test_pw")))
            .body(body).unwrap();

        let resp = app
            .oneshot(request)
            .await
            .unwrap();

        assert_eq!( resp.status(), StatusCode::OK );
        assert!( path.exists() );
        let body = fs::read_to_string(&path).unwrap();
        assert_eq!( body, test_vec );


        // Now we can start to test
        // Fetch the complete file
        let app = Router::new()
            .route( "/*path",get(get_file) )
            .layer(middleware::from_fn(print_request_response));

        let request = Request::builder()
            .uri("/test_repo/keys/__add_file_test_adds_this_two__")
            .method(Method::GET)
            .header("Authorization",  basic_auth("test", Some("test_pw")))
            .body(body).unwrap();

        let resp = app.clone()
            .oneshot(request)
            .await
            .unwrap();

        assert_eq!( resp.status(), StatusCode::OK );
        let (_parts, body) = resp.into_parts() ;
        let byte_vec = body.collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(byte_vec.to_vec()).unwrap();
        assert_eq!(body_str, test_vec);

        // // Read a partial file
        // let test_vec = "Hello SWEET World".to_string();
        // let write_vec = "SWEET World".to_string();
        // let body = Body::new( write_vec.clone() );
        //
        // let request = Request::builder()
        //     .uri("/test_repo/keys/__add_file_test_adds_this_one__")
        //     .method(Method::GET)
        //     .header("Authorization",  basic_auth("test", Some("test_pw")))
        //     .body(body).unwrap();
        //

        
        // Cleaning up
        fs::remove_file(&path).unwrap();
        assert!( !path.exists() );


    }
}