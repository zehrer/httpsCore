use axum::{extract::State, response::IntoResponse, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;
use url::Url;
use webauthn_rs::prelude::*;

const RP_ID: &str = "agent.homenodes.io";
const RP_ORIGIN: &str = "https://agent.homenodes.io";
const RP_NAME: &str = "HomeNode";

// ── State ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    webauthn: Arc<Webauthn>,
    reg_challenges:  Arc<Mutex<HashMap<String, (Uuid, PasskeyRegistration)>>>,
    auth_challenges: Arc<Mutex<HashMap<String, PasskeyAuthentication>>>,
    users:           Arc<RwLock<HashMap<String, (Uuid, Vec<Passkey>)>>>,
}

impl AppState {
    pub fn new() -> Arc<Self> {
        let origin = Url::parse(RP_ORIGIN).expect("Invalid RP origin");
        let webauthn = WebauthnBuilder::new(RP_ID, &origin)
            .expect("Invalid WebAuthn config")
            .rp_name(RP_NAME)
            .build()
            .expect("Failed to build WebAuthn");

        Arc::new(Self {
            webauthn: Arc::new(webauthn),
            reg_challenges:  Arc::new(Mutex::new(HashMap::new())),
            auth_challenges: Arc::new(Mutex::new(HashMap::new())),
            users:           Arc::new(RwLock::new(HashMap::new())),
        })
    }
}

// ── Request / Response types ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct StartRequest {
    pub username: String,
}

#[derive(Deserialize)]
pub struct RegisterFinishRequest {
    pub username:   String,
    pub credential: RegisterPublicKeyCredential,
}

#[derive(Deserialize)]
pub struct LoginFinishRequest {
    pub username:   String,
    pub credential: PublicKeyCredential,
}

#[derive(Serialize)]
struct StatusResponse {
    success: bool,
    message: String,
}

impl StatusResponse {
    fn ok(msg: &str)  -> (StatusCode, Json<Self>) {
        (StatusCode::OK, Json(Self { success: true,  message: msg.into() }))
    }
    fn err(msg: &str) -> (StatusCode, Json<Self>) {
        (StatusCode::BAD_REQUEST, Json(Self { success: false, message: msg.into() }))
    }
}

// ── Handlers ─────────────────────────────────────────────────────────────────

pub async fn register_start(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StartRequest>,
) -> axum::response::Response {
    let username = req.username.to_lowercase();

    let (user_id, existing) = {
        let users = state.users.read().await;
        match users.get(&username) {
            Some((id, keys)) => (
                *id,
                Some(keys.iter().map(|k| k.cred_id().clone()).collect()),
            ),
            None => (Uuid::new_v4(), None),
        }
    };

    match state.webauthn.start_passkey_registration(user_id, &username, &username, existing) {
        Ok((ccr, reg_state)) => {
            state.reg_challenges.lock().await.insert(username, (user_id, reg_state));
            Json(ccr).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn register_finish(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterFinishRequest>,
) -> axum::response::Response {
    let username = req.username.to_lowercase();

    let (user_id, reg_state) = match state.reg_challenges.lock().await.remove(&username) {
        Some(s) => s,
        None => return StatusResponse::err("No pending registration for this user").into_response(),
    };

    match state.webauthn.finish_passkey_registration(&req.credential, &reg_state) {
        Ok(passkey) => {
            let mut users = state.users.write().await;
            users.entry(username)
                .and_modify(|(_, keys)| keys.push(passkey.clone()))
                .or_insert_with(|| (user_id, vec![passkey]));
            StatusResponse::ok("Passkey registered successfully").into_response()
        }
        Err(e) => StatusResponse::err(&e.to_string()).into_response(),
    }
}

pub async fn login_start(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StartRequest>,
) -> axum::response::Response {
    let username = req.username.to_lowercase();

    let creds: Vec<Passkey> = {
        let users = state.users.read().await;
        match users.get(&username) {
            Some((_, keys)) => keys.clone(),
            None => return (StatusCode::NOT_FOUND, "User not found").into_response(),
        }
    };

    match state.webauthn.start_passkey_authentication(&creds) {
        Ok((rcr, auth_state)) => {
            state.auth_challenges.lock().await.insert(username, auth_state);
            Json(rcr).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn login_finish(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginFinishRequest>,
) -> axum::response::Response {
    let username = req.username.to_lowercase();

    let auth_state = match state.auth_challenges.lock().await.remove(&username) {
        Some(s) => s,
        None => return StatusResponse::err("No pending login for this user").into_response(),
    };

    match state.webauthn.finish_passkey_authentication(&req.credential, &auth_state) {
        Ok(_) => StatusResponse::ok("Login successful").into_response(),
        Err(e) => StatusResponse::err(&e.to_string()).into_response(),
    }
}
