use std::{
    fs,
    net::SocketAddr
};

use tokio::net::TcpListener;

use axum:: {
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

use tower_http::services::ServeDir;

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

    let app = Router::new()
        .fallback_service(serve_dir);

    let listener: TcpListener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], PORT)))
        .await
        .unwrap();

    axum::serve(listener, app)
        .await
        .unwrap();
}
