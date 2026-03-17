use sqlx::{
    SqlitePool,
    FromRow
};
use serde::{
    Deserialize,
    Serialize
};

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool
}

#[derive(Debug, FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
}

#[derive(Debug, FromRow)]
pub struct Session {
    pub id: i64,
    pub session_token: String,
    pub user_id: i64,
    pub expires_at: i64
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String
}

#[derive(Serialize)]
pub struct RegisterResponse {
    pub success: bool,
    pub message: String,
    pub session_id: Option<String>
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
    pub session_id: Option<String>
}

#[derive(Deserialize)]
pub struct VerifySessionRequest {
    pub token: String
}

#[derive(Serialize)]
pub struct VerifySessionResponse {
    pub success: bool
}

