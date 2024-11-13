use std::path::Path;

use axum::{
    http::{
        header::{self, AUTHORIZATION},
        StatusCode,
    },
    response::IntoResponse,
    Json,
};
use axum_extra::headers::HeaderMap;
use serde_derive::{Deserialize, Serialize};

use crate::{
    acl::AccessType,
    auth::AuthFromRequest,
    error::ApiResult,
    handlers::{access_check::check_auth_and_acl, file_helpers::IteratorAdapter},
    storage::STORAGE,
    typed_path::PathParts,
};

// FIXME: Make it an enum internally
const API_V1: &str = "application/vnd.x.restic.rest.v1";
const API_V2: &str = "application/vnd.x.restic.rest.v2";

/// List files
/// Interface: GET {path}/{type}/
#[derive(Serialize, Deserialize)]
struct RepoPathEntry {
    name: String,
    size: u64,
}

pub(crate) async fn list_files<P: PathParts>(
    path: P,
    auth: AuthFromRequest,
    headers: HeaderMap,
) -> ApiResult<impl IntoResponse> {
    let (path, tpe, _) = path.parts();

    tracing::debug!("[list_files] path: {path:?}, tpe: {tpe:?}");
    let path = path.unwrap_or_default();
    let path = Path::new(&path);
    let _ = check_auth_and_acl(auth.user, tpe, path, AccessType::Read)?;

    let storage = STORAGE.get().unwrap();
    let read_dir = storage.read_dir(path, tpe.map(|f| f.into()));

    let mut res = match headers
        .get(header::ACCEPT)
        .and_then(|header| header.to_str().ok())
    {
        Some(API_V2) => {
            let read_dir_version = read_dir.map(|e| {
                RepoPathEntry {
                    name: e.file_name().to_str().unwrap().to_string(),
                    size: e.metadata().unwrap().len(),
                    // FIXME:  return Err(WebErrorKind::GettingFileMetadataFailed.into());
                }
            });
            let mut response = Json(&IteratorAdapter::new(read_dir_version)).into_response();
            tracing::debug!("[list_files::dir_content(V2)] {:?}", response.body());
            let _ = response.headers_mut().insert(
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
            let _ = response.headers_mut().insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static(API_V1),
            );
            let status = response.status_mut();
            *status = StatusCode::OK;
            response
        }
    };
    let _ = res
        .headers_mut()
        .insert(AUTHORIZATION, headers.get(AUTHORIZATION).unwrap().clone());

    Ok(res)
}

#[cfg(test)]
mod test {
    use crate::log::print_request_response;
    use crate::test_helpers::{basic_auth_header_value, init_test_environment};
    use crate::{
        handlers::files_list::{list_files, RepoPathEntry, API_V1, API_V2},
        typed_path::RepositoryTpePath,
    };
    use axum::http::header::{ACCEPT, CONTENT_TYPE};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use axum::{middleware, Router};
    use axum_extra::routing::{
        RouterExt, // for `Router::typed_*`
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt; // for `call`, `oneshot`, and `ready`

    #[tokio::test]
    async fn test_get_list_files_passes() {
        init_test_environment();

        // V1
        let app = Router::new()
            .typed_get(list_files::<RepositoryTpePath>)
            .layer(middleware::from_fn(print_request_response));

        let request = Request::builder()
            .uri("/test_repo/keys")
            .header(ACCEPT, API_V1)
            .header(
                "Authorization",
                basic_auth_header_value("test", Some("test_pw")),
            )
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap().to_str().unwrap(),
            API_V1
        );
        let b = resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec();
        assert!(!b.is_empty());
        let body = std::str::from_utf8(&b).unwrap();
        let r: Vec<String> = serde_json::from_str(body).unwrap();
        let mut found = false;

        for rpe in r {
            if rpe == "2e734da3fccb98724ece44efca027652ba7a335c224448a68772b41c0d9229d5" {
                found = true;
                break;
            }
        }
        assert!(found);

        // V2
        let app = Router::new()
            .typed_get(list_files::<RepositoryTpePath>)
            .layer(middleware::from_fn(print_request_response));

        let request = Request::builder()
            .uri("/test_repo/keys")
            .header(ACCEPT, API_V2)
            .header(
                "Authorization",
                basic_auth_header_value("test", Some("test_pw")),
            )
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap().to_str().unwrap(),
            API_V2
        );
        let b = resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec();
        let body = std::str::from_utf8(&b).unwrap();
        let r: Vec<RepoPathEntry> = serde_json::from_str(body).unwrap();
        assert!(!r.is_empty());

        let mut found = false;

        for rpe in r {
            if rpe.name == "2e734da3fccb98724ece44efca027652ba7a335c224448a68772b41c0d9229d5" {
                assert_eq!(rpe.size, 363);
                found = true;
                break;
            }
        }
        assert!(found);

        // We may have more files, this does not work...
        // let rr = r.first().unwrap();
        // assert_eq!( rr.name, "2e734da3fccb98724ece44efca027652ba7a335c224448a68772b41c0d9229d5");
        // assert_eq!(rr.size, 363);
    }
}
