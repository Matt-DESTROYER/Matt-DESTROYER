use std::{
    fs,
    net::SocketAddr
};

use tokio::net::TcpListener;

use axum::{
    body::{
        Body,
        Bytes
    },
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
    normalize_path::NormalizePathLayer,
    services::ServeDir
};

const PORT: u16 = 3002;
const ROOT_DOMAIN: &str = "matthewjames.xyz";

#[tokio::main]
async fn main() {
    let not_found_html = Bytes::from(
        fs::read("./static/404.html")
            .unwrap()
    );

    let cors_layer: CorsLayer = {
        let root = format!("://{}", ROOT_DOMAIN);
        let subdomain = format!(".{}", ROOT_DOMAIN);
        let port_suffix = format!(".{}:", ROOT_DOMAIN);
        let port_col = format!("://{}:", ROOT_DOMAIN);

        CorsLayer::new()
            .allow_methods(Method::GET)
            .allow_origin(AllowOrigin::predicate(move |origin: &HeaderValue, _request_parts| {
                let origin = origin.to_str().unwrap_or("");

                if origin.ends_with(&root) || origin.ends_with(&subdomain) {
                    return true;
                }

                if origin.contains(&port_suffix) || origin.contains(&port_col) {
                    return true;
                }

                false
            }))
    };

    let app = Router::new()
        .fallback_service(ServeDir::new("./static"))
        .layer(NormalizePathLayer::trim_trailing_slash())
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

async fn custom_404_handler(req: Request<Body>, next: Next, html: Bytes) -> Response {
    let response = next.run(req).await;

    if response.status() == StatusCode::NOT_FOUND {
        return (StatusCode::NOT_FOUND, Html(html)).into_response();
    }

    response
}
