use axum::{
    routing::post,
    Router
};
use tokio::net::TcpListener;
use tower_http::{
    compression::CompressionLayer,
    normalize_path::NormalizePathLayer
};

use std::net::SocketAddr;

mod models;
mod db;
mod handlers;

const PORT: u16 = 3005;

#[tokio::main]
async fn main() {
    let state = match db::initialise_db().await {
        Some(state) => state,
        None => {
            eprintln!("An error occurred initialising the app's state...");
            return
        }
    };

    let app = Router::new()
        .route("/login", post(handlers::login))
        .route("/register", post(handlers::register))
        .route("/session", post(handlers::session))
        .with_state(state)
        .layer(NormalizePathLayer::trim_trailing_slash())
        .layer(
            CompressionLayer::new()
                .br(true)
                .gzip(true)
        );

    let listener: TcpListener = tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], PORT)))
        .await
        .unwrap();

    axum::serve(
        listener,
        app
            .into_make_service_with_connect_info::<SocketAddr>()
    )
        .await
        .unwrap();
}

