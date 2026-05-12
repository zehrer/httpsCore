use axum::{
    Router,
    routing::{get, post},
    response::{Html, IntoResponse},
    http::StatusCode,
};
use axum_server::tls_rustls::RustlsConfig;
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};
use time::{Duration, OffsetDateTime};
use std::fs;

mod auth;

const ADDR: &str = "[::]:443";
const HTML_FILE: &str = "index.html";
const CERT_VALIDITY_DAYS: i64 = 365;

#[tokio::main]
async fn main() {
    let tls   = tls_config().await;
    let state = auth::AppState::new();

    let app = Router::new()
        .route("/",                     get(index))
        .route("/health",               get(health))
        .route("/auth/register/start",  post(auth::register_start))
        .route("/auth/register/finish", post(auth::register_finish))
        .route("/auth/login/start",     post(auth::login_start))
        .route("/auth/login/finish",    post(auth::login_finish))
        .with_state(state);

    println!("Listening on https://{ADDR}");

    axum_server::bind_rustls(ADDR.parse::<std::net::SocketAddr>().unwrap(), tls)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn index() -> impl IntoResponse {
    match fs::read_to_string(HTML_FILE) {
        Ok(content) => Html(content).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "index.html not found").into_response(),
    }
}

async fn health() -> &'static str {
    "OK"
}

async fn tls_config() -> RustlsConfig {
    let key_pair = KeyPair::generate().expect("Failed to generate key pair");

    let mut params = CertificateParams::new(vec![
        "agent.homenodes.io".to_string(),
        "localhost".to_string(),
    ])
    .expect("Failed to create certificate params");

    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName,       "Stephan Zehrer");
    dn.push(DnType::OrganizationName, "homenodes.io");
    params.distinguished_name = dn;
    params.not_before = OffsetDateTime::now_utc();
    params.not_after  = OffsetDateTime::now_utc() + Duration::days(CERT_VALIDITY_DAYS);

    let cert = params.self_signed(&key_pair).expect("Failed to sign certificate");

    println!("TLS certificate valid for {CERT_VALIDITY_DAYS} days");

    RustlsConfig::from_pem(
        cert.pem().into_bytes(),
        key_pair.serialize_pem().into_bytes(),
    )
    .await
    .expect("Failed to create TLS config")
}
