use crate::storage::Storage;
use http_types::{Result, StatusCode};

use std::any::Any;

#[async_trait::async_trait]
pub trait Scheme<User: Send + Sync + 'static> {
    type Request: Any + Send + Sync;

    async fn authenticate<S>(&self, state: &S, auth_param: &str) -> Result<Option<User>>
    where
        S: Storage<User, Self::Request> + Send + Sync + 'static;

    fn should_401_on_multiple_values() -> bool {
        true
    }
    fn should_403_on_bad_auth() -> bool {
        true
    }

    fn header_name() -> &'static str {
        "Authorization"
    }
    fn scheme_name() -> &'static str;
}

#[derive(Default, Debug)]
pub struct BasicAuthScheme;

#[derive(Debug)]
pub struct BasicAuthRequest {
    pub username: String,
    pub password: String,
}

#[async_trait::async_trait]
impl<User: Send + Sync + 'static> Scheme<User> for BasicAuthScheme {
    type Request = BasicAuthRequest;

    async fn authenticate<S>(&self, state: &S, auth_param: &str) -> Result<Option<User>>
    where
        S: Storage<User, Self::Request> + Send + Sync + 'static,
    {
        let bytes = base64::decode(auth_param);
        if bytes.is_err() {
            // This is invalid. Fail the request.
            return Err(http_types::Error::from_str(
                StatusCode::Unauthorized,
                "Basic auth param must be valid base64.",
            ));
        }

        let as_utf8 = String::from_utf8(bytes.unwrap());
        if as_utf8.is_err() {
            // You know the drill.
            return Err(http_types::Error::from_str(
                StatusCode::Unauthorized,
                "Basic auth param base64 must contain valid utf-8.",
            ));
        }

        let as_utf8 = as_utf8.unwrap();
        let parts: Vec<_> = as_utf8.split(':').collect();

        if parts.len() < 2 {
            return Ok(None);
        }

        let (username, password) = (parts[0], parts[1]);

        let user = state
            .get_user(BasicAuthRequest {
                username: username.to_owned(),
                password: password.to_owned(),
            })
            .await?;

        Ok(user)
    }

    fn scheme_name() -> &'static str {
        "Basic "
    }
}

#[derive(Default, Debug)]
pub struct BearerAuthScheme {
    prefix: String,
}

pub struct BearerAuthRequest {
    pub token: String,
}

#[async_trait::async_trait]
impl<User: Send + Sync + 'static> Scheme<User> for BearerAuthScheme {
    type Request = BearerAuthRequest;

    async fn authenticate<S>(&self, state: &S, auth_param: &str) -> Result<Option<User>>
    where
        S: Storage<User, Self::Request> + Send + Sync + 'static,
    {
        if !auth_param.starts_with(self.prefix.as_str()) {
            return Ok(None);
        }

        // TODO: validate that the auth_param (sans the prefix) is a valid uuid.
        let user = state
            .get_user(BearerAuthRequest {
                token: (&auth_param[self.prefix.len()..]).to_owned(),
            })
            .await?;
        Ok(user)
    }

    fn scheme_name() -> &'static str {
        "Bearer "
    }
}
