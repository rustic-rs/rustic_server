mod scheme;
mod storage;

pub use scheme::{BasicAuthRequest, BasicAuthScheme, BearerAuthRequest, BearerAuthScheme, Scheme};
pub use storage::Storage;

use std::marker::PhantomData;
use tide::{Middleware, Next, Request, Response, StatusCode};
use tracing::{error, info};

/// Middleware for implementing a given [`Scheme`] (Basic, Bearer, Jwt) backed by a given
/// [`Storage`] backend implemented by the [Tide application
/// `State`](https://docs.rs/tide/0.9.0/tide/#state).
pub struct Authentication<User: Send + Sync + 'static, ImplScheme: Scheme<User>> {
    pub(crate) scheme: ImplScheme,
    _user_t: PhantomData<User>,
}

#[doc(hidden)]
impl<User: Send + Sync + 'static, ImplScheme: Scheme<User>> std::fmt::Debug
    for Authentication<User, ImplScheme>
{
    fn fmt(
        &self,
        formatter: &mut std::fmt::Formatter<'_>,
    ) -> std::result::Result<(), std::fmt::Error> {
        write!(formatter, "Authentication<Scheme>")?;
        Ok(())
    }
}

impl<User: Send + Sync + 'static, ImplScheme: Scheme<User>> Authentication<User, ImplScheme> {
    /// Create a new authentication middleware with a scheme.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// #[derive(Clone)]
    /// struct MyUserType {
    ///     username: String
    /// }
    /// # fn main() -> Result<(), std::io::Error> { async_std::task::block_on(async {
    /// #
    /// use tide_http_auth::{ Authentication, BasicAuthScheme };
    /// Authentication::<MyUserType, BasicAuthScheme>::new(BasicAuthScheme::default());
    /// # Ok(()) })}
    /// ```
    pub fn new(scheme: ImplScheme) -> Self {
        Self {
            scheme,

            _user_t: PhantomData::default(),
        }
    }
}

#[async_trait::async_trait]
impl<ImplScheme, State, User> Middleware<State> for Authentication<User, ImplScheme>
where
    ImplScheme: Scheme<User> + Send + Sync + 'static,
    State: Storage<User, ImplScheme::Request> + Clone + Send + Sync + 'static,
    User: Send + Sync + 'static,
{
    async fn handle(&self, mut req: Request<State>, next: Next<'_, State>) -> tide::Result {
        // read the header
        let auth_header = req.header(ImplScheme::header_name());
        if auth_header.is_none() {
            info!("no auth header, proceeding");
            return Ok(next.run(req).await);
        }
        let value: Vec<_> = auth_header.unwrap().into_iter().collect();

        if value.is_empty() {
            info!("empty auth header, proceeding");
            return Ok(next.run(req).await);
        }

        if value.len() > 1 && ImplScheme::should_401_on_multiple_values() {
            error!("multiple auth headers, bailing");
            return Ok(Response::new(StatusCode::Unauthorized));
        }

        for value in value {
            let value = value.as_str();
            if !value.starts_with(ImplScheme::scheme_name()) {
                continue;
            }
            let auth_param = &value[ImplScheme::scheme_name().len()..];
            let state = req.state();

            info!("saw auth header, attempting to auth");
            if let Some(user) = self.scheme.authenticate(state, auth_param).await? {
                req.set_ext(user);
                break;
            } else if ImplScheme::should_403_on_bad_auth() {
                error!("Authorization header sent but no user returned, bailing");
                return Ok(Response::new(StatusCode::Forbidden));
            }
        }
        Ok(next.run(req).await)
    }
}
