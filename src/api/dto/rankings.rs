use serde::{Deserialize, Serialize};

use crate::application::rankings::RankingQuery;

#[derive(Debug, Deserialize)]
pub struct RankingQueryParams {
    pub organization_id: String,
    pub game_id: Option<String>,
    pub ministry_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

impl From<RankingQueryParams> for RankingQuery {
    fn from(value: RankingQueryParams) -> Self {
        Self {
            organization_id: value.organization_id,
            game_id: value.game_id,
            ministry_id: value.ministry_id,
            from: value.from,
            to: value.to,
            page: value.page.unwrap_or(1),
            per_page: value.per_page.unwrap_or(50),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RankingEntryResponse {
    pub position: u32,
    pub player_id: String,
    pub display_name: String,
    pub ministry_id: Option<String>,
    pub total_points: i32,
    pub matches_played: u32,
    pub wins: u32,
}
