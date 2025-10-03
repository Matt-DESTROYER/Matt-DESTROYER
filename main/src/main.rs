use std::{
    sync::{
        Arc,
        Mutex
    },
    fs,
    net::SocketAddr
};

use tokio::net::TcpListener;

use axum::{
    extract::{
        ws::{
            Message,
            WebSocket,
            WebSocketUpgrade
        },
        State
    },
    http::{
        header,
        Method
    },
    response::{
        Html,
        IntoResponse
    },
    routing::{
        get,
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

use serde_json::Value;

type Clients = Arc<Mutex<Vec<tokio::sync::mpsc::UnboundedSender<String>>>>;

const PORT: u16 = 3000; // matthewjames.xyz

#[tokio::main]
async fn main() {
    let clients: Clients = Arc::new(Mutex::new(Vec::<tokio::sync::mpsc::UnboundedSender<String>>::new()));

    let serve_dir: MethodRouter = get_service(ServeDir::new("./static"))
        .handle_error(|_| async {
            match fs::read_to_string("./static/404.html") {
                Ok(contents) => Html(contents).into_response(),
                Err(_) => (axum::http::StatusCode::NOT_FOUND, "404 Not Found").into_response()
            }
        });

    let cors_layer: CorsLayer = CorsLayer::new()
        .allow_origin(AllowOrigin::list([
            "https://static.matthewjames.xyz".parse().unwrap()
        ]))
        .allow_methods([Method::GET])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]);

    let app: Router = Router::new()
        .route("/websocket", get(ws_handler))
        .with_state(clients)
        .route_service("/", ServeFile::new("./static/home.html"))
        .route_service("/home", ServeFile::new("./static/home.html"))
        .route_service("/Home", ServeFile::new("./static/home.html"))
        .route_service("/about", ServeFile::new("./static/about.html"))
        .route_service("/About", ServeFile::new("./static/about.html"))
        .route_service("/projects", ServeFile::new("./static/projects.html"))
        .route_service("/Projects", ServeFile::new("./static/projects.html"))
        .route_service("/contact", ServeFile::new("./static/contact.html"))
        .route_service("/Contact", ServeFile::new("./static/contact.html"))
        .fallback_service(serve_dir)
        .layer(cors_layer);

    let listener: TcpListener = tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], PORT)))
        .await
        .unwrap();

    axum::serve(listener, app)
        .await
        .unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade, State(clients): State<Clients>) -> impl IntoResponse {
    return ws.on_upgrade(move |socket| handle_socket(socket, Arc::clone(&clients)));
}

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

async fn handle_server_message(socket: &mut SplitSink<WebSocket, Message>, msg: String/*, clients: &Clients*/) -> bool {
    let json: Value = serde_json::from_str(&msg).unwrap();

    match json["name"].as_str() {
        Some("count") => {
            if socket.send(Message::Text(msg.into())).await.is_err() {
                return false;
            }
        },
        _ => {}
    }

    return true;
}

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
                    let json: Value = serde_json::from_str(json_string).unwrap();

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

