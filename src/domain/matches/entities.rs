use crate::domain::matches::{AnswerResult, JoinSource, MatchStatus, RoundStatus, ScoreEventKind};
use crate::domain::{DomainId, JoinPin, NonEmptyString, Points, PositiveLimit, UtcDateTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    pub id: DomainId,
    pub organization_id: DomainId,
    pub game_id: DomainId,
    pub game_mode_id: DomainId,
    pub host_user_id: DomainId,
    pub name: NonEmptyString,
    pub status: MatchStatus,
    pub current_round_id: Option<DomainId>,
    pub winner_participant_id: Option<DomainId>,
    pub created_at: UtcDateTime,
    pub started_at: Option<UtcDateTime>,
    pub ended_at: Option<UtcDateTime>,
    pub updated_at: UtcDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchSettings {
    pub match_id: DomainId,
    pub max_rounds: Option<PositiveLimit>,
    pub time_per_round_seconds: Option<PositiveLimit>,
    pub initial_points: Points,
    pub remote_enabled: bool,
    pub capacity: Option<PositiveLimit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchQuestion {
    pub id: DomainId,
    pub match_id: DomainId,
    pub source_question_id: Option<DomainId>,
    pub text_snapshot: NonEmptyString,
    pub answer_snapshot: NonEmptyString,
    pub points: Points,
    pub used_at: Option<UtcDateTime>,
    pub created_at: UtcDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchParticipant {
    pub id: DomainId,
    pub match_id: DomainId,
    pub player_id: Option<DomainId>,
    pub display_name_snapshot: NonEmptyString,
    pub join_source: JoinSource,
    pub current_score: Points,
    pub lives_remaining: Option<i32>,
    pub eliminated: bool,
    pub joined_at: UtcDateTime,
    pub left_at: Option<UtcDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchRound {
    pub id: DomainId,
    pub match_id: DomainId,
    pub match_question_id: Option<DomainId>,
    pub sequence: PositiveLimit,
    pub status: RoundStatus,
    pub selected_participant_id: Option<DomainId>,
    pub answer_revealed_at: Option<UtcDateTime>,
    pub opened_at: UtcDateTime,
    pub closed_at: Option<UtcDateTime>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchAnswer {
    pub id: DomainId,
    pub match_id: DomainId,
    pub round_id: DomainId,
    pub participant_id: Option<DomainId>,
    pub result: AnswerResult,
    pub points_applied: Points,
    pub answered_at: UtcDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoreEvent {
    pub id: DomainId,
    pub match_id: DomainId,
    pub participant_id: DomainId,
    pub round_id: Option<DomainId>,
    pub kind: ScoreEventKind,
    pub delta: Points,
    pub recorded_by_user_id: Option<DomainId>,
    pub occurred_at: UtcDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchJoinCode {
    pub match_id: DomainId,
    pub pin: JoinPin,
    pub active: bool,
    pub created_at: UtcDateTime,
    pub expires_at: Option<UtcDateTime>,
}
