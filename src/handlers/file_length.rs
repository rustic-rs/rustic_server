use axum::{
    extract::{Path as PathExtract},
    http::{header},
    response::{IntoResponse},
};
use std::path::{Path};
use axum_extra::headers::HeaderMap;
use crate::{
    acl::{AccessType},
    error::{Result},
};
use crate::auth::AuthFromRequest;
use crate::error::ErrorKind;
use crate::handlers::access_check::check_auth_and_acl;
use crate::handlers::path_analysis::{ArchivePathEnum, decompose_path, DEFAULT_PATH};
use crate::storage::{STORAGE};

//==============================================================================
// Length
// Interface: HEAD {path}/{type}/{name}
//==============================================================================

pub(crate) async fn file_length(
    auth: AuthFromRequest,
    path: Option<PathExtract<String>>,
) -> Result<impl IntoResponse> {

    let path_string = path.map_or(DEFAULT_PATH.to_string(), |PathExtract(path_ext)| path_ext);
    let archive_path = decompose_path(path_string)?;
    let p_str = archive_path.path;
    let tpe = archive_path.tpe;
    let name = archive_path.name;
    assert_ne!( archive_path.path_type, ArchivePathEnum::CONFIG);
    tracing::debug!("[length] path: {p_str}, tpe: {tpe}, name: {name}");

    let path = Path::new(&p_str);
    check_auth_and_acl( auth.user, tpe.as_str(), path, AccessType::Read)?;

    let storage = STORAGE.get().unwrap();
    let file = storage.filename(path, &tpe, &name);
    return if file.exists() {
        let storage = STORAGE.get().unwrap();
        let file = match storage.open_file(path, &tpe, &name).await {
            Ok(file) => {file}
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
        headers.insert(
            header::CONTENT_LENGTH,
            length.into()
        );
        Ok(headers)
    } else {
        Err(ErrorKind::FileNotFound(p_str))
    }
}

#[cfg(test)]
mod test {
    use http_body_util::BodyExt;
    use axum::{ middleware, Router};
    use axum::routing::{head};
    use crate::test_server::{basic_auth, init_test_environment, print_request_response};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use axum::http::{header, Method};
    use tower::{ServiceExt};
    use crate::handlers::file_length::file_length; // for `call`, `oneshot`, and `ready`

    #[tokio::test]
    async fn server_file_length_tester() {
        init_test_environment();

        // ----------------------------------
        // File exists
        // ----------------------------------
        let app = Router::new()
            .route( "/*path",head(file_length) )
            .layer(middleware::from_fn(print_request_response));

        let request = Request::builder()
            .uri("/test_repo/keys/2e734da3fccb98724ece44efca027652ba7a335c224448a68772b41c0d9229d5")
            .method(Method::HEAD)
            .header("Authorization",  basic_auth("test", Some("test_pw")))
            .body(Body::empty()).unwrap();

        let resp = app
            .oneshot(request)
            .await
            .unwrap();

        assert_eq!( resp.status(), StatusCode::OK );
        assert_eq!( &resp.status() ,  &StatusCode::from_u16(200).unwrap());

        let length = resp.headers().get(header::CONTENT_LENGTH).unwrap().to_str().unwrap();
        assert_eq!(length , "363");

        let b = resp.into_body().collect().await.unwrap().to_bytes().to_vec();
        assert!(b.is_empty());

        // ----------------------------------
        // File does NOT exist
        // ----------------------------------
        let app = Router::new()
            .route( "/*path",head(file_length) )
            .layer(middleware::from_fn(print_request_response));

        let request = Request::builder()
            .uri("/test_repo/keys/__I_do_not_exist__")
            .method(Method::HEAD)
            .header("Authorization",  basic_auth("test", Some("test_pw")))
            .body(Body::empty()).unwrap();

        let resp = app
            .oneshot(request)
            .await
            .unwrap();

        assert_eq!(resp.status() , StatusCode::NOT_FOUND);

        let b = resp.into_body().collect().await.unwrap().to_bytes().to_vec();
        assert!(b.is_empty());
    }
}