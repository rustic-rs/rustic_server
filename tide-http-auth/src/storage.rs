use std::any::Any;
use tide::Result;

/// A storage provider. Implementors should pick a concrete `Request` type, representing a
/// struct or parameter sent by a corresponding [`Scheme`].
///
/// # Example
///
/// ```no_run
/// # use async_std::task::block_on;
/// # fn main() -> Result<(), std::io::Error> { block_on(async {
/// #
/// use tide_http_auth::{ Storage, BasicAuthRequest, BasicAuthScheme };
/// #[derive(Clone)]
/// struct MyState;
/// struct MyUserType {
///     username: String
/// }
/// let state = MyState { };
///
/// // note that we're implementing the concrete "BasicAuthRequest" type here.
/// #[async_trait::async_trait]
/// impl Storage<MyUserType, BasicAuthRequest> for MyState {
///     async fn get_user(&self, request: BasicAuthRequest) -> tide::Result<Option<MyUserType>> {
///       if request.username == "Basil" && request.password == "meow time now" {
///         // If the credential request succeeds, return your user type here.
///         Ok(Some(MyUserType{username: "Basil".to_string()}))
///       } else {
///         Ok(None) // Nothing went wrong, but these credentials are invalid.
///         // you might also return Err here, to indicate a problem talking to the backing store.
///       }
///     }
/// }
///
/// let mut app = tide::with_state(state);
///
/// // BasicAuthScheme's ::Request associated type is BasicAuthRequest.
/// app.with(tide_http_auth::Authentication::new(BasicAuthScheme::default()));
///
/// # Ok(()) })}
/// ```
#[async_trait::async_trait]
pub trait Storage<User: Send + Sync + 'static, Request: Any + Send + Sync + 'static> {
    #[doc(hidden)]
    async fn get_user(&self, _request: Request) -> Result<Option<User>> {
        Ok(None)
    }
}
