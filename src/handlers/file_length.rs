use crate::auth::AuthFromRequest;
use crate::error::ErrorKind;
use crate::handlers::access_check::check_auth_and_acl;
use crate::handlers::path_analysis::{decompose_path, ArchivePathEnum};
use crate::storage::STORAGE;
use crate::{acl::AccessType, error::Result};
use axum::extract::OriginalUri;
use axum::{http::header, response::IntoResponse};
use axum_extra::headers::HeaderMap;
use std::path::Path;

/// Length
/// Interface: HEAD {path}/{type}/{name}
pub(crate) async fn file_length(
    auth: AuthFromRequest,
    uri: OriginalUri,
) -> Result<impl IntoResponse> {
    //let path_string = path.map_or(DEFAULT_PATH.to_string(), |PathExtract(path_ext)| path_ext);
    let path_string = uri.path();
    let archive_path = decompose_path(path_string)?;
    let p_str = archive_path.path;
    let tpe = archive_path.tpe;
    let name = archive_path.name;
    assert_ne!(archive_path.path_type, ArchivePathEnum::Config);
    tracing::debug!("[length] path: {p_str}, tpe: {tpe}, name: {name}");

    let path = Path::new(&p_str);
    check_auth_and_acl(auth.user, tpe.as_str(), path, AccessType::Read)?;

    let storage = STORAGE.get().unwrap();
    let file = storage.filename(path, &tpe, &name);
    return if file.exists() {
        let storage = STORAGE.get().unwrap();
        let file = match storage.open_file(path, &tpe, &name).await {
            Ok(file) => file,
            Err(_) => {
                return Err(ErrorKind::FileNotFound(p_str));
            }
        };
        let length = match file.metadata().await {
            Ok(meta) => meta.len(),
            Err(_) => {
                return Err(ErrorKind::GettingFileMetadataFailed);
            }
        };
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_LENGTH, length.into());
        Ok(headers)
    } else {
        Err(ErrorKind::FileNotFound(p_str))
    };
}

#[cfg(test)]
mod test {
    use crate::handlers::file_length::file_length;
    use crate::log::print_request_response;
    use crate::test_helpers::{init_test_environment, request_uri_for_test};
    use axum::http::StatusCode;
    use axum::http::{header, Method};
    use axum::routing::head;
    use axum::{middleware, Router};
    use http_body_util::BodyExt;
    use tower::ServiceExt; // for `call`, `oneshot`, and `ready`

    #[tokio::test]
    async fn server_file_length_tester() {
        init_test_environment();

        // ----------------------------------
        // File exists
        // ----------------------------------
        let app = Router::new()
            .route("/*path", head(file_length))
            .layer(middleware::from_fn(print_request_response));

        let uri =
            "/test_repo/keys/2e734da3fccb98724ece44efca027652ba7a335c224448a68772b41c0d9229d5";
        let request = request_uri_for_test(uri, Method::HEAD);
        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(&resp.status(), &StatusCode::from_u16(200).unwrap());

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
            .route("/*path", head(file_length))
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
