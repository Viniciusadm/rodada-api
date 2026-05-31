use crate::domain::rooms::{BuzzerDecision, PresenceRole, RoomStatus};
use crate::domain::{DomainId, JoinPin, PositiveLimit, UtcDateTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomSession {
    pub id: DomainId,
    pub match_id: DomainId,
    pub pin: JoinPin,
    pub status: RoomStatus,
    pub capacity: Option<PositiveLimit>,
    pub opened_at: UtcDateTime,
    pub closed_at: Option<UtcDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresenceSession {
    pub id: DomainId,
    pub room_session_id: DomainId,
    pub user_id: Option<DomainId>,
    pub participant_id: Option<DomainId>,
    pub role: PresenceRole,
    pub connected_at: UtcDateTime,
    pub disconnected_at: Option<UtcDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuzzerEvent {
    pub id: DomainId,
    pub match_id: DomainId,
    pub round_id: DomainId,
    pub participant_id: DomainId,
    pub presence_session_id: Option<DomainId>,
    pub decision: BuzzerDecision,
    pub occurred_at: UtcDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentationSession {
    pub id: DomainId,
    pub room_session_id: DomainId,
    pub presence_session_id: Option<DomainId>,
    pub opened_at: UtcDateTime,
    pub closed_at: Option<UtcDateTime>,
}
