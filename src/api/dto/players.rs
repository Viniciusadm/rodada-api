use serde::{Deserialize, Serialize};

use crate::application::players::{CreatePlayerCommand, UpdatePlayerCommand};

#[derive(Debug, Deserialize)]
pub struct CreatePlayerRequest {
    pub user_id: Option<String>,
    pub display_name: String,
}

impl CreatePlayerRequest {
    pub fn into_command(self, owner_user_id: String) -> CreatePlayerCommand {
        CreatePlayerCommand {
            owner_user_id,
            user_id: self.user_id,
            display_name: self.display_name,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdatePlayerRequest {
    pub display_name: Option<String>,
    pub is_active: Option<bool>,
}

impl UpdatePlayerRequest {
    pub fn into_command(self, player_id: String) -> UpdatePlayerCommand {
        UpdatePlayerCommand {
            player_id,
            display_name: self.display_name,
            is_active: self.is_active,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PlayerResponse {
    pub id: String,
    pub display_name: String,
    pub is_active: bool,
    pub total_points: i64,
    pub matches_played: i64,
    pub wins: i64,
}
