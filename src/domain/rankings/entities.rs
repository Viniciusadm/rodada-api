use crate::domain::{DomainId, Points};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankingEntry {
    pub player_id: DomainId,
    pub display_name: String,
    pub game_id: Option<DomainId>,
    pub position: u32,
    pub total_points: Points,
    pub matches_played: u32,
    pub wins: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerStats {
    pub player_id: DomainId,
    pub total_points: Points,
    pub matches_played: u32,
    pub wins: u32,
    pub level: u32,
}
