use serde::{Deserialize, Serialize};

use crate::application::players::{CreatePlayerCommand, UpdatePlayerCommand};

#[derive(Debug, Deserialize)]
pub struct CreatePlayerRequest {
    pub organization_id: String,
    pub user_id: Option<String>,
    pub ministry_id: Option<String>,
    pub display_name: String,
}

impl From<CreatePlayerRequest> for CreatePlayerCommand {
    fn from(value: CreatePlayerRequest) -> Self {
        Self {
            organization_id: value.organization_id,
            user_id: value.user_id,
            ministry_id: value.ministry_id,
            display_name: value.display_name,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdatePlayerRequest {
    pub ministry_id: Option<String>,
    pub display_name: Option<String>,
    pub is_active: Option<bool>,
}

impl UpdatePlayerRequest {
    pub fn into_command(self, player_id: String) -> UpdatePlayerCommand {
        UpdatePlayerCommand {
            player_id,
            ministry_id: self.ministry_id,
            display_name: self.display_name,
            is_active: self.is_active,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PlayerResponse {
    pub id: String,
    pub display_name: String,
    pub ministry_id: Option<String>,
    pub is_active: bool,
}
