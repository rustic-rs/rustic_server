use std::path::{Path, PathBuf};

use axum::{extract::Request, response::IntoResponse};
use axum_extra::{headers::Range, TypedHeader};
use axum_range::{KnownSize, Ranged};

use crate::{
    acl::AccessType,
    auth::AuthFromRequest,
    error::{ErrorKind, Result},
    handlers::{
        access_check::check_auth_and_acl,
        file_exchange::{check_name, get_save_file, save_body},
    },
    storage::STORAGE,
    typed_path::{RepositoryConfigPath, TpeKind},
};

/// has_config
/// Interface: HEAD {repo}/config
pub(crate) async fn has_config(
    RepositoryConfigPath { repo }: RepositoryConfigPath,
    auth: AuthFromRequest,
) -> Result<impl IntoResponse> {
    let tpe = TpeKind::Config;
    tracing::debug!("[has_config] repository path: {repo}, tpe: {tpe}");
    let path = std::path::Path::new(&repo);
    check_auth_and_acl(auth.user, tpe, path, AccessType::Read)?;

    let storage = STORAGE.get().unwrap();
    let file = storage.filename(path, tpe.into_str(), None);
    if file.exists() {
        Ok(())
    } else {
        Err(ErrorKind::FileNotFound(repo))
    }
}

/// get_config
/// Interface: GET {repo}/config
pub(crate) async fn get_config(
    RepositoryConfigPath { repo }: RepositoryConfigPath,
    auth: AuthFromRequest,
    range: Option<TypedHeader<Range>>,
) -> Result<impl IntoResponse> {
    let tpe = TpeKind::Config;
    tracing::debug!("[get_config] repository path: {repo}, tpe: {tpe}");

    check_name(tpe, None)?;
    let path = Path::new(&repo);

    check_auth_and_acl(auth.user, tpe, path, AccessType::Read)?;

    let storage = STORAGE.get().unwrap();
    let file = storage.open_file(path, tpe.into_str(), None).await?;

    let body = KnownSize::file(file)
        .await
        .map_err(|err| ErrorKind::GettingFileMetadataFailed(format!("{err:?}")))?;
    let range = range.map(|TypedHeader(range)| range);
    Ok(Ranged::new(range, body).into_response())
}

/// add_config
/// Interface: POST {repo}/config
pub(crate) async fn add_config(
    RepositoryConfigPath { repo }: RepositoryConfigPath,
    auth: AuthFromRequest,
    request: Request,
) -> Result<impl IntoResponse> {
    let tpe = TpeKind::Config;
    tracing::debug!("[add_config] repository path: {repo}, tpe: {tpe}");
    let path = PathBuf::from(&repo);
    let file = get_save_file(auth.user, path, tpe, None).await?;

    let stream = request.into_body().into_data_stream();
    save_body(file, stream).await?;
    Ok(())
}

/// delete_config
/// Interface: DELETE {repo}/config
pub(crate) async fn delete_config(
    RepositoryConfigPath { repo }: RepositoryConfigPath,
    auth: AuthFromRequest,
) -> Result<impl IntoResponse> {
    let tpe = TpeKind::Config;
    tracing::debug!("[delete_config] repository path: {repo}, tpe: {tpe}");

    check_name(tpe, None)?;
    let path = Path::new(&repo);
    check_auth_and_acl(auth.user, tpe, path, AccessType::Append)?;

    let storage = STORAGE.get().unwrap();
    storage.remove_file(path, tpe.into_str(), None).await?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::handlers::file_config::{add_config, delete_config, get_config, has_config};
    use crate::handlers::repository::{create_repository, delete_repository};
    use crate::log::print_request_response;
    use crate::test_helpers::{
        basic_auth_header_value, init_test_environment, request_uri_for_test,
    };
    use axum::http::Method;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use axum::{middleware, Router};
    use axum_extra::routing::{
        RouterExt, // for `Router::typed_*`
    };
    use http_body_util::BodyExt;
    use std::path::PathBuf;
    use std::{env, fs};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_fixture_has_config_passes() {
        init_test_environment();

        // -----------------------
        // NOT CONFIG
        // -----------------------
        let app = Router::new()
            .typed_head(has_config)
            .layer(middleware::from_fn(print_request_response));

        let request = Request::builder()
            .uri("/test_repo/data/config")
            .method(Method::HEAD)
            .header(
                "Authorization",
                basic_auth_header_value("test", Some("test_pw")),
            )
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        // -----------------------
        // HAS CONFIG
        // -----------------------
        let app = Router::new()
            .typed_head(has_config)
            .layer(middleware::from_fn(print_request_response));

        let request = Request::builder()
            .uri("/test_repo/config")
            .method(Method::HEAD)
            .header(
                "Authorization",
                basic_auth_header_value("test", Some("test_pw")),
            )
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_add_delete_config_passes() {
        init_test_environment();

        // -----------------------
        //Start with a clean slate
        // -----------------------
        let repo = "repo_remove_me_2".to_string();
        //Start with a clean slate ...
        let cwd = env::current_dir().unwrap();
        let path = PathBuf::new()
            .join(cwd)
            .join("tests")
            .join("fixtures")
            .join("test_data")
            .join("test_repos")
            .join(&repo);
        if path.exists() {
            fs::remove_dir_all(&path).unwrap();
            assert!(!path.exists());
        }
        tracing::debug!("[test_add_delete_config] repo: {:?}", &path);

        // -----------------------
        // Create a new repository
        // -----------------------
        let repo_name_uri = ["/", &repo, "?create=true"].concat();
        let app = Router::new()
            .typed_post(create_repository)
            .layer(middleware::from_fn(print_request_response));

        let request = request_uri_for_test(&repo_name_uri, Method::POST);
        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        // -----------------------
        // ADD CONFIG
        // -----------------------
        let test_vec = "Fancy Config Content".to_string();
        let uri = ["/", &repo, "/index/config"].concat();
        let body = Body::new(test_vec.clone());

        let app = Router::new()
            .typed_post(add_config)
            .layer(middleware::from_fn(print_request_response));

        let request = Request::builder()
            .uri(&uri)
            .method(Method::POST)
            .header(
                "Authorization",
                basic_auth_header_value("test", Some("test_pw")),
            )
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
            .typed_get(get_config)
            .layer(middleware::from_fn(print_request_response));

        let request = request_uri_for_test(&uri, Method::GET);
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
            .typed_head(has_config)
            .layer(middleware::from_fn(print_request_response));

        let request = request_uri_for_test(&uri, Method::HEAD);
        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        // -----------------------
        // DELETE CONFIG
        // -----------------------
        let app = Router::new()
            .typed_delete(delete_config)
            .layer(middleware::from_fn(print_request_response));

        let request = request_uri_for_test(&uri, Method::DELETE);
        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let conf_pth = path.join("data").join("config");
        assert!(!conf_pth.exists());

        // -----------------------
        // CLEAN UP DELETE REPO
        // -----------------------
        let repo_name_uri = ["/", &repo].concat();
        let app = Router::new()
            .typed_delete(delete_repository)
            .layer(middleware::from_fn(print_request_response));

        let request = request_uri_for_test(&repo_name_uri, Method::POST);
        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert!(!path.exists());
    }
}
