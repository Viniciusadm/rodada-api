use crate::application::matches::MatchQuestionDraftCommand;
use crate::domain::games::{MatchMode, RankingDisplay, ScoringMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateMatchTemplateCommand {
    pub host_user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub mode: MatchMode,
    pub scoring_mode: ScoringMode,
    pub max_rounds: Option<u32>,
    pub time_per_round_seconds: Option<u32>,
    pub initial_points: i32,
    pub remote_enabled: bool,
    pub capacity: Option<u32>,
    pub buzzer_arm_delay_seconds: u32,
    pub ranking_display: RankingDisplay,
    pub questions: Vec<MatchQuestionDraftCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateMatchTemplateCommand {
    pub id: String,
    pub host_user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub mode: MatchMode,
    pub scoring_mode: ScoringMode,
    pub max_rounds: Option<u32>,
    pub time_per_round_seconds: Option<u32>,
    pub initial_points: i32,
    pub remote_enabled: bool,
    pub capacity: Option<u32>,
    pub buzzer_arm_delay_seconds: u32,
    pub ranking_display: RankingDisplay,
    pub questions: Vec<MatchQuestionDraftCommand>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteMatchTemplateCommand {
    pub id: String,
    pub host_user_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateMatchTemplateCommand {
    pub id: String,
    pub host_user_id: String,
}
