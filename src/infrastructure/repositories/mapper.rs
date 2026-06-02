use chrono::NaiveDateTime;

use crate::domain::matches::{JoinSource, MatchStatus, ScoreEventKind};
use crate::domain::users::UserRoleKind;
use crate::domain::{DomainId, Email, NonEmptyString, Points, Slug, UtcDateTime};
use crate::errors::AppError;

pub fn id(value: String) -> Result<DomainId, AppError> {
    Ok(DomainId::new(value)?)
}

pub fn non_empty(value: String, field: &'static str) -> Result<NonEmptyString, AppError> {
    Ok(NonEmptyString::new(value, field)?)
}

pub fn slug(value: String) -> Result<Slug, AppError> {
    Ok(Slug::new(value)?)
}

pub fn email(value: String) -> Result<Email, AppError> {
    Ok(Email::new(value)?)
}

pub fn datetime(value: NaiveDateTime) -> Result<UtcDateTime, AppError> {
    Ok(UtcDateTime::new_iso8601(value.to_string())?)
}

pub fn opt_datetime(value: Option<NaiveDateTime>) -> Result<Option<UtcDateTime>, AppError> {
    value.map(datetime).transpose()
}

pub fn opt_id(value: Option<String>) -> Result<Option<DomainId>, AppError> {
    value.map(id).transpose()
}

pub fn match_status(value: String) -> MatchStatus {
    match value.as_str() {
        "in_progress" => MatchStatus::InProgress,
        "finished" => MatchStatus::Finished,
        "cancelled" => MatchStatus::Cancelled,
        _ => MatchStatus::Waiting,
    }
}

pub fn join_source(value: String) -> JoinSource {
    match value.as_str() {
        "remote" => JoinSource::Remote,
        _ => JoinSource::Local,
    }
}

pub fn score_event_kind(value: String) -> ScoreEventKind {
    match value.as_str() {
        "correct_answer" => ScoreEventKind::CorrectAnswer,
        "incorrect_answer" => ScoreEventKind::IncorrectAnswer,
        "manual_adjustment" => ScoreEventKind::ManualAdjustment,
        "penalty" => ScoreEventKind::Penalty,
        "reset" => ScoreEventKind::Reset,
        _ => ScoreEventKind::InitialPoints,
    }
}

pub fn user_role_kind(value: String) -> UserRoleKind {
    match value.as_str() {
        "owner" => UserRoleKind::Owner,
        "admin" => UserRoleKind::Admin,
        "host" => UserRoleKind::Host,
        _ => UserRoleKind::Member,
    }
}

pub fn normalized_name(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub fn points(value: i32) -> Points {
    Points::new(value)
}
