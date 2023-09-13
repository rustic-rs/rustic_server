use clap::Parser;
use rustic_server::{acl::Acl, auth::Auth, log, storage::LocalStorage, web, web::State, Opts};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> tide::Result<()> {
    log::init_tracing();

    let opts = Opts::parse();

    let ports = Ports {
        http: opts.http_port,
        https: opts.https_port,
    };

    // optional: spawn a second server to redirect http requests to this server
    tokio::spawn(redirect_http_to_https(ports));

    // configure certificate and private key used by https
    let config = match opts.tls {
        true => {
            Some(
                RustlsConfig::from_pem_file(
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("self_signed_certs")
                        .join("cert.pem"),
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("self_signed_certs")
                        .join("key.pem"),
                )
                .await
                .unwrap(),
            );
        }
        false => None,
    };

    let storage = LocalStorage::try_new(&opts.path)?;
    let auth = Auth::from_file(opts.no_auth, &opts.path.join(".htpasswd"))?;
    let acl = Acl::from_file(opts.append_only, opts.private_repo, opts.acl)?;

    let new_state = State::new(auth, acl, storage);

    let app = Router::new().route("/", get(handler));

    // run https server
    let addr = SocketAddr::from(([127, 0, 0, 1], ports.https));
    tracing::debug!("listening on {}", addr);
    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .unwrap();

    web::main(new_state, opts.listen, opts.tls, opts.cert, opts.key).await
}

async fn redirect_http_to_https(ports: Ports) {
    fn make_https(host: String, uri: Uri, ports: Ports) -> Result<Uri, BoxError> {
        let mut parts = uri.into_parts();

        parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

        if parts.path_and_query.is_none() {
            parts.path_and_query = Some("/".parse().unwrap());
        }

        let https_host = host.replace(&ports.http.to_string(), &ports.https.to_string());
        parts.authority = Some(https_host.parse()?);

        Ok(Uri::from_parts(parts)?)
    }

    let redirect = move |Host(host): Host, uri: Uri| async move {
        match make_https(host, uri, ports) {
            Ok(uri) => Ok(Redirect::permanent(&uri.to_string())),
            Err(error) => {
                tracing::warn!(%error, "failed to convert URI to HTTPS");
                Err(StatusCode::BAD_REQUEST)
            }
        }
    };

    let addr = SocketAddr::from(([127, 0, 0, 1], ports.http));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, redirect.into_make_service())
        .await
        .unwrap();
}
