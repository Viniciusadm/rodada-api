use serde::{Deserialize, Serialize};

use crate::application::games::{CreateGameCommand, CreateGameModeCommand};

#[derive(Debug, Deserialize)]
pub struct CreateGameRequest {
    pub organization_id: Option<String>,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub icon_key: Option<String>,
    pub sort_order: Option<i32>,
}

impl From<CreateGameRequest> for CreateGameCommand {
    fn from(value: CreateGameRequest) -> Self {
        Self {
            organization_id: value.organization_id,
            slug: value.slug,
            name: value.name,
            description: value.description,
            status: value.status,
            icon_key: value.icon_key,
            sort_order: value.sort_order.unwrap_or_default(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateGameModeRequest {
    pub game_id: String,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub default_initial_points: Option<i32>,
    pub default_time_per_question_seconds: Option<u32>,
}

impl From<CreateGameModeRequest> for CreateGameModeCommand {
    fn from(value: CreateGameModeRequest) -> Self {
        Self {
            game_id: value.game_id,
            code: value.code,
            name: value.name,
            description: value.description,
            default_initial_points: value.default_initial_points,
            default_time_per_question_seconds: value.default_time_per_question_seconds,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct GameResponse {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub status: String,
    pub icon_key: Option<String>,
    pub modes: Vec<GameModeResponse>,
}

#[derive(Debug, Serialize)]
pub struct GameModeResponse {
    pub id: String,
    pub code: String,
    pub name: String,
}
