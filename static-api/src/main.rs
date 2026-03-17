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

/// Starts the HTTP server configured with static file serving, a custom 404 page, CORS rules, path normalization, and response compression.
///
/// This binary entrypoint:
/// - Loads the pre-rendered 404 HTML into memory and uses it for requests that resolve to 404.
/// - Serves files from the `./static` directory as a fallback for unmatched routes.
/// - Applies a CORS policy that allows GET requests from origins matching the configured root domain and its subdomains or port-suffixed variants.
/// - Normalizes request paths by trimming trailing slashes and applies a middleware that substitutes 404 responses with the loaded HTML.
/// - Enables brotli and gzip compression for responses and binds the server to 0.0.0.0 on the configured `PORT`.
///
/// # Examples
///
/// ```ignore
/// // Run the server (binds to 0.0.0.0:PORT)
/// fn main() {
///     // cargo run --bin your_binary_name
/// }
/// ```
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

/// Replaces a 404 response with a provided HTML body while passing through non-404 responses unchanged.
///
/// If the downstream service returns `StatusCode::NOT_FOUND`, this middleware returns a response
/// with status 404 and the given HTML bytes as the body; otherwise it returns the downstream response.
///
/// # Parameters
/// - `html`: Preloaded HTML content used as the body when a 404 response is produced.
///
/// # Returns
/// A `Response` containing either the original downstream response or a 404 response whose body is `html`.
///
/// # Examples
/// ```rust,no_run
/// use axum::{Router, body::Bytes, middleware};
///
/// // assume `custom_404_handler` is in scope and `not_found_html` is Bytes
/// let not_found_html = Bytes::from_static(b"<h1>Not Found</h1>");
/// let app = Router::new()
///     .route_layer(middleware::from_fn_with_state(
///         not_found_html.clone(),
///         |req, next, html| async move { custom_404_handler(req, next, html).await },
///     ));
/// ```
async fn custom_404_handler(req: Request<Body>, next: Next, html: Bytes) -> Response {
    let response = next.run(req).await;

    if response.status() == StatusCode::NOT_FOUND {
        return (StatusCode::NOT_FOUND, Html(html)).into_response();
    }

    response
}
