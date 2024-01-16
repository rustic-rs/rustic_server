use once_cell::sync::OnceCell;
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::{fs, io};
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_auth::{AuthBasic};
use crate::error::ErrorKind;
use serde_derive::{Deserialize};

//Static storage of our credentials
pub static AUTH:OnceCell<Auth> = OnceCell::new();

pub(crate) fn init_auth( state: Auth ) -> Result<(), ErrorKind> {
    if AUTH.get().is_none() {
        match AUTH.set(state) {
            Ok(_) => {}
            Err(_) => {
                return Err(ErrorKind::InternalError("Can not create AUTH struct".to_string()));
            }
        }
    }
    Ok(())
}

// #[enum_dispatch]
// #[derive(Debug, Clone)]
// pub(crate) enum AuthCheckerEnum {
//     Auth(Auth),
// }
//
// impl AuthCheckerEnum {
//     pub fn auth_from_file(no_auth: bool, path: &PathBuf) -> io::Result<Self> {
//         let auth = Auth::from_file(no_auth, path)?;
//         Ok(AuthCheckerEnum::Auth(auth))
//     }
// }

//#[enum_dispatch(AuthCheckerEnum)]
pub trait AuthChecker: Send + Sync + 'static {
    fn verify(&self, user: &str, passwd: &str) -> bool;
}

// read_htpasswd is a helper func that reads the given file in .httpasswd format
// into a Hashmap mapping each user to the whole passwd line
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

#[derive(Debug, Clone)]
pub struct Auth {
    users: Option<HashMap<&'static str, &'static str>>,
}

impl Default for Auth {
    fn default() -> Self {
        Self { users: None }
    }
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
    pub(crate) password: String,
}

#[async_trait::async_trait]
impl<S:Send+Sync> FromRequestParts<S> for AuthFromRequest {
    type Rejection = ErrorKind;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> std::result::Result<Self, ErrorKind> {
        let auth_result = AuthBasic::from_request_parts(parts, state).await;
        let checker = AUTH.get().unwrap();
        tracing::debug!("Got authentication result ...:{:?}", &auth_result);
        return match auth_result {
            Ok(auth) => {
                let AuthBasic((user, passw)) = auth;
                let password = match passw {
                    None => { "".to_string() }
                    Some(p) => { p }
                };
                if checker.verify(user.as_str(), password.as_str() ) {
                    Ok( Self{user, password})
                } else {
                    Err(ErrorKind::UserAuthenticationError(user))
                }
            }
            Err(_) => {
                let user = "".to_string();
                if checker.verify("", "") {
                    return Ok(Self{user, password:"".to_string()})
                }
                Err(ErrorKind::AuthenticationHeaderError)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use std::env;
    use std::path::PathBuf;
    use axum::http::StatusCode;
    use axum::Router;
    use axum::routing::get;
    use std::net::SocketAddr;
    use tokio::net::TcpListener;
    use crate::auth::Auth;
    use crate::test_server::{init_mutex, TestServer, WAIT_DELAY, WEB};
    use super::*;

    #[test]
    fn test_htaccess_account() -> Result<()>{
        let cwd = env::current_dir()?;
        let htaccess = PathBuf::new()
            .join(cwd)
            .join("test_data" )
            .join("htaccess" );
        let auth = Auth::from_file(false, &htaccess )?;
        assert!( auth.verify("test", "test_pw"));
        assert!( ! auth.verify("test", "__test_pw"));

        Ok(())
    }

    #[test]
    fn test_static_htaccess() {
        let cwd = env::current_dir().unwrap();
        let htaccess = PathBuf::new()
            .join(cwd)
            .join("test_data" )
            .join("htaccess" );

        dbg!(&htaccess);

        let auth = Auth::from_file(false, &htaccess ).unwrap();
        init_auth(auth).unwrap();

        let auth = AUTH.get().unwrap();
        assert!( auth.verify("test", "test_pw"));
        assert!( ! auth.verify("test", "__test_pw"));
    }


    #[tokio::test]
    async fn server_auth_tester() -> Result<()> {

        init_mutex();
        let _r = WEB.get().take().unwrap();

        let app = Router::new()
            .route("/basic", get(tester_basic))
            .route("/rustic_server", get(tester_rustic_server));

        let mut server = TestServer::new(app);
        server.launch().await;

        // Tests
        good().await;
        wrong_authentication().await;
        nothing().await;

       server.stop_server().await;

        Ok(())
    }


    async fn tester_basic(AuthBasic((id, password)): AuthBasic) -> String {
        format!("Got {} and {:?}", id, password)
    }

    async fn tester_rustic_server( auth:AuthFromRequest ) -> String {
        format!("User = {}", auth.user )
    }

    /// The requests which should be returned fine
    async fn good() {
        // Try good basic
        let client = reqwest::Client::new();
        let resp = client
            .get(TestServer::url("/basic"))
            .basic_auth("My Username", Some("My Password"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), StatusCode::OK.as_u16());
        assert_eq!(
            resp.text().await.unwrap(),
            String::from("Got My Username and Some(\"My Password\")")
        );

        // Try good rustic_server
        let client = reqwest::Client::new();
        let resp = client
            .get(TestServer::url("/rustic_server"))
            .basic_auth("test", Some("test_pw"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), StatusCode::OK.as_u16());
        assert_eq!(
            resp.text().await.unwrap(),
            String::from("User = test")
        );
    }

    async fn wrong_authentication() {
        // Try bearer  authetication method in basic
        let client = reqwest::Client::new();
        let resp = client
            .get(TestServer::url("/basic"))
            .bearer_auth("123124nfienrign")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), StatusCode::BAD_REQUEST.as_u16());
        assert_eq!(
            resp.text().await.unwrap(),
            String::from("`Authorization` header must be for basic authentication")
        );

        // Try wrong password rustic_server
        let client = reqwest::Client::new();
        let resp = client
            .get(TestServer::url("/rustic_server"))
            .basic_auth("test", Some("__test_pw"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), StatusCode::FORBIDDEN.as_u16());
        assert_eq!(
            resp.text().await.unwrap(),
            String::from("Failed to authenticate user: \"test\"")
        );
    }

    /// Sees if we can get nothing from basic or bearer successfully
    async fn nothing() {
        // Try basic
        let client = reqwest::Client::new();
        let resp = client
            .get(TestServer::url("/basic"))
            .basic_auth("", Some(""))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), StatusCode::OK.as_u16());
        assert_eq!(
            resp.text().await.unwrap(),
            String::from("Got  and Some(\"\")")
        );
    }
}
