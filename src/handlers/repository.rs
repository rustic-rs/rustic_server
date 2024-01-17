use crate::auth::AuthFromRequest;
use crate::error::ErrorKind;
use crate::handlers::access_check::check_auth_and_acl;
use crate::handlers::path_analysis::{decompose_path, ArchivePathEnum, DEFAULT_PATH, TYPES};
use crate::storage::STORAGE;
use crate::{acl::AccessType, error::Result};
use axum::extract::Query;
use axum::{extract::Path as PathExtract, http::StatusCode, response::IntoResponse};
use serde_derive::Deserialize;
use std::path::Path;

//==============================================================================
// Create_repository
// Interface: POST {path}?create=true
//==============================================================================
#[derive(Default, Deserialize)]
#[serde(default)]
pub(crate) struct Create {
    create: bool,
}

// FIXME: The input path should be 1 folder deep (right??)
pub(crate) async fn create_repository(
    auth: AuthFromRequest,
    path: Option<PathExtract<String>>,
    Query(params): Query<Create>,
) -> Result<impl IntoResponse> {
    let path_string = path.map_or(DEFAULT_PATH.to_string(), |PathExtract(path_ext)| path_ext);
    let archive_path = decompose_path(path_string)?;
    let p_str = archive_path.path;
    let tpe = archive_path.tpe;
    assert_eq!(&archive_path.path_type, &ArchivePathEnum::NONE);
    assert_eq!(&tpe, "");
    tracing::debug!("[create_repository] repo_path: {p_str:?}");

    let path = Path::new(&p_str);
    //FIXME: Is Append the right access leven, or should we require Modify?
    check_auth_and_acl(auth.user, &tpe, path, AccessType::Append)?;

    let storage = STORAGE.get().unwrap();
    match params.create {
        true => {
            for tpe_i in TYPES.iter() {
                if let Err(e) = storage.create_dir(path, tpe_i) {
                    return Err(ErrorKind::CreatingDirectoryFailed(e.to_string()));
                };
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

//==============================================================================
// Delete_repository
// Interface: Delete {path}
//==============================================================================

// FIXME: The input path should at least NOT point to a file in any repository
pub(crate) async fn delete_repository(
    auth: AuthFromRequest,
    path: Option<PathExtract<String>>,
) -> Result<impl IntoResponse> {
    let path_string = path.map_or(DEFAULT_PATH.to_string(), |PathExtract(path_ext)| path_ext);
    let archive_path = decompose_path(path_string)?;
    let p_str = archive_path.path;
    let tpe = archive_path.tpe;
    assert_eq!(archive_path.path_type, ArchivePathEnum::NONE);
    assert_eq!(&tpe, "");
    tracing::debug!("[delete_repository] repo_path: {p_str:?}");

    let path = Path::new(&p_str);
    //FIXME: We surely need modify access to delete right??
    check_auth_and_acl(auth.user, "", path, AccessType::Modify)?;

    let storage = STORAGE.get().unwrap();
    if let Err(e) = storage.remove_repository(path) {
        tracing::debug!("[got IO error] {e:?}");
        return Err(ErrorKind::RemovingRepositoryFailed(
            path.to_string_lossy().into(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::handlers::repository::{create_repository, delete_repository};
    use crate::test_server::{basic_auth, init_test_environment, print_request_response};
    use axum::http::Method;
    use axum::routing::post;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use axum::{middleware, Router};
    use std::path::PathBuf;
    use std::{env, fs};
    use tower::ServiceExt; // for `call`, `oneshot`, and `ready`

    /// The acl.toml test allows the create of "repo_remove_me"
    /// for user test with the correct password
    #[tokio::test]
    async fn repo_create_delete_test() {
        init_test_environment();

        //Start with a clean slate ...
        let cwd = env::current_dir().unwrap();
        let path = PathBuf::new()
            .join(cwd)
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

        let request = Request::builder()
            .uri(&repo_name_uri)
            .method(Method::POST)
            .header("Authorization", basic_auth("test", Some("test_pw")))
            .body(Body::empty())
            .unwrap();

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

        let request = Request::builder()
            .uri(&repo_name_uri)
            .method(Method::POST)
            .header("Authorization", basic_auth("test", Some("test_pw")))
            .body(Body::empty())
            .unwrap();

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
                basic_auth("test", Some("__wrong_password__")),
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
