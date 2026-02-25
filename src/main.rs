use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    // Map of client ID to their message sender
    // RwLock allows multiple readers but exclusive writers
    pub clients: Arc<RwLock<HashMap<String, ClientInfo>>>,
    // Broadcast channel for sending messages to all clients
    pub broadcast_tx: broadcast::Sender<BroadcastMessage>,
}

pub struct ClientInfo {
    pub id: String,
    pub username: String,
    pub connected_at: std::time::Instant,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct BroadcastMessage {
    pub sender_id: String,
    pub username: String,
    pub content: String,
    pub timestamp: u64,
}

use axum::{
    extract::{State, WebSocketUpgrade},
    extract::ws::{Message, WebSocket},
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};

#[tokio::main]
async fn main() {
    // Initialize logging for debugging connection issues
    tracing_subscriber::init();

    // Create broadcast channel with buffer for 100 messages
    // If a slow client falls behind by 100+ messages, they'll miss some
    let (broadcast_tx, _) = broadcast::channel(100);

    let state = AppState {
        clients: Arc::new(RwLock::new(HashMap::new())),
        broadcast_tx,
    };

    let app = Router::new()
        .route("/ws", get(websocket_handler))
        .route("/health", get(|| async { "OK" }))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("Server running on port 3000");
    axum::serve(listener, app).await.unwrap();
}