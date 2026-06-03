use async_trait::async_trait;

use crate::application::matches::{MatchRealtimeEventKind, RealtimeMatchPublisher};
use crate::domain::DomainId;
use crate::errors::AppError;
use crate::realtime::{
    MatchEventPayload, RealtimeChannel, RealtimeEvent, RealtimeEventKind, RealtimeHub,
};

#[async_trait]
impl RealtimeMatchPublisher for RealtimeHub {
    async fn publish_match_event(
        &self,
        match_id: &str,
        kind: MatchRealtimeEventKind,
    ) -> Result<(), AppError> {
        let channel = RealtimeChannel::Match(DomainId::new(match_id.to_owned())?);
        self.publish(RealtimeEvent::new(
            channel,
            realtime_kind(kind),
            MatchEventPayload::new(match_id).into_value(),
        ));
        Ok(())
    }

    async fn publish_user_event(
        &self,
        user_id: &str,
        kind: MatchRealtimeEventKind,
    ) -> Result<(), AppError> {
        let user_id = DomainId::new(user_id.to_owned())?;
        let payload_user_id = user_id.as_str().to_owned();
        let channel = RealtimeChannel::User(user_id);
        self.publish(RealtimeEvent::new(
            channel,
            realtime_kind(kind),
            serde_json::json!({ "user_id": payload_user_id }),
        ));
        Ok(())
    }
}

fn realtime_kind(kind: MatchRealtimeEventKind) -> RealtimeEventKind {
    match kind {
        MatchRealtimeEventKind::Created => RealtimeEventKind::MatchCreated,
        MatchRealtimeEventKind::Updated => RealtimeEventKind::MatchUpdated,
        MatchRealtimeEventKind::BuzzerUpdated => RealtimeEventKind::BuzzerUpdated,
        MatchRealtimeEventKind::RoomUpdated => RealtimeEventKind::RoomUpdated,
        MatchRealtimeEventKind::RankingUpdated => RealtimeEventKind::RankingUpdated,
    }
}
