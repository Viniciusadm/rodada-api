use crate::domain::{DomainId, Email, NonEmptyString, Slug, UtcDateTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Organization {
    pub id: DomainId,
    pub name: NonEmptyString,
    pub slug: Slug,
    pub contact_email: Option<Email>,
    pub owner_user_id: Option<DomainId>,
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ministry {
    pub id: DomainId,
    pub organization_id: DomainId,
    pub name: NonEmptyString,
    pub slug: Slug,
    pub is_active: bool,
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
}
