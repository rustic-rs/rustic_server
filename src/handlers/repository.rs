use std::path::Path;

use axum::{
    extract::{Path as AxumPath, Query},
    http::StatusCode,
    response::IntoResponse,
};
use serde_derive::Deserialize;

use crate::{
    acl::AccessType,
    auth::AuthFromRequest,
    error::Result,
    handlers::{access_check::check_auth_and_acl, path_analysis::constants::TYPES},
    storage::STORAGE,
};

/// Create_repository
/// Interface: POST {path}?create=true
#[derive(Default, Deserialize)]
#[serde(default)]
pub(crate) struct Create {
    create: bool,
}

pub(crate) async fn create_repository(
    AxumPath((path, tpe)): AxumPath<(Option<String>, String)>,
    auth: AuthFromRequest,
    Query(params): Query<Create>,
) -> Result<impl IntoResponse> {
    tracing::debug!("[create_repository] repo_path: {path:?}");
    let path = path.unwrap_or_default();
    let path = Path::new(&path);
    //FIXME: Is Append the right access leven, or should we require Modify?
    check_auth_and_acl(auth.user, &tpe, path, AccessType::Append)?;

    let storage = STORAGE.get().unwrap();
    match params.create {
        true => {
            for tpe_i in TYPES.iter() {
                storage.create_dir(path, tpe_i)?
            }

            Ok((
                StatusCode::OK,
                format!("Called create_files with path {:?}\n", path),
            ))
        }
        false => Ok((
            StatusCode::OK,
            format!("Called create_files with path {:?}, create=false\n", path),
        )),
    }
}

/// Delete_repository
/// Interface: Delete {path}
/// FIXME: The input path should at least NOT point to a file in any repository
pub(crate) async fn delete_repository(
    AxumPath(path): AxumPath<Option<String>>,
    auth: AuthFromRequest,
) -> Result<impl IntoResponse> {
    tracing::debug!("[delete_repository] repo_path: {path:?}");
    let path_str = path.unwrap_or_default();
    let path = Path::new(&path_str);
    //FIXME: We surely need modify access to delete right??
    check_auth_and_acl(auth.user, "", path, AccessType::Modify)?;

    let storage = STORAGE.get().unwrap();
    storage.remove_repository(path)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::handlers::repository::{create_repository, delete_repository};
    use crate::log::print_request_response;
    use crate::test_helpers::{
        basic_auth_header_value, init_test_environment, request_uri_for_test,
    };
    use axum::http::Method;
    use axum::routing::{delete, post};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use axum::{middleware, Router};
    use std::path::PathBuf;
    use std::{env, fs};
    use tower::ServiceExt;

    /// The acl.toml test allows the create of "repo_remove_me"
    /// for user test with the correct password
    #[tokio::test]
    async fn repo_create_delete_test() {
        init_test_environment();

        //Start with a clean slate ...
        let cwd = env::current_dir().unwrap();
        let path = PathBuf::new()
            .join(cwd)
            .join("tests")
            .join("fixtures")
            .join("test_data")
            .join("test_repos")
            .join("repo_remove_me");
        if path.exists() {
            fs::remove_dir_all(&path).unwrap();
            assert!(!path.exists());
        }

        let cwd = env::current_dir().unwrap();
        let not_allowed_path = PathBuf::new()
            .join(cwd)
            .join("tests")
            .join("fixtures")
            .join("test_data")
            .join("test_repos")
            .join("repo_not_allowed");
        if not_allowed_path.exists() {
            fs::remove_dir_all(&not_allowed_path).unwrap();
            assert!(!not_allowed_path.exists());
        }

        // ------------------------------------
        // Create a new repository: {path}?create=true
        // ------------------------------------
        let repo_name_uri = "/repo_remove_me?create=true".to_string();
        let app = Router::new()
            .route("/*path", post(create_repository))
            .layer(middleware::from_fn(print_request_response));

        let request = request_uri_for_test(&repo_name_uri, Method::POST);
        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert!(path.exists());

        // ------------------------------------------
        // Create a new repository WITHOUT ACL access
        // ------------------------------------------
        let repo_name_uri = "/repo_not_allowed?create=true".to_string();
        let app = Router::new()
            .route("/*path", post(create_repository))
            .layer(middleware::from_fn(print_request_response));

        let request = request_uri_for_test(&repo_name_uri, Method::POST);
        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        assert!(!not_allowed_path.exists());

        // ------------------------------------------
        // Delete a repository WITHOUT ACL access
        // ------------------------------------------
        let repo_name_uri = "/repo_remove_me?create=true".to_string();
        let app = Router::new()
            .route("/*path", post(delete_repository))
            .layer(middleware::from_fn(print_request_response));

        let request = Request::builder()
            .uri(&repo_name_uri)
            .method(Method::POST)
            .header(
                "Authorization",
                basic_auth_header_value("test", Some("__wrong_password__")),
            )
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        assert!(path.exists());

        // ------------------------------------------
        // Delete a repository WITH access...
        // ------------------------------------------
        assert!(path.exists()); // pre condition: repo exists
        let repo_name_uri = "/repo_remove_me".to_string();
        let app = Router::new()
            .route("/*path", delete(delete_repository))
            .layer(middleware::from_fn(print_request_response));

        let request = request_uri_for_test(&repo_name_uri, Method::DELETE);
        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert!(!path.exists());
    }
}
