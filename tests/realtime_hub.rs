use holyplay_backend::domain::DomainId;
use holyplay_backend::realtime::{
    user_channel, MatchEventPayload, PresenceStatus, RealtimeChannel, RealtimeEvent,
    RealtimeEventKind, RealtimeHub, GLOBAL_CHANNEL,
};
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn two_subscribers_receive_match_updates() {
    let hub = RealtimeHub::new(16);
    let channel = RealtimeChannel::Match(DomainId::new("match-1").unwrap());
    let mut first = hub.subscribe(channel.clone());
    let mut second = hub.subscribe(channel.clone());

    hub.publish(RealtimeEvent::new(
        channel,
        RealtimeEventKind::MatchUpdated,
        MatchEventPayload::new("match-1").into_value(),
    ));

    let first_event = first.recv().await.unwrap();
    let second_event = second.recv().await.unwrap();

    assert_eq!(first_event.kind, RealtimeEventKind::MatchUpdated);
    assert_eq!(second_event.kind, RealtimeEventKind::MatchUpdated);
    assert_eq!(first_event.channel, "match:match-1");
    assert_eq!(second_event.channel, "match:match-1");
}

#[tokio::test]
async fn subscriber_ignores_other_channels() {
    let hub = RealtimeHub::new(16);
    let match_channel = RealtimeChannel::Match(DomainId::new("match-1").unwrap());
    let other_channel = RealtimeChannel::Match(DomainId::new("match-2").unwrap());
    let mut subscription = hub.subscribe(match_channel.clone());

    hub.publish(RealtimeEvent::new(
        other_channel,
        RealtimeEventKind::MatchUpdated,
        MatchEventPayload::new("match-2").into_value(),
    ));
    hub.publish(RealtimeEvent::new(
        match_channel,
        RealtimeEventKind::MatchUpdated,
        MatchEventPayload::new("match-1").into_value(),
    ));

    let event = subscription.recv().await.unwrap();

    assert_eq!(event.channel, "match:match-1");
}

#[tokio::test]
async fn connection_can_join_and_leave_channels() {
    let hub = RealtimeHub::new(16);
    let initial_channel = RealtimeChannel::Match(DomainId::new("match-1").unwrap());
    let room_channel = RealtimeChannel::Room(DomainId::new("room-1").unwrap());
    let (mut subscription, changes) =
        hub.subscribe_connection("conn-1", "user-1", initial_channel.clone());

    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].status, PresenceStatus::Connected);
    assert_eq!(hub.connection_count(), 1);
    assert_eq!(hub.user_connection_count("user-1"), 1);

    let join_change = hub
        .join_connection(&subscription, "conn-1", room_channel.clone())
        .unwrap();
    assert_eq!(join_change.channel, "room:room-1");
    assert_eq!(join_change.status, PresenceStatus::Connected);

    hub.publish(RealtimeEvent::new(
        room_channel.clone(),
        RealtimeEventKind::RoomUpdated,
        serde_json::json!({ "room_id": "room-1" }),
    ));
    let event = subscription.recv().await.unwrap();
    assert_eq!(event.channel, "room:room-1");

    let leave_change = hub
        .leave_connection(&subscription, "conn-1", &room_channel)
        .unwrap();
    assert_eq!(leave_change.status, PresenceStatus::Disconnected);

    hub.publish(RealtimeEvent::new(
        room_channel,
        RealtimeEventKind::RoomUpdated,
        serde_json::json!({ "room_id": "room-1" }),
    ));
    assert!(timeout(Duration::from_millis(20), subscription.recv())
        .await
        .is_err());
}

#[tokio::test]
async fn presence_stays_online_until_last_user_connection_leaves_channel() {
    let hub = RealtimeHub::new(16);
    let channel = RealtimeChannel::Match(DomainId::new("match-1").unwrap());
    let (_first, first_changes) = hub.subscribe_connection("conn-1", "user-1", channel.clone());
    let (_second, second_changes) = hub.subscribe_connection("conn-2", "user-1", channel.clone());

    assert_eq!(first_changes.len(), 1);
    assert!(second_changes.is_empty());
    assert_eq!(hub.channel_user_count("match:match-1", "user-1"), 2);

    let first_disconnect = hub.unregister_connection("conn-1");
    assert!(first_disconnect.is_empty());
    assert_eq!(hub.channel_user_count("match:match-1", "user-1"), 1);

    let second_disconnect = hub.unregister_connection("conn-2");
    assert_eq!(second_disconnect.len(), 1);
    assert_eq!(second_disconnect[0].status, PresenceStatus::Disconnected);
    assert_eq!(hub.user_connection_count("user-1"), 0);
}

#[tokio::test]
async fn can_publish_to_user_and_global_channels() {
    let hub = RealtimeHub::new(16);
    let channel = RealtimeChannel::Match(DomainId::new("match-1").unwrap());
    let (mut user_one, _) = hub.subscribe_connection("conn-1", "user-1", channel.clone());
    let (mut user_two, _) = hub.subscribe_connection("conn-2", "user-2", channel);

    hub.publish_user(
        "user-1",
        RealtimeEventKind::Ack,
        serde_json::json!({ "ok": true }),
    );
    let event = user_one.recv().await.unwrap();
    assert_eq!(event.channel, user_channel("user-1"));
    assert!(timeout(Duration::from_millis(20), user_two.recv())
        .await
        .is_err());

    hub.publish_global(
        RealtimeEventKind::Ack,
        serde_json::json!({ "broadcast": true }),
    );
    assert_eq!(user_one.recv().await.unwrap().channel, GLOBAL_CHANNEL);
    assert_eq!(user_two.recv().await.unwrap().channel, GLOBAL_CHANNEL);
}
