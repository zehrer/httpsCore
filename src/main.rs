use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::fs;

use rcgen::{CertificateParams, DistinguishedName, KeyPair};
use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConnection, StreamOwned};
use time::{Duration, OffsetDateTime};

const ADDR: &str = "[::]:443";
const HTML_FILE: &str = "index.html";
const CERT_VALIDITY_DAYS: i64 = 365;

fn tls_config() -> Arc<ServerConfig> {
    let key_pair = KeyPair::generate().expect("Failed to generate key pair");

    let mut params = CertificateParams::new(vec!["localhost".to_string()])
        .expect("Failed to create certificate params");
    params.distinguished_name = DistinguishedName::new();
    params.not_before = OffsetDateTime::now_utc();
    params.not_after = OffsetDateTime::now_utc() + Duration::days(CERT_VALIDITY_DAYS);

    let cert = params.self_signed(&key_pair).expect("Failed to generate certificate");

    let cert_der = CertificateDer::from(cert.der().to_vec());
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_pair.serialize_der()));

    println!("TLS certificate valid for {CERT_VALIDITY_DAYS} days");

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der], key)
        .expect("Invalid certificate or key");

    Arc::new(config)
}

fn main() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install crypto provider");

    let config = tls_config();

    let listener = TcpListener::bind(ADDR)
        .expect("Failed to bind — try: sudo setcap cap_net_bind_service=+ep ./server_runtime");
    println!("Listening on https://{ADDR}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_connection(stream, Arc::clone(&config)),
            Err(e) => eprintln!("Connection error: {e}"),
        }
    }
}

fn handle_connection(stream: TcpStream, config: Arc<ServerConfig>) {
    let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_default();

    let conn = match ServerConnection::new(config) {
        Ok(c) => c,
        Err(e) => { eprintln!("TLS error: {e}"); return; }
    };
    let mut tls = StreamOwned::new(conn, stream);

    let request_line = {
        let mut reader = BufReader::new(&mut tls);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap_or(0);
        line.trim().to_string()
    };

    println!("{peer} -> {request_line}");

    let (status, body) = match fs::read_to_string(HTML_FILE) {
        Ok(content) => ("200 OK", content),
        Err(_) => (
            "404 Not Found",
            "<html><body><h1>404 - index.html not found</h1></body></html>".to_string(),
        ),
    };

    let len = body.len();
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {len}\r\n\r\n{body}"
    );
    let _ = tls.write_all(response.as_bytes());
}
