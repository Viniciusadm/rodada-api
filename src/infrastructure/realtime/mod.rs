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

    async fn publish_organization_event(
        &self,
        organization_id: &str,
        kind: MatchRealtimeEventKind,
    ) -> Result<(), AppError> {
        let organization_id = DomainId::new(organization_id.to_owned())?;
        let payload_organization_id = organization_id.as_str().to_owned();
        let channel = match kind {
            MatchRealtimeEventKind::RankingUpdated => RealtimeChannel::Ranking(organization_id),
            _ => RealtimeChannel::Organization(organization_id),
        };
        self.publish(RealtimeEvent::new(
            channel,
            realtime_kind(kind),
            serde_json::json!({ "organization_id": payload_organization_id }),
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
