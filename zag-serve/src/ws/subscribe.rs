//! WebSocket handler for multiplexed event streaming across sessions.

use axum::{
    extract::{Query, WebSocketUpgrade, ws::Message},
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};

use crate::types::SubscribeQuery;

/// GET /api/v1/subscribe (WebSocket upgrade)
pub async fn subscribe(
    Query(query): Query<SubscribeQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_subscribe(socket, query))
}

async fn handle_subscribe(socket: axum::extract::ws::WebSocket, query: SubscribeQuery) {
    let (mut sender, mut _receiver) = socket.split();

    let params = zag_orch::subscribe::SubscribeParams {
        tag: query.tag,
        event_type: query.event_type,
        global: false,
        json: true,
        root: None,
    };

    let mut rx = match zag_orch::subscribe::subscribe_events(&params) {
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

    while let Some(event) = rx.recv().await {
        let json = match serde_json::to_string(&event) {
            Ok(j) => j,
            Err(_) => continue,
        };
        if sender.send(Message::Text(json.into())).await.is_err() {
            break;
        }
    }

    let _ = sender.close().await;
}
