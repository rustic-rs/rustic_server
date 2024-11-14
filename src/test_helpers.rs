use std::{path::PathBuf, sync::Mutex, sync::OnceLock};

use axum::{
    body::Body,
    http::{HeaderValue, Method},
};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    acl::{init_acl, Acl},
    auth::{init_auth, Auth},
    config::{AclSettings, HtpasswdSettings, RusticServerConfig, StorageSettings},
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
            basic_auth_header_value("restic", Some("restic")),
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
/// This means that the `init()` call must be robust for this.
/// Since we do not need this in production code, it is located in the test code.
static TRACER: OnceLock<Mutex<usize>> = OnceLock::new();
fn init_mutex() {
    let _ = TRACER.get_or_init(|| {
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

pub(crate) fn server_config() -> RusticServerConfig {
    let server_config_path = PathBuf::from("tests/fixtures/test_data/rustic_server.toml");
    RusticServerConfig::from_file(&server_config_path).unwrap()
}

pub(crate) fn init_test_environment(server_config: RusticServerConfig) {
    init_tracing();
    init_static_htpasswd(server_config.auth);
    init_static_auth(server_config.acl);
    init_static_storage(server_config.storage);
}

fn init_static_htpasswd(htpasswd_settings: HtpasswdSettings) {
    let auth = Auth::from_config(&htpasswd_settings).unwrap();
    init_auth(auth).unwrap();
}

fn init_static_auth(acl_settings: AclSettings) {
    let acl = Acl::from_config(&acl_settings).unwrap();
    init_acl(acl).unwrap();
}

fn init_static_storage(storage_settings: StorageSettings) {
    let data_dir = storage_settings
        .data_dir
        .unwrap_or_else(|| PathBuf::from("tests/generate/test_storage/"));

    let local_storage = LocalStorage::try_new(&data_dir).unwrap();
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
