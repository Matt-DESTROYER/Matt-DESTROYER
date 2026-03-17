use std::{
    fs,
    net::SocketAddr,
    sync::{
        Arc,
        Mutex
    }
};

use tokio::net::TcpListener;

use axum::{
    body::{
        Body,
        Bytes
    },
    extract::{
        ws::{
            Message,
            WebSocket,
            WebSocketUpgrade
        },
        State
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
    routing::get,
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

use futures_util::{
    stream::SplitSink,
    StreamExt,
    SinkExt
};

use serde_json;

type Clients = Arc<Mutex<Vec<tokio::sync::mpsc::UnboundedSender<String>>>>;

const PORT: u16 = 3000;
const ROOT_DOMAIN: &str = "matthewjames.xyz";

/// Runs the Axum HTTP server serving static pages and a WebSocket endpoint.
///
/// The server serves specific HTML files for "/", "/home", "/about", "/projects", and "/contact",
/// provides a WebSocket endpoint at "/websocket", normalizes trailing slashes, applies a CORS
/// policy allowing GET from any origin, enables gzip and Brotli compression, and returns a
/// preloaded custom 404 HTML page when a route is not found.
///
/// # Examples
///
/// ```no_run
/// // Starts the server (runs indefinitely). Run from a binary, not as a doctest.
/// crate::main();
/// ```
#[tokio::main]
async fn main() {
    let clients: Clients = Arc::new(Mutex::new(Vec::<tokio::sync::mpsc::UnboundedSender<String>>::new()));

    let not_found_html = Bytes::from(
        fs::read("./static/404.html")
            .unwrap()
    );

    let cors_layer: CorsLayer = CorsLayer::new()
        .allow_methods(Method::GET)
        .allow_origin(Any);

    let app: Router = Router::new()
        .route("/websocket", get(ws_handler))
        .with_state(clients)
        .route_service("/", ServeFile::new("./static/home.html"))
        .route_service("/home", ServeFile::new("./static/home.html"))
        .route_service("/about", ServeFile::new("./static/about.html"))
        .route_service("/projects", ServeFile::new("./static/projects.html"))
        .route_service("/contact", ServeFile::new("./static/contact.html"))
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

/// Replaces any 404 response produced by downstream handlers with the provided HTML body.
///
/// If the downstream response has a 404 status, returns a response with status 404 and the
/// given `html` as an `Html` body. Otherwise returns the downstream response unchanged.
///
/// # Parameters
///
/// - `req`: The incoming request forwarded to the next handler.
/// - `next`: The next middleware/handler in the chain to invoke.
/// - `html`: Preloaded HTML bytes used as the body when a 404 is encountered.
///
/// # Examples
///
/// ```no_run
/// use axum::{Router, routing::get, middleware::from_fn_with_state, body::Bytes};
///
/// // assume `custom_404_handler` is in scope and `not_found_html` is Bytes
/// let not_found_html = Bytes::from_static(b"<h1>Not Found</h1>");
///
/// let app = Router::new()
///     .route("/", get(|| async { "root" }))
///     .layer(from_fn_with_state(not_found_html.clone(), |req, next, html| async move {
///         custom_404_handler(req, next, html).await
///     }));
/// ```
async fn custom_404_handler(req: Request<Body>, next: Next, html: Bytes) -> Response {
    let response = next.run(req).await;

    if response.status() == StatusCode::NOT_FOUND {
        return (StatusCode::NOT_FOUND, Html(html)).into_response();
    }

    response
}

/// Upgrades the request to a WebSocket and delegates the connected socket to the connection handler.
///
/// On upgrade, spawns `handle_socket` with the upgraded `WebSocket` and a cloned reference to the shared
/// clients list so the new connection can be tracked and receive broadcasts.
///
/// # Examples
///
/// ```rust,no_run
/// use axum::{routing::get, Router, extract::State};
/// use std::sync::Arc;
/// use tokio::sync::Mutex;
///
/// type Clients = Arc<Mutex<Vec<tokio::sync::mpsc::UnboundedSender<String>>>>;
///
/// // In an Axum app:
/// // let clients: Clients = Arc::new(Mutex::new(Vec::new()));
/// // let app = Router::new().route("/websocket", get(ws_handler)).with_state(clients);
/// ```
async fn ws_handler(ws: WebSocketUpgrade, State(clients): State<Clients>) -> impl IntoResponse {
    return ws.on_upgrade(move |socket| handle_socket(socket, Arc::clone(&clients)));
}

/// Handle a newly established WebSocket connection and manage its lifecycle within the shared client set.
///
/// This function registers the connection with the shared `clients` list, broadcasts the updated client
/// count, then forwards incoming server-side and client-side messages to their respective handlers
/// until the connection closes or a handler indicates termination. On exit it removes the connection
/// from `clients` and broadcasts the updated count.
///
/// # Examples
///
/// ```no_run
/// use std::sync::Arc;
/// use tokio::sync::Mutex;
/// use axum::extract::ws::WebSocket;
///
/// // Create a shared clients list (type alias in the real crate)
/// let clients: Arc<Mutex<Vec<tokio::sync::mpsc::UnboundedSender<String>>>> = Arc::new(Mutex::new(Vec::new()));
///
/// // `socket` would be obtained from an actual WebSocket upgrade in a running server.
/// // Here we only show the intended call site; constructing a real `WebSocket` is omitted.
/// async fn example_use(socket: WebSocket, clients: Arc<Mutex<Vec<tokio::sync::mpsc::UnboundedSender<String>>>>) {
///     // spawn the handler for the connection
///     tokio::spawn(async move {
///         crate::handle_socket(socket, clients).await;
///     });
/// }
/// ```
async fn handle_socket(socket: WebSocket, clients: Clients) {
    let (mut socket_sender, mut socket_receiver) = socket.split();
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<String>();

    clients.lock().unwrap().push(sender.clone());

    broadcast(count(&clients), &clients);

    loop {
        tokio::select! {
            // server messages
            Some(msg) = receiver.recv() => {
                if !handle_server_message(&mut socket_sender, msg/*, &clients*/).await {
                    break;
                }
            }
            // client messages
            Some(Ok(msg)) = socket_receiver.next() => {
                if !handle_client_message(&mut socket_sender, msg, &clients).await {
                    break;
                }
            },
            else => break
        }
    }

    clients.lock().unwrap().retain(|client| !client.same_channel(&sender));

    broadcast(count(&clients), &clients);
}

/// Handles a server-originated WebSocket message and forwards it to the given socket when the message requests a client count.
///
/// Parses `msg` as JSON and, if the JSON object has `"name": "count"`, attempts to send the original message text to `socket`.
///
/// # Parameters
///
/// - `msg`: A JSON-encoded message payload sent from the server to clients (e.g., `{"name":"count","data":N}`).
///
/// # Returns
///
/// `true` if the connection should remain open, `false` if sending to the socket failed and the caller should close the connection.
///
/// # Examples
///
/// ```no_run
/// use axum::extract::ws::{Message, WebSocket};
/// use futures_util::stream::SplitSink;
///
/// # async fn demo(socket: &mut SplitSink<WebSocket, Message>) {
/// let msg = r#"{"name":"count","data":3}"#.to_string();
/// let keep_open = crate::handle_server_message(socket, msg).await;
/// assert!(keep_open);
/// # }
/// ```
async fn handle_server_message(socket: &mut SplitSink<WebSocket, Message>, msg: String/*, clients: &Clients*/) -> bool {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&msg) {
        match json["name"].as_str() {
            Some("count") => {
                if socket.send(Message::Text(msg.into())).await.is_err() {
                    return false;
                }
            },
            _ => {}
        }
    }

    return true;
}

/// Handles an incoming WebSocket message from a client and sends appropriate responses.
///
/// This function processes `Text` messages for simple commands:
/// - "ping" -> replies with "pong"
/// - "heartbeat" -> replies with "heartbeat"
/// - JSON with `"name": "count"` -> replies with the current client count as JSON
/// It treats a `Close` message as a signal to terminate the connection.
///
/// # Returns
///
/// `true` to keep the connection open, `false` to close the connection.
///
/// # Examples
///
/// ```no_run
/// # use tokio_tungstenite::tungstenite::Message;
/// # use tokio_tungstenite::WebSocketStream;
/// # use futures::stream::SplitSink;
/// # async fn example(socket: &mut SplitSink<WebSocketStream<tokio::net::TcpStream>, Message>, clients: &crate::Clients) {
/// // Send a ping text message to the handler
/// let keep_alive = crate::handle_client_message(socket, Message::Text("ping".into()), clients).await;
/// assert!(keep_alive);
/// # }
/// ```
async fn handle_client_message(socket: &mut SplitSink<WebSocket, Message>, msg: Message, clients: &Clients) -> bool {
    match msg {
        Message::Text(text) => {
            match text.as_str() {
                "ping" => {
                    if socket.send(Message::Text("pong".into())).await.is_err() {
                        return false;
                    }
                },
                "heartbeat" => {
                    if socket.send(Message::Text("heartbeat".into())).await.is_err() {
                        return false;
                    }
                },
                json_string => {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_string) {
                        match json["name"].as_str() {
                            Some("count") => {
                                if socket.send(Message::Text(count(&clients).into())).await.is_err() {
                                    return false;
                                }
                            },
                            _ => {}
                        }
                    }
                }
            }
        },
        Message::Close(_) => {
            return false;
        },
        _ => {}
    }

    return true;
}

fn count(clients: &Clients) -> String {
    let clients = clients.lock().unwrap();
    return format!("{{\"name\":\"count\",\"data\":{}}}", clients.len());
}

fn broadcast(msg: String, clients: &Clients) {
    let mut clients = clients.lock().unwrap();
    clients.retain(|client| client.send(msg.clone()).is_ok());
}

