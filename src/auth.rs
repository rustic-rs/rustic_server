use std::{borrow::Borrow, path::PathBuf};

use abscissa_core::SecretString;
use axum::{extract::FromRequestParts, http::request::Parts};
use axum_auth::AuthBasic;
use serde_derive::Deserialize;
use std::sync::OnceLock;

use crate::{
    config::HtpasswdSettings,
    error::{ApiErrorKind, ApiResult, AppResult},
    htpasswd::{CredentialMap, Htpasswd},
};

//Static storage of our credentials
pub static AUTH: OnceLock<Auth> = OnceLock::new();

pub(crate) fn init_auth(auth: Auth) -> AppResult<()> {
    let _ = AUTH.get_or_init(|| auth);
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct Auth {
    users: Option<CredentialMap>,
}

impl From<CredentialMap> for Auth {
    fn from(users: CredentialMap) -> Self {
        Self { users: Some(users) }
    }
}

impl From<Htpasswd> for Auth {
    fn from(htpasswd: Htpasswd) -> Self {
        Self {
            users: Some(htpasswd.credentials),
        }
    }
}

impl Auth {
    pub fn from_file(disable_auth: bool, path: &PathBuf) -> AppResult<Self> {
        Ok(if disable_auth {
            Self::default()
        } else {
            Htpasswd::from_file(path)?.into()
        })
    }

    pub fn from_config(settings: &HtpasswdSettings, path: PathBuf) -> AppResult<Self> {
        Self::from_file(settings.is_disabled(), &path)
    }

    // verify verifies user/passwd against the credentials saved in users.
    // returns true if Auth::users is None.
    pub fn verify(&self, user: impl Into<String>, passwd: impl Into<String>) -> bool {
        let user = user.into();
        let passwd = passwd.into();

        self.users.as_ref().map_or(true, |users| matches!(users.get(&user), Some(passwd_data) if htpasswd_verify::Htpasswd::from(passwd_data.to_string().borrow()).check(user, passwd)))
    }

    pub const fn is_disabled(&self) -> bool {
        self.users.is_none()
    }
}

#[derive(Deserialize, Debug)]
pub struct AuthFromRequest {
    pub(crate) user: String,
    pub(crate) _password: SecretString,
}

#[async_trait::async_trait]
impl<S: Send + Sync> FromRequestParts<S> for AuthFromRequest {
    type Rejection = ApiErrorKind;

    // FIXME: We also have a configuration flag do run without authentication
    // This must be handled here too ... otherwise we get an Auth header missing error.
    async fn from_request_parts(parts: &mut Parts, state: &S) -> ApiResult<Self> {
        let checker = AUTH.get().unwrap();

        let auth_result = AuthBasic::from_request_parts(parts, state).await;

        tracing::debug!(?auth_result, "[AUTH]");

        return match auth_result {
            Ok(auth) => {
                let AuthBasic((user, passw)) = auth;
                let password = passw.unwrap_or_else(String::new);
                if checker.verify(user.as_str(), password.as_str()) {
                    Ok(Self {
                        user,
                        _password: password.into(),
                    })
                } else {
                    Err(ApiErrorKind::UserAuthenticationError(user))
                }
            }
            Err(_) => {
                let user = String::new();
                if checker.verify("", "") {
                    return Ok(Self {
                        user,
                        _password: String::new().into(),
                    });
                }
                Err(ApiErrorKind::AuthenticationHeaderError)
            }
        };
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::testing::{basic_auth_header_value, init_test_environment, server_config};

    use anyhow::Result;
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
        routing::get,
        Router,
    };
    use http_body_util::BodyExt;
    use rstest::{fixture, rstest};
    use tower::ServiceExt;

    #[fixture]
    fn auth() -> Auth {
        let htpasswd = PathBuf::from("tests/fixtures/test_data/.htpasswd");
        Auth::from_file(false, &htpasswd).unwrap()
    }

    #[rstest]
    fn test_auth_passes(auth: Auth) -> Result<()> {
        assert!(auth.verify("rustic", "rustic"));
        assert!(!auth.verify("rustic", "_rustic"));

        Ok(())
    }

    #[rstest]
    fn test_auth_from_file_passes(auth: Auth) {
        init_auth(auth).unwrap();

        let auth = AUTH.get().unwrap();
        assert!(auth.verify("rustic", "rustic"));
        assert!(!auth.verify("rustic", "_rustic"));
    }

    async fn format_auth_basic(AuthBasic((id, password)): AuthBasic) -> String {
        format!("Got {} and {:?}", id, password)
    }

    async fn format_handler_from_auth_request(auth: AuthFromRequest) -> String {
        format!("User = {}", auth.user)
    }

    /// The requests which should be returned OK
    #[tokio::test]
    async fn test_authentication_passes() {
        init_test_environment(server_config());

        // -----------------------------------------
        // Try good basic
        // -----------------------------------------
        let app = Router::new().route("/basic", get(format_auth_basic));

        let request = Request::builder()
            .uri("/basic")
            .method(Method::GET)
            .header(
                "Authorization",
                basic_auth_header_value("My Username", Some("My Password")),
            )
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_parts().1;
        let byte_vec = body.into_data_stream().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(byte_vec.to_vec()).unwrap();
        assert_eq!(
            body_str,
            String::from("Got My Username and Some(\"My Password\")")
        );

        // -----------------------------------------
        // Try good using auth struct
        // -----------------------------------------
        let app = Router::new().route("/rustic_server", get(format_handler_from_auth_request));

        let request = Request::builder()
            .uri("/rustic_server")
            .method(Method::GET)
            .header(
                "Authorization",
                basic_auth_header_value("rustic", Some("rustic")),
            )
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status().as_u16(), StatusCode::OK.as_u16());
        let body = resp.into_parts().1;
        let byte_vec = body.collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(byte_vec.to_vec()).unwrap();
        assert_eq!(body_str, String::from("User = rustic"));
    }

    #[tokio::test]
    async fn test_fail_authentication_passes() {
        init_test_environment(server_config());

        // -----------------------------------------
        // Try wrong password rustic_server
        // -----------------------------------------
        let app = Router::new().route("/rustic_server", get(format_handler_from_auth_request));

        let request = Request::builder()
            .uri("/rustic_server")
            .method(Method::GET)
            .header(
                "Authorization",
                basic_auth_header_value("rustic", Some("_rustic")),
            )
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        // -----------------------------------------
        // Try without authentication header
        // -----------------------------------------
        let app = Router::new().route("/rustic_server", get(format_handler_from_auth_request));

        let request = Request::builder()
            .uri("/rustic_server")
            .method(Method::GET)
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status().as_u16(), StatusCode::FORBIDDEN);
    }
}
