use crate::acl::AccessType;
use crate::auth::AuthFromRequest;
use crate::error::ErrorKind;
use crate::error::Result;
use crate::handlers::access_check::check_auth_and_acl;
use crate::handlers::file_exchange::{check_name, get_file, get_save_file, save_body};
use crate::handlers::path_analysis::{decompose_path, ArchivePathEnum, DEFAULT_PATH};
use crate::storage::STORAGE;
use axum::extract::Request;
use axum::http::HeaderMap;
use axum::{extract::Path as PathExtract, response::IntoResponse};
use axum_extra::headers::Range;
use axum_extra::TypedHeader;
use std::path::{Path, PathBuf};

//==============================================================================
// has_config
// Interface: HEAD {path}/config
//==============================================================================
pub(crate) async fn has_config(
    auth: AuthFromRequest,
    path: Option<PathExtract<String>>,
) -> Result<impl IntoResponse> {
    let path_string = path.map_or(DEFAULT_PATH.to_string(), |PathExtract(path_ext)| path_ext);
    let archive_path = decompose_path(path_string.clone())?;
    tracing::debug!("[has_config] archive_path: {}", &archive_path);
    let p_str = archive_path.path;
    let tpe = archive_path.tpe;
    let name = archive_path.name;
    // assert_eq!( &archive_path.path_type, &ArchivePathEnum::CONFIG); --> Correct assert, but interferes with our tests
    // assert_eq!( &tpe, TPE_CONFIG); --> Correct assert, but interferes with our tests
    assert_eq!(&name, "config");
    tracing::debug!("[has_config] path: {p_str}, tpe: {tpe}, name: {name}");

    let path = Path::new(&p_str);
    check_auth_and_acl(auth.user, tpe.as_str(), path, AccessType::Read)?;

    let storage = STORAGE.get().unwrap();
    let file = storage.filename(path, &tpe, &name);
    if file.exists() {
        Ok(())
    } else {
        Err(ErrorKind::FileNotFound(p_str))
    }
}

//==============================================================================
// get_config
// Interface: GET {path}/config
//==============================================================================

pub(crate) async fn get_config(
    auth: AuthFromRequest,
    path: Option<PathExtract<String>>,
    range: Option<TypedHeader<Range>>,
) -> Result<impl IntoResponse> {
    // let path_string = path.map_or(DEFAULT_PATH.to_string(), |PathExtract(path_ext)| path_ext);
    // let archive_path = decompose_path(path_string.clone());
    // let p_str = archive_path.path;
    // let tpe = archive_path.tpe;
    // assert_eq!( &archive_path.path_type, &ArchivePathEnum::CONFIG);
    // assert_eq!( &tpe, TPE_CONFIG);
    // tracing::debug!("[get_config] path: {p_str:?}, tpe: {tpe}, name: config");

    return get_file(auth, path, range).await;
}

//==============================================================================
// add_config
// Interface: POST {path}/config
//==============================================================================

pub(crate) async fn add_config(
    auth: AuthFromRequest,
    path: Option<PathExtract<String>>,
    request: Request,
) -> Result<impl IntoResponse> {
    let path_string = path.map_or(DEFAULT_PATH.to_string(), |PathExtract(path_ext)| path_ext);
    let archive_path = decompose_path(path_string.clone())?;
    let p_str = archive_path.path;
    let tpe = archive_path.tpe;
    let name = archive_path.name;
    assert_eq!(&archive_path.path_type, &ArchivePathEnum::CONFIG);
    assert_eq!(&name, "config");
    tracing::debug!("[add_config] path: {p_str}, tpe: {tpe}, name: {name}");

    let pth = PathBuf::new().join(&p_str);
    let file = get_save_file(auth.user, pth, tpe.as_str(), name).await?;

    let stream = request.into_body().into_data_stream();
    save_body(file, stream).await?;
    Ok(())
}

//==============================================================================
// delete_config
// Interface: DELETE {path}/config
// FIXME: The original restic spec does not define delete_config --> but rustic did ??
//==============================================================================

//#[debug_handler]
pub(crate) async fn delete_config(
    auth: AuthFromRequest,
    path: Option<PathExtract<String>>,
) -> Result<impl IntoResponse> {
    let path_string = path.map_or(DEFAULT_PATH.to_string(), |PathExtract(path_ext)| path_ext);
    let archive_path = decompose_path(path_string.clone())?;
    let p_str = archive_path.path;
    let tpe = archive_path.tpe;
    let name = archive_path.name;
    assert_eq!(&archive_path.path_type, &ArchivePathEnum::CONFIG);
    tracing::debug!("[delete_config] path: {p_str}, tpe: {tpe}, name: {name}");

    check_name(tpe.as_str(), &name)?;
    let path = Path::new(&p_str);
    let pth = Path::new(path);
    check_auth_and_acl(auth.user, tpe.as_str(), pth, AccessType::Append)?;

    let storage = STORAGE.get().unwrap();
    if storage.remove_file(path, tpe.as_str(), &name).is_err() {
        return Err(ErrorKind::RemovingFileFailed(p_str));
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::handlers::file_config::{add_config, delete_config, get_config, has_config};
    use crate::handlers::repository::{create_repository, delete_repository};
    use crate::test_server::{basic_auth, init_test_environment, print_request_response};
    use axum::http::Method;
    use axum::routing::{delete, get, head, post};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use axum::{middleware, Router};
    use axum_extra::headers::Range;
    use axum_extra::TypedHeader;
    use http_body_util::BodyExt;
    use std::path::PathBuf;
    use std::{env, fs};
    use tower::ServiceExt;

    #[tokio::test]
    async fn tester_has_config() {
        init_test_environment();

        // -----------------------
        // NOT CONFIG
        // -----------------------
        let app = Router::new()
            .route("/*path", head(has_config))
            .layer(middleware::from_fn(print_request_response));

        let request = Request::builder()
            .uri("/test_repo/data/config")
            .method(Method::HEAD)
            .header("Authorization", basic_auth("test", Some("test_pw")))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        // -----------------------
        // HAS CONFIG
        // -----------------------
        let app = Router::new()
            .route("/*path", head(has_config))
            .layer(middleware::from_fn(print_request_response));

        let request = Request::builder()
            .uri("/test_repo/config")
            .method(Method::HEAD)
            .header("Authorization", basic_auth("test", Some("test_pw")))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_add_delete_config() {
        init_test_environment();

        // -----------------------
        //Start with a clean slate
        // -----------------------
        let repo = "repo_remove_me_2".to_string();
        //Start with a clean slate ...
        let cwd = env::current_dir().unwrap();
        let path = PathBuf::new()
            .join(cwd)
            .join("test_data")
            .join("test_repos")
            .join(&repo);
        if path.exists() {
            fs::remove_dir_all(&path).unwrap();
            assert!(!path.exists());
        }

        // -----------------------
        // Create a new repository
        // -----------------------
        let repo_name_uri = ["/", &repo, "?create=true"].concat();
        let app = Router::new()
            .route("/*path", post(create_repository))
            .layer(middleware::from_fn(print_request_response));

        let request = Request::builder()
            .uri(&repo_name_uri)
            .method(Method::POST)
            .header("Authorization", basic_auth("test", Some("test_pw")))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        // -----------------------
        // ADD CONFIG
        // -----------------------
        let test_vec = "Fancy Config Content".to_string();
        let uri = ["/", &repo, "/index/config"].concat();
        let body = Body::new(test_vec.clone());

        let app = Router::new()
            .route("/*path", post(add_config))
            .layer(middleware::from_fn(print_request_response));

        let request = Request::builder()
            .uri(&uri)
            .method(Method::POST)
            .header("Authorization", basic_auth("test", Some("test_pw")))
            .body(body)
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let conf_pth = path.join("index").join("config");
        assert!(conf_pth.exists());
        let conf_str = fs::read_to_string(conf_pth).unwrap();
        assert_eq!(&conf_str, &test_vec);

        // -----------------------
        // GET CONFIG
        // -----------------------
        let app = Router::new()
            .route("/*path", get(get_config))
            .layer(middleware::from_fn(print_request_response));

        let request = Request::builder()
            .uri(&uri)
            .method(Method::GET)
            .header("Authorization", basic_auth("test", Some("test_pw")))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let (_parts, body) = resp.into_parts();
        let byte_vec = body.collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(byte_vec.to_vec()).unwrap();
        assert_eq!(body_str, test_vec);

        // -----------------------
        // HAS CONFIG
        // - differs from tester_has_config() that we have a non empty path now
        // -----------------------
        let app = Router::new()
            .route("/*path", head(has_config))
            .layer(middleware::from_fn(print_request_response));

        let request = Request::builder()
            .uri(&uri)
            .method(Method::HEAD)
            .header("Authorization", basic_auth("test", Some("test_pw")))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        // -----------------------
        // DELETE CONFIG
        // -----------------------
        let app = Router::new()
            .route("/*path", delete(delete_config))
            .layer(middleware::from_fn(print_request_response));

        let request = Request::builder()
            .uri(&uri)
            .method(Method::DELETE)
            .header("Authorization", basic_auth("test", Some("test_pw")))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let conf_pth = path.join("data").join("config");
        assert!(!conf_pth.exists());

        // -----------------------
        // CLEAN UP DELETE REPO
        // -----------------------
        let repo_name_uri = ["/", &repo].concat();
        let app = Router::new()
            .route("/*path", post(delete_repository))
            .layer(middleware::from_fn(print_request_response));

        let request = Request::builder()
            .uri(&repo_name_uri)
            .method(Method::POST)
            .header("Authorization", basic_auth("test", Some("test_pw")))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert!(!path.exists());
    }
}
