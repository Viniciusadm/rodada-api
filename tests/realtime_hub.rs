use holyplay_backend::domain::DomainId;
use holyplay_backend::realtime::{
    MatchEventPayload, RealtimeChannel, RealtimeEvent, RealtimeEventKind, RealtimeHub,
};

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
