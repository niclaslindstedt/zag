//! WebSocket handler for single-session event streaming.

use axum::{
    extract::{Path, Query, WebSocketUpgrade, ws::Message},
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};

use crate::types::StreamQuery;

/// GET /api/v1/sessions/:id/stream (WebSocket upgrade)
pub async fn stream(
    Path(id): Path<String>,
    Query(query): Query<StreamQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_stream(socket, id, query))
}

async fn handle_stream(
    socket: axum::extract::ws::WebSocket,
    session_id: String,
    query: StreamQuery,
) {
    let (mut sender, mut _receiver) = socket.split();

    // Resolve log path
    let log_path =
        match zag_orch::listen::resolve_session_log(Some(&session_id), false, false, None) {
            Ok(p) => p,
            Err(e) => {
                let _ = sender
                    .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                        code: 1008,
                        reason: e.to_string().into(),
                    })))
                    .await;
                return;
            }
        };

    // Parse filter query param into a vec
    let filters = query
        .filter
        .map(|f| f.split(',').map(|s| s.trim().to_string()).collect());

    // Start streaming events
    let mut rx = match zag_orch::listen::stream_session_events(&log_path, filters) {
        Ok(rx) => rx,
        Err(e) => {
            let _ = sender
                .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: 1011,
                    reason: e.to_string().into(),
                })))
                .await;
            return;
        }
    };

    // Forward events as JSON text frames
    while let Some(event) = rx.recv().await {
        let json = match serde_json::to_string(&event) {
            Ok(j) => j,
            Err(_) => continue,
        };
        if sender.send(Message::Text(json.into())).await.is_err() {
            break; // client disconnected
        }
    }

    let _ = sender.close().await;
}
