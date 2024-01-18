// use crate::acl::{init_acl, Acl};
// use crate::auth::init_auth;
// use crate::auth::Auth;
// use crate::error::ErrorKind;
// use crate::log::init_tracing;
// use crate::storage::{init_storage, LocalStorage};
// use axum::http::HeaderValue;
// /// FIXME: Should we keep the server to allow a test to run in the test folder
// /// For example using rustic to fill a backup over this web server to localhost??
// use axum::{
//     body::{Body, Bytes},
//     extract::Request,
//     middleware::Next,
//     response::{IntoResponse, Response},
//     routing::post,
//     Router, ServiceExt,
// };
// use http_body_util::BodyExt;
// use once_cell::sync::OnceCell;
// use std::env;
// use std::net::SocketAddr;
// use std::path::PathBuf;
// use std::sync::{Arc, Mutex};
// use tokio::net::TcpListener;
// use tokio::sync::oneshot::{Receiver, Sender};
// use rustic_server::log::init_tracing;
//
// pub(crate) const WAIT_DELAY: u64 = 250; //Delay in ms to wait for server start
//
// //When starting a server we fetch the mutex to force serial testing
// pub(crate) static WEB: OnceCell<Arc<Mutex<usize>>> = OnceCell::new();
// pub(crate) fn init_mutex() {
//     WEB.get_or_init(|| Arc::new(Mutex::new(0)));
// }
//
// pub struct TestServer {
//     tx: Option<Sender<()>>,
//     router: Router,
// }
//
// impl TestServer {
//     pub fn new(router: Router) -> Self {
//         TestServer { tx: None, router }
//     }
//
//     pub async fn stop_server(self) {
//         if self.tx.is_some() {
//             self.tx
//                 .unwrap()
//                 .send(())
//                 .expect("Failed to stop the test server");
//         }
//         tokio::time::sleep(tokio::time::Duration::from_millis(WAIT_DELAY)).await;
//     }
//
//     pub async fn launch(&mut self) {
//         init_mutex();
//         let _r = WEB.get().take().unwrap();
//
//         let routes = self.router.clone();
//         let (tx, rx) = tokio::sync::oneshot::channel::<()>();
//         self.tx = Some(tx);
//         tokio::task::spawn(TestServer::launcher(rx, routes));
//         tokio::time::sleep(tokio::time::Duration::from_millis(WAIT_DELAY)).await;
//     }
//
//     /// Launches spin-off axum instance
//     pub async fn launcher(rx: Receiver<()>, app: Router) {
//         //FIXME: Is this the best place for this during test?
//         init_tracing();
//
//         TestServer::test_init_static_htaccess();
//         TestServer::test_init_static_auth();
//         TestServer::test_init_static_storage();
//
//         // Launch
//         let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
//
//         axum::serve(
//             TcpListener::bind(addr).await.unwrap(),
//             app.into_make_service(),
//         )
//             .with_graceful_shutdown(async {
//                 rx.await.ok();
//             })
//             .await
//             .unwrap();
//     }
//
//     pub fn url(path: &str) -> String {
//         format!("http://127.0.0.1:3000{}", path)
//     }
//
