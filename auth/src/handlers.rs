use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use axum::{
    Json,
    extract::{ConnectInfo, State},
    http::{HeaderMap, header},
};
use rand::rngs::OsRng;
use sqlx::{Pool, Row, Sqlite};

use std::net::{IpAddr, SocketAddr};

use crate::models::{
    AppState, LoginRequest, LoginResponse, RegisterRequest, RegisterResponse, VerifySessionRequest,
    VerifySessionResponse,
};

use crate::db::{create_session, log_attempt, prune_old_logs, prune_sessions, verify_session};

fn get_client_ip_raw(headers: &HeaderMap) -> Option<&str> {
    Some(headers.get("CF-Connecting-IP")?.to_str().ok()?)
}

/*
fn get_client_ip(headers: &HeaderMap) -> Option<IpAddr> {
    headers
        .get("CF-Connecting-IP")?
        .to_str()
        .ok()?
        .parse::<IpAddr>()
        .ok()
}
*/

fn log_attempt_and_prune(db: &Pool<Sqlite>, username: &String, ip: &String, success: bool) {
    let db_clone = db.clone();
    let username = username.clone();
    let ip = ip.clone();
    tokio::spawn(async move {
        log_attempt(&db_clone, username, ip, success).await;
        prune_old_logs(&db_clone).await;
    });
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Json<RegisterResponse> {
    if payload.username.trim() != payload.username {
        return Json(RegisterResponse {
            success: false,
            message: "Username has leading or trailing whitespace".into(),
            session_id: None,
        });
    } else if payload.password.trim() != payload.password {
        return Json(RegisterResponse {
            success: false,
            message: "Password has leading or trailing whitespace".into(),
            session_id: None,
        });
    }

    if payload.username.len() < 3 {
        return Json(RegisterResponse {
            success: false,
            message: "Username too short".into(),
            session_id: None,
        });
    } else if payload.username.len() > 50 {
        return Json(RegisterResponse {
            success: false,
            message: "Username too long".into(),
            session_id: None,
        });
    }

    if payload.password.len() < 8 {
        return Json(RegisterResponse {
            success: false,
            message: "Password too short".into(),
            session_id: None,
        });
    } else if payload.password.len() > 100 {
        return Json(RegisterResponse {
            success: false,
            message: "Password too long".into(),
            session_id: None,
        });
    }

    let salt = SaltString::generate(&mut OsRng);
    let hashed_pw = match Argon2::default().hash_password(payload.password.as_bytes(), &salt) {
        Ok(hashed_pw) => hashed_pw.to_string(),
        Err(_) => {
            return Json(RegisterResponse {
                success: false,
                message: "Failed to hash password".into(),
                session_id: None,
            });
        }
    };

    let insert_result =
        sqlx::query("INSERT INTO users (username, password_hash) VALUES ($1, $2) RETURNING id")
            .bind(&payload.username)
            .bind(&hashed_pw)
            .fetch_one(&state.db)
            .await;

    let user_id: i64 = match insert_result {
        Ok(row) => row.get("id"),
        Err(_) => {
            return Json(RegisterResponse {
                success: false,
                message: "Username taken".into(),
                session_id: None,
            });
        }
    };

    let token: Option<String> = create_session(&state.db, user_id).await;

    Json(RegisterResponse {
        success: true,
        message: "Registered successfully!".into(),
        session_id: token,
    })
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<LoginRequest>,
) -> Json<LoginResponse> {
    let ip = match get_client_ip_raw(&headers) {
        Some(ip) => ip.to_owned(),
        None => {
            return Json(LoginResponse {
                success: false,
                message: "Failed to obtain IP".into(),
                session_id: None,
            });
        }
    };

    let user = match sqlx::query("SELECT id, password_hash FROM users WHERE username = $1")
        .bind(&payload.username)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None)
    {
        Some(row) => row,
        None => {
            log_attempt_and_prune(&state.db, &payload.username, &ip, false);

            return Json(LoginResponse {
                success: false,
                message: "Invalid username or password".into(),
                session_id: None,
            });
        }
    };

    let stored_hash: String = user.get("password_hash");
    let user_id: i64 = user.get("id");

    let parsed_hash = match PasswordHash::new(&stored_hash) {
        Ok(hash) => hash,
        Err(_) => {
            // should we log if it's a server error?
            // maybe it's caused by a malformed user request?
            log_attempt_and_prune(&state.db, &payload.username, &ip, false);

            return Json(LoginResponse {
                success: false,
                message: "Failed to parse saved password hash".into(),
                session_id: None,
            });
        }
    };

    let is_valid = Argon2::default()
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .is_ok();

    if !is_valid {
        log_attempt_and_prune(&state.db, &payload.username, &ip, false);

        return Json(LoginResponse {
            success: false,
            message: "Invalid username or password".into(),
            session_id: None,
        });
    }

    log_attempt_and_prune(&state.db, &payload.username, &ip, true);

    let token = create_session(&state.db, user_id).await;

    Json(LoginResponse {
        success: true,
        message: "Logged in successfully".into(),
        session_id: token,
    })
}

pub async fn session(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<VerifySessionRequest>,
) -> Json<VerifySessionResponse> {
    let ip = match get_client_ip_raw(&headers) {
        Some(ip) => ip.to_owned(),
        None => return Json(VerifySessionResponse { success: false }),
    };

    let valid_session = match verify_session(&state.db, payload.token, payload.user_id).await {
        Some(_) => true,
        None => false,
    };

    tokio::spawn(async move {
        prune_sessions(&state.db).await;
    });

    Json(VerifySessionResponse {
        success: valid_session,
    })
}
