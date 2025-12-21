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
        Any,
        CorsLayer
    },
    services::{
        ServeDir,
        ServeFile
    }
};

const PORT: u16 = 3001; // projects.matthewjames.xyz

#[tokio::main]
async fn main() {
    let not_found_html = Bytes::from(
        fs::read("./static/404.html")
            .unwrap()
    );
    let cors_layer: CorsLayer = CorsLayer::new()
        .allow_methods(Method::GET)
        .allow_origin(Any);

    let app = Router::new()
        .route_service("/", ServeFile::new("./static/projects.html"))
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

async fn custom_404_handler(req: Request<Body>, next: Next, html: Bytes) -> Response {
    let response = next.run(req).await;

    if response.status() == StatusCode::NOT_FOUND {
        return (StatusCode::NOT_FOUND, Html(html)).into_response();
    }

    response
}
