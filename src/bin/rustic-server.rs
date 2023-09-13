use clap::Parser;
use rustic_server::{acl::Acl, auth::Auth, log, storage::LocalStorage, web, web::AppState, Opts};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    log::init_tracing();

    let opts = Opts::parse();

    let ports = Ports {
        http: opts.http_port,
        https: opts.https_port,
    };

    // optional: spawn a second server to redirect http requests to this server
    tokio::spawn(redirect_http_to_https(ports));

    let storage = LocalStorage::try_new(&opts.path)?;
    let auth = Auth::from_file(opts.no_auth, &opts.path.join(".htpasswd"))?;
    let acl = Acl::from_file(opts.append_only, opts.private_repo, opts.acl)?;

    let new_state = AppState::new(auth, acl, storage);

    web::main(new_state, opts.listen, ports, opts.tls, opts.cert, opts.key).await
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
