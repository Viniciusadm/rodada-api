use crate::domain::{DomainId, Email, NonEmptyString, UtcDateTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: DomainId,
    pub email: Email,
    pub password_hash: NonEmptyString,
    pub display_name: NonEmptyString,
    pub is_active: bool,
    pub last_login_at: Option<UtcDateTime>,
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
}
