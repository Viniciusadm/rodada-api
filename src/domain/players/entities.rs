use crate::domain::{DomainId, NonEmptyString, UtcDateTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    pub id: DomainId,
    pub organization_id: DomainId,
    pub user_id: Option<DomainId>,
    pub ministry_id: Option<DomainId>,
    pub display_name: NonEmptyString,
    pub is_active: bool,
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
}
