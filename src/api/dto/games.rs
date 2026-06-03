use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct GameResponse {
    pub id: String,
    pub organization_id: Option<String>,
    pub slug: String,
    pub name: String,
    pub description: String,
    /// Gameplay kind: `buzzer` | `true_false` | `free_text`. Drives how the
    /// frontend authors questions, renders the player UI, and how answers score.
    pub kind: String,
    pub icon_key: Option<String>,
    pub player_count: i64,
    pub modes: Vec<GameModeResponse>,
}

#[derive(Debug, Serialize)]
pub struct GameModeResponse {
    pub id: String,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub default_initial_points: Option<i32>,
    pub default_time_per_question_seconds: Option<i32>,
    pub is_active: bool,
}
