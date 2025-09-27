use axum::{
    extract::ws::{
        Message,
        WebSocket,
        WebSocketUpgrade
    },
    extract::State,
    response::{
        Html,
        IntoResponse,
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
    sync::{
        Arc,
        Mutex
    },
    fs,
    net::SocketAddr
};

use futures_util::{
    stream::SplitSink,
    StreamExt,
    SinkExt
};

use serde_json::Value;

type Clients = Arc<Mutex<Vec<tokio::sync::mpsc::UnboundedSender<String>>>>;

#[tokio::main]
async fn main() {
    let clients: Clients = Arc::new(Mutex::new(Vec::<tokio::sync::mpsc::UnboundedSender<String>>::new()));

    let serve_dir = get_service(ServeDir::new("./public"))
        .handle_error(|_| async {
            match fs::read_to_string("./public/404.html") {
                Ok(contents) => Html(contents).into_response(),
                Err(_) => (axum::http::StatusCode::NOT_FOUND, "404 Not Found").into_response()
            }
        });

    let app = Router::new()
        .route("/socket", get(ws_handler))
        .with_state(clients)
        .route_service("/", ServeFile::new("./public/home.html"))
        .route_service("/home", ServeFile::new("./public/home.html"))
        .route_service("/Home", ServeFile::new("./public/home.html"))
        .route_service("/about", ServeFile::new("./public/about.html"))
        .route_service("/About", ServeFile::new("./public/about.html"))
        .route_service("/projects", ServeFile::new("./public/projects.html"))
        .route_service("/Projects", ServeFile::new("./public/projects.html"))
        .route_service("/contact", ServeFile::new("./public/contact.html"))
        .route_service("/Contact", ServeFile::new("./public/contact.html"))
        .fallback_service(serve_dir);

    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 3002)))
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
                if !handle_server_message(&mut socket_sender, msg, &clients).await {
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

async fn handle_server_message(socket: &mut SplitSink<WebSocket, Message>, msg: String, clients: &Clients) -> bool {
    let json: Value = serde_json::from_str(&msg).unwrap();

    match json["name"].as_str() {
        Some("count") => {
            if socket.send(Message::Text(count(clients).into())).await.is_err() {
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
    let clients = clients.lock().unwrap();
    for client in clients.iter() {
        let _ = client.send(msg.clone());
    }
}

