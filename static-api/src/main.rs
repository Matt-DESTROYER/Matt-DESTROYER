use std::{
    fs,
    net::SocketAddr
};

use tokio::net::TcpListener;

use axum::{
    http::{
        header,
        Method
    },
    response::{
        Html,
        IntoResponse
    },
    routing::{
        get_service,
        MethodRouter
    },
    Router
};

use tower_http::{
    cors::{
        AllowOrigin,
        CorsLayer
    },
    services::ServeDir
};

const PORT: u16 = 3002; // static.matthewjames.xyz

#[tokio::main]
async fn main() {
    let serve_dir: MethodRouter = get_service(ServeDir::new("./static"))
        .handle_error(|_| async {
            match fs::read_to_string("./static/404.html") {
                Ok(contents) => Html(contents).into_response(),
                Err(_) => (axum::http::StatusCode::NOT_FOUND, "404 Not Found").into_response()
            }
        });

    let cors_layer: CorsLayer = CorsLayer::new()
        .allow_origin(AllowOrigin::list([
            "https://matthewjames.xyz".parse().unwrap(),
            "https://projects.matthewjames.xyz".parse().unwrap()
        ]))
        .allow_methods([Method::GET])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    let app = Router::new()
        .fallback_service(serve_dir)
        .layer(cors_layer);

    let listener: TcpListener = tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], PORT)))
        .await
        .unwrap();

    axum::serve(listener, app)
        .await
        .unwrap();
}
