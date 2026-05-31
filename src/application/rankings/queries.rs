#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankingQuery {
    pub organization_id: String,
    pub game_id: Option<String>,
    pub ministry_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub page: u32,
    pub per_page: u32,
}
