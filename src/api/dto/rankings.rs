use serde::{Deserialize, Serialize};

use crate::application::rankings::RankingQuery;

#[derive(Debug, Deserialize)]
pub struct RankingQueryParams {
    pub game_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

impl RankingQueryParams {
    pub fn into_query(self, owner_user_id: String) -> RankingQuery {
        RankingQuery {
            owner_user_id,
            game_id: self.game_id,
            from: self.from,
            to: self.to,
            page: self.page.unwrap_or(1),
            per_page: self.per_page.unwrap_or(50),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RankingEntryResponse {
    pub position: u32,
    pub player_id: String,
    pub display_name: String,
    pub total_points: i32,
    pub matches_played: u32,
    pub wins: u32,
}
