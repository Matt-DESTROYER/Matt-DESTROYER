use sqlx::{
	SqlitePool,
	sqlite::{
		SqlitePoolOptions,
		SqliteConnectOptions
	},
	Row
};
use rand::{
	Rng,
	distributions::Alphanumeric
};
use std::{
	str::FromStr,
	time::SystemTime
};

use crate::models::{self, AppState};

const DATABASE: &str = "sqlite://database.db";
const SESSION_VALID_TIME: i64 = 7 * 24 * 60 * 60;
const TIME_TILL_LOG_CLEAR: i64 = 30 * 24 * 60 * 60;

pub async fn initialise_db() -> Option<models::AppState> {
	let connection_options = match SqliteConnectOptions::from_str(DATABASE) {
        Ok(con_opts) => con_opts.create_if_missing(true),
        Err(error) => {
            eprintln!("Error: failed to create connection options...");
            eprintln!("{}", error);
            return None
        }
    };

    let db = match SqlitePoolOptions::new()
            .connect_with(connection_options)
            .await {
        Ok(pool) => pool,
        Err(error) => {
            eprintln!("Error: failed to connect to database...");
            eprintln!("{}", error);
            return None
        }
    };

    match sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS sessions (
            id INTEGER PRIMARY KEY,
            session_token TEXT NOT NULL UNIQUE,
            user_id INTEGER NOT NULL,
            expires_at DATETIME NOT NULL,
            FOREIGN KEY(user_id) REFERENCES users(id)
        );

        CREATE TABLE IF NOT EXISTS login_attempts (
            id INTEGER PRIMARY KEY,
            username TEXT NOT NULL,
            ip_address TEXT NOT NULL,
            success BOOLEAN NOT NULL,
            attempted_at INTEGER NOT NULL
        );
        "#
    )
            .execute(&db)
            .await {
        Ok(_) => {},
        Err(error) => {
            eprintln!("Error: failed to ensure database is initialised...");
            eprintln!("{}", error);
            return None
        }
    };

	Some(AppState { db: db })
}

pub async fn create_session(db: &SqlitePool, user_id: i64) -> Option<String> {
    let token: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    let expires_at = match std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH) {
        Ok(time) => time.as_secs() as i64 + SESSION_VALID_TIME,
        Err(error) => {
            eprint!("Error: could not get timestamp");
            eprint!("{}", error);
            return None;
        }
    };

    match sqlx::query("INSERT INTO sessions (session_token, user_id, expires_at) VALUES ($1, $2, $3)")
            .bind(&token)
            .bind(user_id)
            .bind(expires_at)
            .execute(db)
            .await {
        Ok(_) => {},
        Err(error) => {
            eprintln!("Error: could not insert session token into database");
            eprintln!("{}", error);
            return None;
        }
    };

    Some(token)
}

pub async fn verify_session(db: &SqlitePool, token: String) -> Option<i64> {
    let row = match sqlx::query("SELECT user_id, expires_at FROM sessions WHERE session_token = $1")
            .bind(&token)
            .fetch_optional(db)
            .await
            .unwrap_or(None) {
        Some(token) => token,
        None => return None
    };

    let expires_at: i64 = row.get("expires_at");
    let user_id: i64 = row.get("user_id");

    let now = match SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH) {
        Ok(time) => time.as_secs() as i64,
        Err(_) => return None
    };

    if expires_at < now {
        let _ = sqlx::query("DELETE FROM sessions WHERE session_token = $1")
            .bind(&token)
            .execute(db)
            .await;
        return None;
    }

    Some(user_id)
}

pub async fn prune_sessions(db: &SqlitePool) {
    let now = match SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH) {
        Ok(time) => time.as_secs() as i64,
        Err(_) => return
    };

    let _ = sqlx::query("DELETE FROM sessions WHERE expires_at < $1")
        .bind(now)
        .execute(db)
        .await;
}

pub async fn log_attempt(db: &SqlitePool, username: String, ip: String, success: bool) {
    let now = match SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH) {
        Ok(time) => time.as_secs() as i64,
        Err(_) => return
    };

    let _ = sqlx::query("INSERT INTO login_attempts (username, ip_address, success, attempted_at) VALUES ($1, $2, $3, $4)")
        .bind(username)
        .bind(ip)
        .bind(success)
        .bind(now)
        .execute(db)
        .await;
}

pub async fn prune_old_logs(db: &SqlitePool) {
    let retention_limit = match SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH) {
        Ok(time) => time.as_secs() as i64 - TIME_TILL_LOG_CLEAR,
        Err(_) => return
    };

    let _ = sqlx::query("DELETE FROM login_attempts WHERE attempted_at < $1")
        .bind(retention_limit)
        .execute(db)
        .await;
}

