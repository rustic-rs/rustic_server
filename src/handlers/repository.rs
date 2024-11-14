use std::path::PathBuf;

use axum::{extract::Query, http::StatusCode, response::IntoResponse};
use serde_derive::Deserialize;

use crate::{
    acl::AccessType, auth::AuthFromRequest, error::ApiResult,
    handlers::access_check::check_auth_and_acl, storage::STORAGE, typed_path::TpeKind,
};

// used for using auto-generated TpeKind variant names
use crate::typed_path::PathParts;
use strum::VariantNames;

/// `Create_repository`
/// Interface: POST {path}?create=true
#[derive(Default, Deserialize)]
#[serde(default)]
pub struct Create {
    create: bool,
}

pub async fn create_repository<P: PathParts>(
    path: P,
    auth: AuthFromRequest,
    Query(params): Query<Create>,
) -> ApiResult<impl IntoResponse> {
    tracing::debug!(
        "[create_repository] repository path: {}",
        path.repo().unwrap()
    );
    let path = PathBuf::new().join(path.repo().unwrap());
    let _ = check_auth_and_acl(auth.user, None, &path, AccessType::Append)?;

    let storage = STORAGE.get().unwrap();
    match params.create {
        true => {
            for tpe in TpeKind::VARIANTS.iter() {
                // config is not a directory, but a file
                // it is handled separately
                if tpe == &TpeKind::Config.into_str() {
                    continue;
                }

                storage.create_dir(&path, Some(tpe)).await?;
            }

            Ok((
                StatusCode::OK,
                format!("Called create_files with path {:?}", &path),
            ))
        }
        false => Ok((
            StatusCode::OK,
            format!("Called create_files with path {:?}, create=false", &path),
        )),
    }
}

/// `Delete_repository`
/// Interface: Delete {path}
// FIXME: The input path should at least NOT point to a file in any repository
pub async fn delete_repository<P: PathParts>(
    path: P,
    auth: AuthFromRequest,
) -> ApiResult<impl IntoResponse> {
    tracing::debug!(
        "[delete_repository] repository path: {}",
        &path.repo().unwrap()
    );
    let path = PathBuf::new().join(path.repo().unwrap());
    let _ = check_auth_and_acl(auth.user, None, &path, AccessType::Modify)?;

    let storage = STORAGE.get().unwrap();
    storage.remove_repository(&path).await?;

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::log::print_request_response;
    use crate::test_helpers::{
        basic_auth_header_value, init_test_environment, request_uri_for_test,
    };
    use crate::typed_path::RepositoryPath;
    use crate::{
        handlers::repository::{create_repository, delete_repository},
        test_helpers::server_config,
    };
    use axum::http::Method;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use axum::{middleware, Router};
    use axum_extra::routing::RouterExt;
    use std::path::PathBuf;
    use tokio::fs;
    use tower::ServiceExt;

    /// The acl.toml test allows the create of "repo_remove_me"
    /// for user test with the correct password
    #[tokio::test]
    async fn test_repo_create_delete_passes() {
        init_test_environment(server_config());

        //Start with a clean slate ...
        let path = PathBuf::new()
            .join("tests")
            .join("generated")
            .join("test_storage")
            .join("repo_remove_me");

        if path.exists() {
            fs::remove_dir_all(&path).await.unwrap();
            assert!(!path.exists());
        }

        let not_allowed_path = PathBuf::new()
            .join("tests")
            .join("generated")
            .join("test_storage")
            .join("repo_not_allowed");

        if not_allowed_path.exists() {
            fs::remove_dir_all(&not_allowed_path).await.unwrap();
            assert!(!not_allowed_path.exists());
        }

        // ------------------------------------
        // Create a new repository: {path}?create=true
        // ------------------------------------
        let repo_name_uri = "/repo_remove_me/?create=true".to_string();
        let app = Router::new()
            .typed_post(create_repository::<RepositoryPath>)
            .layer(middleware::from_fn(print_request_response));

        let request = request_uri_for_test(&repo_name_uri, Method::POST);
        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert!(path.exists());

        // ------------------------------------------
        // Create a new repository WITHOUT ACL access
        // ------------------------------------------
        let repo_name_uri = "/repo_not_allowed/?create=true".to_string();
        let app = Router::new()
            .typed_post(create_repository::<RepositoryPath>)
            .layer(middleware::from_fn(print_request_response));

        let request = request_uri_for_test(&repo_name_uri, Method::POST);
        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        assert!(!not_allowed_path.exists());

        // ------------------------------------------
        // Delete a repository WITHOUT ACL access
        // ------------------------------------------
        let repo_name_uri = "/repo_remove_me/?create=true".to_string();
        let app = Router::new()
            .typed_delete(delete_repository::<RepositoryPath>)
            .layer(middleware::from_fn(print_request_response));

        let request = Request::builder()
            .uri(&repo_name_uri)
            .method(Method::DELETE)
            .header(
                "Authorization",
                basic_auth_header_value("restic", Some("__wrong_password__")),
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
        let repo_name_uri = "/repo_remove_me/".to_string();
        let app = Router::new()
            .typed_delete(delete_repository::<RepositoryPath>)
            .layer(middleware::from_fn(print_request_response));

        let request = request_uri_for_test(&repo_name_uri, Method::DELETE);
        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert!(!path.exists());
    }
}
