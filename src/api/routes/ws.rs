use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;

use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::application::auth::TokenVerifier;
use crate::infrastructure::auth::JwtTokenService;
use crate::realtime::{
    PresencePayload, RealtimeChannel, RealtimeEvent, RealtimeEventKind, RealtimeHub,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/ws", get(connect))
}

async fn connect(
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    let user_id =
        match JwtTokenService::new(state.jwt_secret.clone()).verify_access_token(&query.token) {
            Ok(user_id) => user_id,
            Err(error) => return app_error(error),
        };
    let channel = match RealtimeChannel::parse(&query.channel) {
        Ok(channel) => channel,
        Err(error) => return app_error(error),
    };
    let hub = state.realtime.clone();

    ws.on_upgrade(move |socket| websocket_session(socket, hub, channel, user_id))
        .into_response()
}

async fn websocket_session(
    socket: WebSocket,
    hub: RealtimeHub,
    channel: RealtimeChannel,
    user_id: String,
) {
    let mut subscription = hub.subscribe(channel.clone());
    hub.publish(RealtimeEvent::new(
        channel.clone(),
        RealtimeEventKind::PresenceUpdated,
        PresencePayload::connected(user_id.clone()).into_value(),
    ));

    let (mut sender, mut receiver) = socket.split();
    loop {
        tokio::select! {
            event = subscription.recv() => {
                let Ok(event) = event else {
                    break;
                };
                let Ok(message) = serde_json::to_string(&event) else {
                    continue;
                };
                if sender.send(Message::Text(message)).await.is_err() {
                    break;
                }
            }
            inbound = receiver.next() => {
                match inbound {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(payload))) => {
                        if sender.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
        }
    }

    hub.publish(RealtimeEvent::new(
        channel,
        RealtimeEventKind::PresenceUpdated,
        PresencePayload::disconnected(user_id).into_value(),
    ));
}

#[derive(Debug, Deserialize)]
struct WsQuery {
    token: String,
    channel: String,
}
