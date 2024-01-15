use crate::acl::{AccessType, ACL, AclChecker};
use std::path::{Path};
use axum::http::{StatusCode};
use axum::response::IntoResponse;
use crate::web::{TYPES};
use crate::{
    error::{Result},
};
use crate::error::ErrorKind;


pub(crate) fn check_auth_and_acl(
    user:String,
    tpe:&str,
    path: &Path,
    append: AccessType,
) -> Result<impl IntoResponse> {
    // don't allow paths that includes any of the defined types
    for part in path.iter() {
        //FIXME: Rewrite to?? -> if TYPES.contains(part) {}
        if let Some(part) = part.to_str() {
            for tpe_i in TYPES.iter() {
                if &part == tpe_i {
                    return Err(ErrorKind::PathNotAllowed(path.display().to_string()));
                }
            }
        }
    }

    let acl = ACL.get().unwrap();
    let path = if let Some(path) = path.to_str() {
        path
    }
    else {
        return Err(ErrorKind::NonUnicodePath(path.display().to_string()));
    };
    let allowed = acl.allowed(user.as_str(), path, tpe, append);
    tracing::debug!(
        "[auth] user: {user}, path: {path}, tpe: {tpe}, allowed: {allowed}"
    );

    match allowed {
        true => Ok(StatusCode::OK),
        false => Err(ErrorKind::PathNotAllowed(path.to_string())),
    }
}