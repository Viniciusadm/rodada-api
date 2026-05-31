#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatePlayerCommand {
    pub organization_id: String,
    pub user_id: Option<String>,
    pub ministry_id: Option<String>,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdatePlayerCommand {
    pub player_id: String,
    pub ministry_id: Option<String>,
    pub display_name: Option<String>,
    pub is_active: Option<bool>,
}
