use std::path::Path;

use axum::{http::StatusCode, response::IntoResponse};

// used for using auto-generated TpeKind variant names
use strum::VariantNames;

use crate::{
    acl::{AccessType, AclChecker, ACL},
    error::{ApiErrorKind, ApiResult},
    typed_path::TpeKind,
};

pub(crate) fn check_auth_and_acl(
    user: String,
    tpe: impl Into<Option<TpeKind>>,
    path: &Path,
    append: AccessType,
) -> ApiResult<impl IntoResponse> {
    let tpe = tpe.into();

    // don't allow paths that includes any of the defined types
    for part in path.iter() {
        //FIXME: Rewrite to?? -> if TYPES.contains(part) {}
        if let Some(part) = part.to_str() {
            for tpe_i in TpeKind::VARIANTS.iter() {
                if &part == tpe_i {
                    return Err(ApiErrorKind::PathNotAllowed(path.display().to_string()));
                }
            }
        }
    }

    let acl = ACL.get().unwrap();
    let path = if let Some(path) = path.to_str() {
        path
    } else {
        return Err(ApiErrorKind::NonUnicodePath(path.display().to_string()));
    };
    let allowed = acl.allowed(&user, path, tpe, append);
    tracing::debug!("[auth] user: {user}, path: {path}, tpe: {tpe:?}, allowed: {allowed}");

    match allowed {
        true => Ok(StatusCode::OK),
        false => Err(ApiErrorKind::PathNotAllowed(path.to_string())),
    }
}
