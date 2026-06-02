#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateGameCommand {
    pub organization_id: Option<String>,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub icon_key: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateGameCommand {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub icon_key: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateGameModeCommand {
    pub game_id: String,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub default_initial_points: Option<i32>,
    pub default_time_per_question_seconds: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateGameModeCommand {
    pub id: String,
    pub game_id: String,
    pub name: String,
    pub description: Option<String>,
    pub default_initial_points: Option<i32>,
    pub default_time_per_question_seconds: Option<u32>,
    pub is_active: bool,
}
