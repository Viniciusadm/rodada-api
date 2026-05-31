#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateNotificationPreferenceCommand {
    pub organization_id: String,
    pub user_id: String,
    pub preference_key: String,
    pub enabled: bool,
}
