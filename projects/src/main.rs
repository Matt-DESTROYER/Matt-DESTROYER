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
    normalize_path::NormalizePathLayer,
    services::{
        ServeDir,
        ServeFile
    }
};

const PORT: u16 = 3001; // projects.matthewjames.xyz

/// Starts the HTTP server that serves static files and applies middleware for
/// path normalization, CORS, compression, and a custom 404 page.
///
/// The server binds to 0.0.0.0:PORT and:
/// - serves "./static/projects.html" at the root path ("/"),
/// - serves other static files from "./static",
/// - trims trailing slashes from request paths,
/// - replaces downstream 404 responses with the contents of "./static/404.html",
/// - allows GET requests from any origin via CORS,
/// - enables Brotli and gzip compression.
///
/// # Examples
///
/// ```
/// # // Run the compiled binary to start the server:
/// # // $ cargo run
/// ```
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

/// Replaces downstream 404 responses with the provided HTML 404 page.
///
/// When the downstream handler returns a response with status `404 Not Found`,
/// this middleware returns a `404` response whose body is the given HTML bytes;
/// otherwise it forwards the downstream response unchanged.
///
/// # Examples
///
/// ```no_run
/// use axum::{Router, routing::get, body::Bytes};
/// // register `custom_404_handler` as a middleware that receives the preloaded 404 HTML
/// let not_found_html = Bytes::from_static(b"<h1>Not Found</h1>");
/// let app = Router::new()
///     .route("/", get(|| async { "ok" }))
///     .layer(axum::middleware::from_fn_with_state(not_found_html.clone(), custom_404_handler));
/// ```
async fn custom_404_handler(req: Request<Body>, next: Next, html: Bytes) -> Response {
    let response = next.run(req).await;

    if response.status() == StatusCode::NOT_FOUND {
        return (StatusCode::NOT_FOUND, Html(html)).into_response();
    }

    response
}
