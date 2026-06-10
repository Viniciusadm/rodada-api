use async_trait::async_trait;

use crate::application::match_templates::{
    CreateMatchTemplateCommand, DeleteMatchTemplateCommand, UpdateMatchTemplateCommand,
};
use crate::domain::games::McOption;
use crate::errors::AppError;

#[async_trait]
pub trait MatchTemplateRepository: Send + Sync {
    async fn create(&self, id: &str, command: &CreateMatchTemplateCommand) -> Result<(), AppError>;
    async fn update(&self, command: &UpdateMatchTemplateCommand) -> Result<bool, AppError>;
    async fn delete(&self, command: &DeleteMatchTemplateCommand) -> Result<bool, AppError>;
    async fn list_summaries_by_owner(
        &self,
        owner_user_id: &str,
        search: Option<&str>,
    ) -> Result<Vec<MatchTemplateSummaryState>, AppError>;
    async fn get_with_questions(
        &self,
        owner_user_id: &str,
        id: &str,
    ) -> Result<Option<MatchTemplateState>, AppError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchTemplateSummaryState {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub mode: String,
    pub scoring_mode: String,
    pub mode_label: String,
    pub scoring_mode_label: String,
    pub question_count: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchTemplateState {
    pub id: String,
    pub host_user_id: String,
    pub name: String,
    pub description: Option<String>,
    pub mode: String,
    pub scoring_mode: String,
    pub mode_label: String,
    pub scoring_mode_label: String,
    pub max_rounds: Option<u32>,
    pub time_per_round_seconds: Option<u32>,
    pub initial_points: i32,
    pub remote_enabled: bool,
    pub capacity: Option<u32>,
    pub buzzer_arm_delay_seconds: u32,
    pub ranking_display: String,
    pub questions: Vec<MatchTemplateQuestionState>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchTemplateQuestionState {
    pub id: String,
    pub source_question_id: Option<String>,
    pub question_type: String,
    pub prompt: String,
    pub expected_answer: String,
    pub options: Vec<McOption>,
    pub points: i32,
    pub position: u32,
}
