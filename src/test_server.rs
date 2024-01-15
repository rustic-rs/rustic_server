use axum::{body::{Body, Bytes}, extract::Request, middleware::{Next}, response::{IntoResponse, Response}, routing::post, Router, ServiceExt};
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use axum::http::HeaderValue;
use tokio::net::TcpListener;
use crate::acl::{Acl, init_acl};
use crate ::auth::{Auth, };
use crate::auth::init_auth;
use crate::error::ErrorKind;
use crate::log::init_tracing;
use crate::storage::{init_storage, LocalStorage};
use http_body_util::BodyExt;
use once_cell::sync::OnceCell;
use tokio::sync::oneshot::{Receiver, Sender};

pub(crate) const WAIT_DELAY:u64 = 250; //Delay in ms to wait for server start

//When starting a server we fetch the mutex to force serial testing
pub(crate) static WEB:OnceCell<Arc<Mutex<usize>>> = OnceCell::new();
pub(crate) fn init_mutex() {
    WEB.get_or_init(|| {
        Arc::new(Mutex::new(0))
    });
}


pub struct TestServer {
    tx: Option<Sender<()>>,
    router: Router,
}

impl TestServer {

    pub fn new(router:Router) -> Self {
        TestServer{tx:None, router}
    }

    pub async fn stop_server(self) {
        if self.tx.is_some() {
            self.tx.unwrap().send(()).expect("Failed to stop the test server");
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(WAIT_DELAY)).await;
    }

    pub async fn launch(&mut self) {
        init_mutex();
        let _r = WEB.get().take().unwrap();

        let routes = self.router.clone();
        let (tx, rx) = tokio::sync::oneshot::channel::<()>();
        self.tx = Some(tx);
        tokio::task::spawn(TestServer::launcher(rx, routes));
        tokio::time::sleep(tokio::time::Duration::from_millis(WAIT_DELAY)).await;
    }

    /// Launches spin-off axum instance
    pub async fn launcher(rx:Receiver<()>, app: Router) {
        //FIXME: Is this the best place for this during test?
        init_tracing();

        TestServer::test_init_static_htaccess();
        TestServer::test_init_static_auth();
        TestServer::test_init_static_storage();

        // Launch
        let addr = SocketAddr::from(([127, 0, 0, 1], 3000));


        axum::serve(
            TcpListener::bind(addr).await.unwrap(),
            app.into_make_service(),
        ).with_graceful_shutdown(async {
            rx.await.ok();
        })
            .await
            .unwrap();
    }

    pub fn url(path: &str) -> String {
        format!("http://127.0.0.1:3000{}", path)
    }

    pub fn test_init_static_htaccess() {
        let cwd = env::current_dir().unwrap();
        let htaccess = PathBuf::new()
            .join(cwd)
            .join("test_data")
            .join("htaccess");

        let auth = Auth::from_file(false, &htaccess).unwrap();
        init_auth(auth).unwrap();
    }

    pub fn test_init_static_auth() {
        let cwd = env::current_dir().unwrap();
        let acl_path = PathBuf::new()
            .join(cwd)
            .join("test_data")
            .join("acl.toml");

        let acl = Acl::from_file(false, true, Some(acl_path)).unwrap();
        init_acl(acl).unwrap();
    }

   pub fn test_init_static_storage() {
        let cwd = env::current_dir().unwrap();
        let repo_path = PathBuf::new()
            .join(cwd)
            .join("test_data")
            .join("test_repos");

        let local_storage = LocalStorage::try_new(&repo_path).unwrap();
        init_storage(local_storage).unwrap();
    }
}

pub fn init_test_environment() {
    //FIXME: Is this the best place for this during test?
    init_tracing();

    TestServer::test_init_static_htaccess();
    TestServer::test_init_static_auth();
    TestServer::test_init_static_storage();

}

pub fn basic_auth<U, P>(username: U, password: Option<P>) -> HeaderValue
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


pub async fn print_request_response(
    req: Request,
    next: Next,
) -> Result<impl IntoResponse, ErrorKind> {
    let (parts, body) = req.into_parts();
    for (k,v) in parts.headers.iter() {
        tracing::debug!("request-header: {k:?} -> {v:?} ");
    }
    let bytes = buffer_and_print("request", body).await?;
    let req = Request::from_parts(parts, Body::from(bytes));

    let res = next.run(req).await;

    let (parts, body) = res.into_parts();
    for (k,v) in parts.headers.iter() {
        tracing::debug!("reply-header: {k:?} -> {v:?} ");
    }
    let bytes = buffer_and_print("response", body).await?;
    let res = Response::from_parts(parts, Body::from(bytes));

    Ok(res)
}

async fn buffer_and_print<B>(direction: &str, body: B) -> Result<Bytes, ErrorKind>
    where
        B: axum::body::HttpBody<Data = Bytes>,
        B::Error: std::fmt::Display,
{
    let bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(err) => {
            return Err(ErrorKind::BadRequest(
                format!("failed to read {direction} body: {err}"),
            ));
        }
    };

    if let Ok(body) = std::str::from_utf8(&bytes) {
        tracing::debug!("{direction} body = {body:?}");
    }

    Ok(bytes)
}
