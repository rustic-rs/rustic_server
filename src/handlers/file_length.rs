use std::path::Path;

use axum::{http::header, response::IntoResponse};
use axum_extra::headers::HeaderMap;

use crate::{
    acl::AccessType,
    auth::AuthFromRequest,
    error::{ApiErrorKind, ApiResult},
    handlers::access_check::check_auth_and_acl,
    storage::STORAGE,
    typed_path::PathParts,
};

/// Length
/// Interface: HEAD {path}/{type}/{name}
pub(crate) async fn file_length<P: PathParts>(
    path: P,
    auth: AuthFromRequest,
) -> ApiResult<impl IntoResponse> {
    let (path, tpe, name) = path.parts();

    tracing::debug!("[length] path: {path:?}, tpe: {tpe:?}, name: {name:?}");
    let path_str = path.unwrap_or_default();
    let path = Path::new(&path_str);
    let _ = check_auth_and_acl(auth.user, tpe, path, AccessType::Read)?;

    let tpe = if let Some(tpe) = tpe {
        tpe.into_str()
    } else {
        return Err(ApiErrorKind::InternalError("tpe is not valid".to_string()));
    };

    let storage = STORAGE.get().unwrap();
    let file = storage.filename(path, tpe, name.as_deref());
    let res = if file.exists() {
        let storage = STORAGE.get().unwrap();
        let file = match storage.open_file(path, tpe, name.as_deref()).await {
            Ok(file) => file,
            Err(_) => {
                return Err(ApiErrorKind::FileNotFound(path_str));
            }
        };
        let length = match file.metadata().await {
            Ok(meta) => meta.len(),
            Err(err) => {
                return Err(ApiErrorKind::GettingFileMetadataFailed(format!(
                    "path: {path:?}, tpe: {tpe}, name: {name:?}, err: {err}",
                )));
            }
        };
        let mut headers = HeaderMap::new();
        let _ = headers.insert(header::CONTENT_LENGTH, length.into());
        Ok(headers)
    } else {
        Err(ApiErrorKind::FileNotFound(path_str))
    };

    res
}

#[cfg(test)]
mod test {
    use crate::log::print_request_response;
    use crate::test_helpers::{init_test_environment, request_uri_for_test};
    use crate::{handlers::file_length::file_length, typed_path::RepositoryTpeNamePath};
    use axum::http::StatusCode;
    use axum::http::{header, Method};
    use axum::{middleware, Router};
    use axum_extra::routing::{
        RouterExt, // for `Router::typed_*`
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt; // for `call`, `oneshot`, and `ready`

    #[tokio::test]
    async fn test_get_file_length_passes() {
        init_test_environment();

        // ----------------------------------
        // File exists
        // ----------------------------------
        let app = Router::new()
            .typed_head(file_length::<RepositoryTpeNamePath>)
            .layer(middleware::from_fn(print_request_response));

        let uri =
            "/test_repo/keys/2e734da3fccb98724ece44efca027652ba7a335c224448a68772b41c0d9229d5";
        let request = request_uri_for_test(uri, Method::HEAD);
        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);

        let length = resp
            .headers()
            .get(header::CONTENT_LENGTH)
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(length, "363");

        let b = resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec();
        assert!(b.is_empty());

        // ----------------------------------
        // File does NOT exist
        // ----------------------------------
        let app = Router::new()
            .typed_head(file_length::<RepositoryTpeNamePath>)
            .layer(middleware::from_fn(print_request_response));

        let uri = "/test_repo/keys/__I_do_not_exist__";
        let request = request_uri_for_test(uri, Method::HEAD);
        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);

        let b = resp
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec();
        assert!(b.is_empty());
    }
}
