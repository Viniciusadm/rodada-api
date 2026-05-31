use crate::domain::notifications::NotificationPreferenceKey;
use crate::domain::{DomainId, UtcDateTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationPreference {
    pub id: DomainId,
    pub organization_id: DomainId,
    pub user_id: DomainId,
    pub key: NotificationPreferenceKey,
    pub enabled: bool,
    pub updated_at: UtcDateTime,
}
