# httpsCore

A minimal, IPv6-only HTTPS server written in pure Rust.

## Overview

httpsCore is intentionally small — no frameworks, no async runtime, no unnecessary dependencies. It serves static HTML over TLS on port 443 and is designed to run on a Linux home server. The goal is a solid, understandable foundation to build on.

## Features

- IPv6-only (`[::]` — all interfaces, no IPv4)
- TLS via [rustls](https://github.com/rustls/rustls) — pure Rust, no OpenSSL
- Self-signed certificate generated at startup via [rcgen](https://github.com/rustls/rcgen)
- Serves a static `index.html` from the working directory
- Single-threaded, stdlib only for the server loop

## Requirements

- Rust 1.75+
- Linux (tested on Debian/Ubuntu)
- Port 443 requires either `sudo` or the `cap_net_bind_service` capability

## Build & Run

```bash
cargo build --release

# Grant port 443 without root (one-time after each build)
sudo setcap cap_net_bind_service=+ep ./target/release/server_runtime

# Run (index.html must be in the working directory)
./target/release/server_runtime
```

## Deploy to a remote host

```bash
scp Cargo.toml Cargo.lock src/main.rs index.html user@host:~/serverRuntime/
scp src/main.rs user@host:~/serverRuntime/src/

ssh user@host "cd ~/serverRuntime && cargo build --release && \
  sudo setcap cap_net_bind_service=+ep ./target/release/server_runtime && \
  nohup ./target/release/server_runtime > server.log 2>&1 &"
```

## TLS

The server generates a self-signed certificate on every startup. Browsers will show a security warning — this is expected. The project will later include a dedicated client that handles certificate validation accordingly.

## Roadmap

- [ ] Persist TLS certificate across restarts
- [ ] Request routing (serve different content per path)
- [ ] Custom client with certificate pinning
- [ ] systemd service unit
- [ ] Automatic IPv6 address discovery via MyFRITZ!
