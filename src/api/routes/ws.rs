use std::borrow::Cow;
use std::time::{Duration, Instant};

use axum::extract::ws::{close_code, CloseFrame, Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::time::{interval, MissedTickBehavior};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::application::auth::TokenVerifier;
use crate::errors::AppError;
use crate::infrastructure::auth::JwtTokenService;
use crate::realtime::{
    ack_payload, error_payload, pong_payload, serialize_event, ClientRealtimeMessage,
    ConnectionRateLimit, PresencePayload, PresenceStatus, RealtimeChannel, RealtimeEvent,
    RealtimeEventKind, RealtimeHub,
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
    if let Err(error) = authorize_channel(&state, &user_id, &channel).await {
        return app_error(error);
    }
    let hub = state.realtime.clone();

    ws.on_upgrade(move |socket| websocket_session(socket, state, hub, channel, user_id))
        .into_response()
}

async fn websocket_session(
    socket: WebSocket,
    state: AppState,
    hub: RealtimeHub,
    channel: RealtimeChannel,
    user_id: String,
) {
    let connection_id = Uuid::new_v4().to_string();
    let (mut subscription, presence_changes) =
        hub.subscribe_connection(connection_id.clone(), user_id.clone(), channel.clone());
    publish_presence_changes(&hub, presence_changes);
    info!(%connection_id, %user_id, channel = %channel.key(), "websocket connected");

    let (mut sender, mut receiver) = socket.split();
    let mut heartbeat = interval(Duration::from_secs(25));
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut awaiting_pong = false;
    let mut rate_limit = ConnectionRateLimit::new(60, Duration::from_secs(60));

    loop {
        tokio::select! {
            event = subscription.recv() => {
                let Ok(event) = event else {
                    warn!(%connection_id, "websocket subscription closed");
                    break;
                };
                let Some(message) = serialize_event(&event) else {
                    continue;
                };
                if sender.send(Message::Text(message)).await.is_err() {
                    debug!(%connection_id, "websocket send failed");
                    break;
                }
            }
            _ = heartbeat.tick() => {
                if awaiting_pong {
                    let _ = sender
                        .send(Message::Close(Some(CloseFrame {
                            code: close_code::AWAY,
                            reason: Cow::from("heartbeat timeout"),
                        })))
                        .await;
                    warn!(%connection_id, "websocket heartbeat timeout");
                    break;
                }
                awaiting_pong = true;
                if sender.send(Message::Ping(Vec::new())).await.is_err() {
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
                    Some(Ok(Message::Pong(_))) => {
                        awaiting_pong = false;
                    }
                    Some(Ok(Message::Text(text))) => {
                        if !rate_limit.allow(Instant::now()) {
                            send_system_error(&mut sender, "rate_limited", "too many websocket messages", &connection_id).await;
                            let _ = sender
                                .send(Message::Close(Some(CloseFrame {
                                    code: close_code::POLICY,
                                    reason: Cow::from("rate limited"),
                                })))
                                .await;
                            break;
                        }
                        if !handle_client_message(&state, &hub, &subscription, &mut sender, &connection_id, &user_id, text.as_str()).await {
                            break;
                        }
                    }
                    Some(Ok(Message::Binary(_))) => {
                        send_system_error(&mut sender, "unsupported_message", "binary websocket messages are not supported", &connection_id).await;
                    }
                    Some(Err(error)) => {
                        debug!(%connection_id, error = %error, "websocket receive failed");
                        break;
                    }
                }
            }
        }
    }

    let presence_changes = hub.unregister_connection(&connection_id);
    publish_presence_changes(&hub, presence_changes);
    info!(%connection_id, %user_id, "websocket disconnected");
}

#[derive(Debug, Deserialize)]
struct WsQuery {
    token: String,
    channel: String,
}

async fn handle_client_message(
    state: &AppState,
    hub: &RealtimeHub,
    subscription: &crate::realtime::RealtimeSubscription,
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    connection_id: &str,
    user_id: &str,
    text: &str,
) -> bool {
    let message = match serde_json::from_str::<ClientRealtimeMessage>(text) {
        Ok(message) => message,
        Err(_) => {
            send_system_error(
                sender,
                "invalid_json",
                "invalid realtime message",
                connection_id,
            )
            .await;
            return true;
        }
    };

    match message {
        ClientRealtimeMessage::Join { channel } => {
            let channel = match RealtimeChannel::parse(&channel) {
                Ok(channel) => channel,
                Err(_) => {
                    send_system_error(
                        sender,
                        "invalid_channel",
                        "invalid realtime channel",
                        connection_id,
                    )
                    .await;
                    return true;
                }
            };
            if let Err(error) = authorize_channel(state, user_id, &channel).await {
                warn!(%connection_id, %user_id, error = %error, "websocket join denied");
                send_system_error(
                    sender,
                    "forbidden_channel",
                    "user cannot join realtime channel",
                    connection_id,
                )
                .await;
                return true;
            }
            let change = hub.join_connection(subscription, connection_id, channel.clone());
            if let Some(change) = change {
                publish_presence_changes(hub, vec![change]);
            }
            send_system_event(
                sender,
                RealtimeEventKind::Ack,
                ack_payload("join", Some(&channel.key()), connection_id),
            )
            .await
        }
        ClientRealtimeMessage::Leave { channel } => {
            let channel = match RealtimeChannel::parse(&channel) {
                Ok(channel) => channel,
                Err(_) => {
                    send_system_error(
                        sender,
                        "invalid_channel",
                        "invalid realtime channel",
                        connection_id,
                    )
                    .await;
                    return true;
                }
            };
            let change = hub.leave_connection(subscription, connection_id, &channel);
            if let Some(change) = change {
                publish_presence_changes(hub, vec![change]);
            }
            send_system_event(
                sender,
                RealtimeEventKind::Ack,
                ack_payload("leave", Some(&channel.key()), connection_id),
            )
            .await
        }
        ClientRealtimeMessage::Ping => {
            send_system_event(sender, RealtimeEventKind::Pong, pong_payload(connection_id)).await
        }
    }
}

async fn send_system_error(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    code: &str,
    message: &str,
    connection_id: &str,
) {
    send_system_event(
        sender,
        RealtimeEventKind::Error,
        error_payload(code, message, connection_id),
    )
    .await;
}

async fn send_system_event(
    sender: &mut futures_util::stream::SplitSink<WebSocket, Message>,
    kind: RealtimeEventKind,
    payload: serde_json::Value,
) -> bool {
    let event = RealtimeEvent::new_key("system".to_owned(), kind, payload);
    let Some(message) = serialize_event(&event) else {
        return true;
    };
    sender.send(Message::Text(message)).await.is_ok()
}

fn publish_presence_changes(hub: &RealtimeHub, changes: Vec<crate::realtime::PresenceChange>) {
    for change in changes {
        let payload = match change.status {
            PresenceStatus::Connected => PresencePayload::connected(change.user_id),
            PresenceStatus::Disconnected => PresencePayload::disconnected(change.user_id),
        };
        hub.publish(RealtimeEvent::new_key(
            change.channel,
            RealtimeEventKind::PresenceUpdated,
            payload.into_value(),
        ));
    }
}

async fn authorize_channel(
    _state: &AppState,
    user_id: &str,
    channel: &RealtimeChannel,
) -> Result<(), AppError> {
    // Single-tenant model: a user may only listen to their own user channel.
    // Match/room channels are open to any authenticated connection (the token was
    // already verified upstream).
    match channel {
        RealtimeChannel::User(id) if id.as_str() != user_id => Err(AppError::Forbidden(
            "cannot subscribe to another user's channel".to_owned(),
        )),
        _ => Ok(()),
    }
}
