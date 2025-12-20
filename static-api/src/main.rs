use std::{
    fs,
    net::SocketAddr,
    sync::Arc
};

use tokio::net::TcpListener;

use axum::{
    body::Body,
    http::{
        Request,
        StatusCode
    },
    middleware::{
        self,
        Next
    },
    response::{
        Html,
        IntoResponse,
        Response
    },
    Router
};

use tower_http::{
    cors::{
        Any,
        CorsLayer
    },
    services::ServeDir
};

const PORT: u16 = 3002; // static.matthewjames.xyz

#[tokio::main]
async fn main() {
    let not_found_html = Arc::new(
        fs::read_to_string("./static/404.html")
            .unwrap_or_else(|_| "<h1>404 Not Found</h1>".to_string())
    );

    let cors_layer: CorsLayer = CorsLayer::new()
        .allow_methods(Any)
        .allow_origin(Any);

    let app = Router::new()
        .layer(cors_layer)
        .fallback_service(ServeDir::new("./static"))
        .layer(middleware::from_fn(move |req, next| {
            custom_404_handler(req, next, not_found_html.clone())
        }));

    let listener: TcpListener = tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], PORT)))
        .await
        .unwrap();

    axum::serve(listener, app)
        .await
        .unwrap();
}

async fn custom_404_handler(req: Request<Body>, next: Next, html: Arc<String>) -> Response {
    let response = next.run(req).await;

    if response.status() == StatusCode::NOT_FOUND {
        return (StatusCode::NOT_FOUND, Html(html.as_str().to_string())).into_response();
    }

    response
}
