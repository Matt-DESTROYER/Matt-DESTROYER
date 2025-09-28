use axum:: {
    response::{
        Html,
        IntoResponse
    },
    routing::{
        get,
        get_service
    },
    Router
};

use tower_http::services::{
    ServeDir,
    ServeFile
};

use std::{
    fs,
    net::SocketAddr
};

const PORT: u16 = 3002;

#[tokio::main]
async fn main() {
    let serve_dir = get_service(ServeDir::new("./static"))
        .handle_error(|_| async {
            match fs::read_to_string("./static/404.html") {
                Ok(contents) => Html(contents).into_response(),
                Err(_) => (axum::http::StatusCode::NOT_FOUND, "404 Not Found").into_response()
            }
        });

    let app = Router::new()
        .route_service("/", ServeFile::new("./static/projects.html"))
        .fallback_service(serve_dir);

    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], PORT)))
        .await
        .unwrap();

    axum::serve(listener, app)
        .await
        .unwrap();
}

