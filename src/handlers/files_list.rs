use std::{path::Path, str::FromStr};

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
    error::{ApiErrorKind, ApiResult},
    handlers::{access_check::check_auth_and_acl, file_helpers::IteratorAdapter},
    storage::STORAGE,
    typed_path::PathParts,
};

#[derive(Debug, Clone, Copy)]
enum ApiVersionKind {
    V1,
    V2,
}

impl ApiVersionKind {
    pub fn to_static_str(&self) -> &'static str {
        match self {
            ApiVersionKind::V1 => "application/vnd.x.restic.rest.v1",
            ApiVersionKind::V2 => "application/vnd.x.restic.rest.v2",
        }
    }
}

impl std::fmt::Display for ApiVersionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiVersionKind::V1 => write!(f, "application/vnd.x.restic.rest.v1"),
            ApiVersionKind::V2 => write!(f, "application/vnd.x.restic.rest.v2"),
        }
    }
}

impl FromStr for ApiVersionKind {
    type Err = ApiErrorKind;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "application/vnd.x.restic.rest.v1" => Ok(ApiVersionKind::V1),
            "application/vnd.x.restic.rest.v2" => Ok(ApiVersionKind::V2),
            _ => Err(ApiErrorKind::InvalidApiVersion(s.to_string())),
        }
    }
}

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

    tracing::debug!(?path, "type" = ?tpe, "[list_files]");

    let path = path.unwrap_or_default();

    let path = Path::new(&path);

    let _ = check_auth_and_acl(auth.user, tpe, path, AccessType::Read)?;

    let storage = STORAGE.get().unwrap();

    let read_dir = storage.read_dir(path, tpe.map(|f| f.into()));

    let mut res = match headers
        .get(header::ACCEPT)
        .and_then(|header| header.to_str().ok())
    {
        Some(version) if version == ApiVersionKind::V2.to_static_str() => {
            let read_dir_version = read_dir.map(|entry| {
                RepoPathEntry {
                    name: entry.file_name().to_str().unwrap().to_string(),
                    size: entry.metadata().unwrap().len(),
                    // FIXME:  return Err(WebErrorKind::GettingFileMetadataFailed.into());
                }
            });

            let mut response = Json(&IteratorAdapter::new(read_dir_version)).into_response();

            tracing::debug!("[list_files::dir_content] Api V2 | {:?}", response.body());

            let _ = response.headers_mut().insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static(ApiVersionKind::V2.to_static_str()),
            );

            let status = response.status_mut();

            *status = StatusCode::OK;

            response
        }
        _ => {
            let read_dir_version = read_dir.map(|e| e.file_name().to_str().unwrap().to_string());

            let mut response = Json(&IteratorAdapter::new(read_dir_version)).into_response();

            tracing::debug!(
                "[list_files::dir_content] Fallback to V1 | {:?}",
                response.body()
            );

            let _ = response.headers_mut().insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static(ApiVersionKind::V1.to_static_str()),
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
    use axum::{
        body::Body,
        http::{
            header::{ACCEPT, CONTENT_TYPE},
            Request, StatusCode,
        },
        middleware, Router,
    };
    use axum_extra::routing::RouterExt; // for `Router::typed_*`
    use http_body_util::BodyExt;
    use tower::ServiceExt; // for `call`, `oneshot`, and `ready`

    use crate::{
        handlers::files_list::{list_files, ApiVersionKind, RepoPathEntry},
        log::print_request_response,
        test_helpers::{basic_auth_header_value, init_test_environment, server_config},
        typed_path::RepositoryTpePath,
    };

    #[tokio::test]
    async fn test_get_list_files_passes() {
        init_test_environment(server_config());

        // V1
        let app = Router::new()
            .typed_get(list_files::<RepositoryTpePath>)
            .layer(middleware::from_fn(print_request_response));

        let request = Request::builder()
            .uri("/test_repo/keys/")
            .header(ACCEPT, ApiVersionKind::V1.to_static_str())
            .header(
                "Authorization",
                basic_auth_header_value("restic", Some("restic")),
            )
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap().to_str().unwrap(),
            ApiVersionKind::V1.to_static_str()
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
            if rpe == "3f918b737a2b9f72f044d06d6009eb34e0e8d06668209be3ce86e5c18dac0295" {
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
            .uri("/test_repo/keys/")
            .header(ACCEPT, ApiVersionKind::V2.to_static_str())
            .header(
                "Authorization",
                basic_auth_header_value("restic", Some("restic")),
            )
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        assert_eq!(
            resp.headers().get(CONTENT_TYPE).unwrap().to_str().unwrap(),
            ApiVersionKind::V2.to_static_str()
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
            if rpe.name == "3f918b737a2b9f72f044d06d6009eb34e0e8d06668209be3ce86e5c18dac0295" {
                assert_eq!(rpe.size, 460);
                found = true;
                break;
            }
        }
        assert!(found);

        // We may have more files, this does not work...
        // let rr = r.first().unwrap();
        // assert_eq!( rr.name, "3f918b737a2b9f72f044d06d6009eb34e0e8d06668209be3ce86e5c18dac0295");
        // assert_eq!(rr.size, 363);
    }
}
