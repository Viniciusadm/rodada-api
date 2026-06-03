use crate::domain::questions::ContentImportStatus;
use crate::domain::{DomainId, NonEmptyString, Points, UtcDateTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionBank {
    pub id: DomainId,
    pub game_id: DomainId,
    pub name: NonEmptyString,
    pub created_by_user_id: DomainId,
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    pub id: DomainId,
    pub question_bank_id: DomainId,
    pub text: NonEmptyString,
    pub answer: NonEmptyString,
    pub default_points: Points,
    pub is_active: bool,
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentImport {
    pub id: DomainId,
    pub question_bank_id: DomainId,
    pub imported_by_user_id: DomainId,
    pub status: ContentImportStatus,
    pub valid_rows: u32,
    pub invalid_rows: u32,
    pub created_at: UtcDateTime,
    pub completed_at: Option<UtcDateTime>,
}
