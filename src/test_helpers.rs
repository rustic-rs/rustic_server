use std::{env, path::PathBuf, sync::Mutex, sync::OnceLock};

use axum::{
    body::Body,
    http::{HeaderValue, Method},
};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    acl::{init_acl, Acl},
    auth::{init_auth, Auth},
    storage::{init_storage, LocalStorage},
};

// ------------------------------------------------
// test facility prevent repeated calls in tests
// ------------------------------------------------

/// Common requests, using a password that should
/// be recognized as OK for the repository we are trying to access.
pub fn request_uri_for_test(uri: &str, method: Method) -> axum::http::Request<Body> {
    axum::http::Request::builder()
        .uri(uri)
        .method(method)
        .header(
            "Authorization",
            basic_auth_header_value("test", Some("test_pw")),
        )
        .body(Body::empty())
        .unwrap()
}

// ------------------------------------------------
// test facility for tracing
// ------------------------------------------------

pub(crate) fn init_tracing() {
    init_mutex();
}

/// When we initialize the global tracing subscriber, this must only happen once.
/// During tests, each test will initialize, to make sure we have at least tracing once.
/// This means that the init() call must be robust for this.
/// Since we do not need this in production code, it is located in the test code.
static TRACER: OnceLock<Mutex<usize>> = OnceLock::new();
fn init_mutex() {
    TRACER.get_or_init(|| {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "RUSTIC_SERVER_LOG_LEVEL=debug".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
        Mutex::new(0)
    });
}

// ------------------------------------------------
// test facility for creating a minimum test environment
// ------------------------------------------------

pub(crate) fn init_test_environment() {
    init_tracing();
    init_static_htpasswd();
    init_static_auth();
    init_static_storage();
}

fn init_static_htpasswd() {
    let cwd = env::current_dir().unwrap();
    let htpasswd = PathBuf::new()
        .join(cwd)
        .join("tests")
        .join("fixtures")
        .join("test_data")
        .join(".htpasswd");
    tracing::debug!("[test_init_static_storage] repo: {:?}", &htpasswd);
    let auth = Auth::from_file(false, &htpasswd).unwrap();
    init_auth(auth).unwrap();
}

fn init_static_auth() {
    let cwd = env::current_dir().unwrap();
    let acl_path = PathBuf::new()
        .join(cwd)
        .join("tests")
        .join("fixtures")
        .join("test_data")
        .join("acl.toml");
    tracing::debug!("[test_init_static_storage] repo: {:?}", &acl_path);
    let acl = Acl::from_file(false, true, Some(acl_path)).unwrap();
    init_acl(acl).unwrap();
}

fn init_static_storage() {
    let cwd = env::current_dir().unwrap();
    let repo_path = PathBuf::new()
        .join(cwd)
        .join("tests")
        .join("fixtures")
        .join("test_data")
        .join("test_repos");
    tracing::debug!("[test_init_static_storage] repo: {:?}", &repo_path);
    let local_storage = LocalStorage::try_new(&repo_path).unwrap();
    init_storage(local_storage).unwrap();
}

// ------------------------------------------------
// test facility for authentication
// ------------------------------------------------

/// Creates a header value from a username, and password.
/// Copy for the reqwest crate;
pub(crate) fn basic_auth_header_value<U, P>(username: U, password: Option<P>) -> HeaderValue
where
    U: std::fmt::Display,
    P: std::fmt::Display,
{
    use base64::prelude::BASE64_STANDARD;
    use base64::write::EncoderWriter;
    use std::io::Write;

    let mut buf = b"Basic ".to_vec();
    {
        let mut encoder = EncoderWriter::new(&mut buf, &BASE64_STANDARD);
        let _ = write!(encoder, "{}:", username);
        if let Some(password) = password {
            let _ = write!(encoder, "{}", password);
        }
    }
    let mut header = HeaderValue::from_bytes(&buf).expect("base64 is always valid HeaderValue");
    header.set_sensitive(true);
    header
}
