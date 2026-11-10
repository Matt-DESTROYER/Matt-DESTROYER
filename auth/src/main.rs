use axum::{
    Router, ServiceExt,
    body::Body,
    http::{HeaderValue, Method, Request, header::CONTENT_TYPE},
    routing::post,
};
use tokio::net::TcpListener;
use tower::Layer;

use tower_http::{cors::CorsLayer, normalize_path::NormalizePathLayer};

use std::net::SocketAddr;

mod db;
mod handlers;
mod models;

const PORT: u16 = 3005;

#[tokio::main]
async fn main() {
    let state = match db::initialise_db().await {
        Some(state) => state,
        None => {
            eprintln!("An error occurred initialising the app's state...");
            std::process::exit(1);
        }
    };

    let cors_layer = CorsLayer::new()
        .allow_origin("https://matthewjames.xyz".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::POST])
        .allow_headers([CONTENT_TYPE])
        .allow_credentials(true);

    let router = Router::new()
        .route("/login", post(handlers::login))
        .route("/register", post(handlers::register))
        .route("/session", post(handlers::session))
        .with_state(state)
        .layer(cors_layer);

    let app = NormalizePathLayer::trim_trailing_slash().layer(router);

    let listener: TcpListener =
        tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], PORT)))
            .await
            .unwrap();

    axum::serve(
        listener,
        ServiceExt::<Request<Body>>::into_make_service_with_connect_info::<SocketAddr>(app),
    )
    .await
    .unwrap();
}
