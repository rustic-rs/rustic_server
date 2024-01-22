use crate::error::ErrorKind;
use anyhow::Result;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_auth::AuthBasic;
use once_cell::sync::OnceCell;
use serde_derive::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::{fs, io};

//Static storage of our credentials
pub static AUTH: OnceCell<Auth> = OnceCell::new();

pub(crate) fn init_auth(state: Auth) -> Result<()> {
    if AUTH.get().is_none() {
        AUTH.set(state).unwrap()
    }
    Ok(())
}

pub trait AuthChecker: Send + Sync + 'static {
    fn verify(&self, user: &str, passwd: &str) -> bool;
}

/// read_htpasswd is a helper func that reads the given file in .httpasswd format
/// into a Hashmap mapping each user to the whole passwd line
fn read_htpasswd(file_path: &PathBuf) -> io::Result<HashMap<&'static str, &'static str>> {
    let s = fs::read_to_string(file_path)?;
    // make the contents static in memory
    let s = Box::leak(s.into_boxed_str());

    let mut user_map = HashMap::new();
    for line in s.lines() {
        let user = line.split(':').collect::<Vec<&str>>()[0];
        user_map.insert(user, line);
    }
    Ok(user_map)
}

#[derive(Debug, Default, Clone)]
pub struct Auth {
    users: Option<HashMap<&'static str, &'static str>>,
}

impl Auth {
    pub fn from_file(no_auth: bool, path: &PathBuf) -> io::Result<Self> {
        Ok(Self {
            users: match no_auth {
                true => None,
                false => Some(read_htpasswd(path)?),
            },
        })
    }
}

impl AuthChecker for Auth {
    // verify verifies user/passwd against the credentials saved in users.
    // returns true if Auth::users is None.
    fn verify(&self, user: &str, passwd: &str) -> bool {
        match &self.users {
            Some(users) => {
                matches!(users.get(user), Some(passwd_data) if htpasswd_verify::Htpasswd::from(*passwd_data).check(user, passwd))
            }
            None => true,
        }
    }
}

#[derive(Deserialize)]
pub struct AuthFromRequest {
    pub(crate) user: String,
    pub(crate) _password: String,
}

#[async_trait::async_trait]
impl<S: Send + Sync> FromRequestParts<S> for AuthFromRequest {
    type Rejection = ErrorKind;

    // FIXME: We also have a configuration flag do run without authentication
    // This must be handled here too ... otherwise we get an Auth header missing error.
    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> std::result::Result<Self, ErrorKind> {
        let checker = AUTH.get().unwrap();
        let auth_result = AuthBasic::from_request_parts(parts, state).await;
        tracing::debug!("Got authentication result ...:{:?}", &auth_result);
        return match auth_result {
            Ok(auth) => {
                let AuthBasic((user, passw)) = auth;
                let password = passw.unwrap_or_else(|| "".to_string());
                if checker.verify(user.as_str(), password.as_str()) {
                    Ok(Self {
                        user,
                        _password: password,
                    })
                } else {
                    Err(ErrorKind::UserAuthenticationError(user))
                }
            }
            Err(_) => {
                let user = "".to_string();
                if checker.verify("", "") {
                    return Ok(Self {
                        user,
                        _password: "".to_string(),
                    });
                }
                Err(ErrorKind::AuthenticationHeaderError)
            }
        };
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::auth::Auth;
    use crate::test_helpers::{basic_auth_header_value, init_test_environment};
    use anyhow::Result;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use axum::routing::get;
    use axum::Router;
    use http_body_util::BodyExt;
    use std::env;
    use std::path::PathBuf;
    use tower::ServiceExt;

    #[test]
    fn test_auth() -> Result<()> {
        let cwd = env::current_dir()?;
        let htaccess = PathBuf::new()
            .join(cwd)
            .join("tests")
            .join("fixtures")
            .join("test_data")
            .join("htaccess");
        let auth = Auth::from_file(false, &htaccess)?;
        assert!(auth.verify("test", "test_pw"));
        assert!(!auth.verify("test", "__test_pw"));

        Ok(())
    }

    #[test]
    fn test_auth_from_file() {
        let cwd = env::current_dir().unwrap();
        let htaccess = PathBuf::new()
            .join(cwd)
            .join("tests")
            .join("fixtures")
            .join("test_data")
            .join("htaccess");

        dbg!(&htaccess);

        let auth = Auth::from_file(false, &htaccess).unwrap();
        init_auth(auth).unwrap();

        let auth = AUTH.get().unwrap();
        assert!(auth.verify("test", "test_pw"));
        assert!(!auth.verify("test", "__test_pw"));
    }

    async fn test_handler_basic(AuthBasic((id, password)): AuthBasic) -> String {
        format!("Got {} and {:?}", id, password)
    }

    async fn test_handler_from_request(auth: AuthFromRequest) -> String {
        format!("User = {}", auth.user)
    }

    /// The requests which should be returned OK
    #[tokio::test]
    async fn test_authentication() {
        init_test_environment();

        // -----------------------------------------
        // Try good basic
        // -----------------------------------------
        let app = Router::new().route("/basic", get(test_handler_basic));

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

        assert_eq!(resp.status().as_u16(), StatusCode::OK.as_u16());
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
        let app = Router::new().route("/rustic_server", get(test_handler_from_request));

        let request = Request::builder()
            .uri("/rustic_server")
            .method(Method::GET)
            .header(
                "Authorization",
                basic_auth_header_value("test", Some("test_pw")),
            )
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status().as_u16(), StatusCode::OK.as_u16());
        let body = resp.into_parts().1;
        let byte_vec = body.collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(byte_vec.to_vec()).unwrap();
        assert_eq!(body_str, String::from("User = test"));
    }

    #[tokio::test]
    async fn test_authentication_errors() {
        init_test_environment();

        // -----------------------------------------
        // Try wrong password rustic_server
        // -----------------------------------------
        let app = Router::new().route("/rustic_server", get(test_handler_from_request));

        let request = Request::builder()
            .uri("/rustic_server")
            .method(Method::GET)
            .header(
                "Authorization",
                basic_auth_header_value("test", Some("__test_pw")),
            )
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        // -----------------------------------------
        // Try without authentication header
        // -----------------------------------------
        let app = Router::new().route("/rustic_server", get(test_handler_from_request));

        let request = Request::builder()
            .uri("/rustic_server")
            .method(Method::GET)
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(request).await.unwrap();

        assert_eq!(resp.status().as_u16(), StatusCode::FORBIDDEN);
    }
}
