use axum::{
    extract::{
        ConnectInfo,
        State
    },
    http::HeaderMap,
    Json,
};
use argon2::{
    password_hash::{
        PasswordHasher,
        SaltString,
        PasswordVerifier,
        PasswordHash
    },
    Argon2
};
use rand::rngs::OsRng;
use sqlx::Row;

use std::net::SocketAddr;

use crate::models::{
	AppState,
	RegisterRequest,
	RegisterResponse,
	LoginRequest,
	LoginResponse,
	VerifySessionRequest,
	VerifySessionResponse
};

use crate::db::{
	create_session,
	prune_sessions,
	verify_session,
	log_attempt,
	prune_old_logs
};

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>
) -> Json<RegisterResponse> {
    let salt = SaltString::generate(&mut OsRng);
    let hashed_pw = match Argon2::default()
            .hash_password(payload.password.as_bytes(), &salt) {
        Ok(hashed_pw) => hashed_pw.to_string(),
        Err(_) => return Json(RegisterResponse {
            success: false,
            message: "Failed to hash password".into(),
            session_id: None
        })
    };

    let insert_result = sqlx::query("INSERT INTO users (username, password_hash) VALUES ($1, $2) RETURNING id")
        .bind(&payload.username)
        .bind(&hashed_pw)
        .fetch_one(&state.db)
        .await;

    let user_id: i64 = match insert_result {
        Ok(row) => row.get("id"),
        Err(_) => return Json(RegisterResponse {
            success: false,
            message: "Username taken".into(),
            session_id: None
        })
    };

    let token = create_session(&state.db, user_id).await;

    Json(RegisterResponse {
        success: true,
        message: "Registered successfully!".into(),
        session_id: token
    })
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<LoginRequest>
) -> Json<LoginResponse> {
    let ip = headers
        .get("CF-Connecting-IP")
        .and_then(|h| h.to_str().ok())
        .unwrap_or(addr.ip().to_string().as_str())
        .to_string();

    let row = sqlx::query("SELECT id, password_hash FROM users WHERE username = $1")
        .bind(&payload.username)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

    let user = match row {
        Some(row) => row,
        None => {
            let db_clone = state.db.clone();
            let username = payload.username.clone();
			let ip = ip.clone();
            tokio::spawn(async move {
                log_attempt(&db_clone, username, ip, false).await;
            });

            return Json(LoginResponse {
                success: false,
                message: "User not found".into(),
                session_id: None
            })
        }
    };

    let stored_hash: String = user.get("password_hash");
    let user_id: i64 = user.get("id");

    let parsed_hash = match PasswordHash::new(&stored_hash) {
        Ok(hash) => hash,
        Err(_) => {
            let db_clone = state.db.clone();
            let username = payload.username.clone();
			let ip = ip.clone();
            tokio::spawn(async move {
                log_attempt(&db_clone, username, ip, false).await;
            });

            return Json(LoginResponse {
                success: false,
                message: "Failed to parse saved password hash".into(),
                session_id: None
            })
        }
    };

    let is_valid = Argon2::default()
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .is_ok();

    if !is_valid {
        return Json(LoginResponse {
            success: false,
            message: "Incorrect password".into(),
            session_id: None
        });
    } else  {
        let db_clone = state.db.clone();
        let username = payload.username.clone();
		let ip = ip.clone();
        tokio::spawn(async move {
            log_attempt(&db_clone, username, ip, false).await;
        });
    }

    {
        let db_clone = state.db.clone();
        let username = payload.username.clone();
		let ip = ip.clone();
        tokio::spawn(async move {
            log_attempt(&db_clone, username, ip, false).await;
            prune_old_logs(&db_clone).await;
            prune_sessions(&db_clone).await;
        });
    }

    let token = create_session(&state.db, user_id).await;

    Json(LoginResponse {
        success: true,
        message: "Logged in successfully".into(),
        session_id: token
    })
}

pub async fn session(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(payload): Json<VerifySessionRequest>
) -> Json<VerifySessionResponse> {
	let ip = headers
        .get("CF-Connecting-IP")
        .and_then(|h| h.to_str().ok())
        .unwrap_or(addr.ip().to_string().as_str())
        .to_string();

	let valid_session = match verify_session(&state.db, payload.token).await {
		Some(_) => true,
		None => false
	};

	tokio::spawn(async move {
		//log_attempt(&state.db, username, ip, valid_session).await;
		prune_old_logs(&state.db).await;
		prune_sessions(&state.db).await;
	});

	Json(VerifySessionResponse {
		success: valid_session
	})
}

