use std::{
    fs,
    net::SocketAddr,
    sync::Arc
};

use tokio::net::TcpListener;

use axum::{
    body::Body,
    http::{
        HeaderValue,
        Method,
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
    compression::CompressionLayer,
    cors::{
        AllowOrigin,
        CorsLayer
    },
    services::ServeDir
};

const PORT: u16 = 3002;
const ROOT_DOMAIN: &str = "matthewjames.xyz";

#[tokio::main]
async fn main() {
    let not_found_html = Arc::new(
        fs::read_to_string("./static/404.html")
            .unwrap_or_else(|_| "<h1>404 Not Found</h1>".to_string())
    );

    let cors_layer: CorsLayer = CorsLayer::new()
        .allow_methods(Method::GET)
        .allow_origin(AllowOrigin::predicate(|origin: &HeaderValue, _request_parts| {
            let origin = origin.to_str().unwrap_or("");

            if origin.ends_with(&format!("://{}", ROOT_DOMAIN)) || origin.ends_with(&format!(".{}", ROOT_DOMAIN)) {
                return true;
            }

            if origin.contains(&format!(".{}:", ROOT_DOMAIN)) || origin.contains(&format!("://{}:", ROOT_DOMAIN)) {
                return true;
            }

            false
        }));

    let app = Router::new()
        .fallback_service(ServeDir::new("./static"))
        .layer(middleware::from_fn(move |req, next| {
            custom_404_handler(req, next, not_found_html.clone())
        }))
        .layer(cors_layer)
        .layer(
            CompressionLayer::new()
                .br(true)
                .gzip(true)
        );

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
