use async_trait::async_trait;

use crate::api::routes::v1::matches::match_state_response;
use crate::application::matches::{MatchRealtimeEventKind, MatchState, RealtimeMatchPublisher};
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

    async fn publish_match_state(
        &self,
        match_id: &str,
        kind: MatchRealtimeEventKind,
        state: MatchState,
    ) -> Result<(), AppError> {
        let channel = RealtimeChannel::Match(DomainId::new(match_id.to_owned())?);
        let state = serde_json::to_value(match_state_response(state))
            .map_err(|error| AppError::Internal(error.to_string()))?;
        self.publish(RealtimeEvent::new(
            channel,
            realtime_kind(kind),
            MatchEventPayload::new(match_id)
                .with_state(state)
                .into_value(),
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
    }
}
