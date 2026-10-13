use serde::{Deserialize, Serialize};

use crate::api::dto::matches::{MatchQuestionDraftRequest, McOptionResponse};
use crate::application::match_templates::{
    CreateMatchTemplateCommand, MatchTemplateQuestionState, MatchTemplateState,
    MatchTemplateSummaryState, UpdateMatchTemplateCommand,
};
use crate::domain::games::{MatchMode, RankingDisplay, ScoringMode};
use crate::errors::AppError;

#[derive(Debug, Deserialize)]
pub struct MatchTemplatePayload {
    pub mode: String,
    pub scoring_mode: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub max_rounds: Option<u32>,
    pub time_per_round_seconds: Option<u32>,
    pub initial_points: Option<i32>,
    pub remote_enabled: Option<bool>,
    pub capacity: Option<u32>,
    pub buzzer_arm_delay_seconds: Option<u32>,
    pub ranking_display: Option<String>,
    pub present_screen: Option<String>,
    pub questions: Vec<MatchQuestionDraftRequest>,
}

struct ParsedPayload {
    mode: MatchMode,
    scoring_mode: ScoringMode,
    ranking_display: RankingDisplay,
    questions: Vec<crate::application::matches::MatchQuestionDraftCommand>,
}

impl MatchTemplatePayload {
    fn parse(self) -> Result<(Self, ParsedPayload), AppError> {
        let mode = MatchMode::from_code(&self.mode)
            .ok_or_else(|| AppError::Validation(format!("unknown match mode: {}", self.mode)))?;
        let scoring_mode = match mode {
            MatchMode::Quiz => ScoringMode::Classic,
            MatchMode::Buzzer => {
                ScoringMode::from_code(self.scoring_mode.as_deref().unwrap_or("classic"))
                    .ok_or_else(|| AppError::Validation("unknown scoring mode".to_owned()))?
            }
        };
        let ranking_display =
            RankingDisplay::from_code(self.ranking_display.as_deref().unwrap_or("always"))
                .ok_or_else(|| AppError::Validation("unknown ranking display".to_owned()))?;
        let questions = self
            .questions
            .iter()
            .cloned()
            .map(|question| question.into_command(mode))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((
            self,
            ParsedPayload {
                mode,
                scoring_mode,
                ranking_display,
                questions,
            },
        ))
    }

    pub fn into_create_command(
        self,
        host_user_id: String,
    ) -> Result<CreateMatchTemplateCommand, AppError> {
        let (raw, parsed) = self.parse()?;
        Ok(CreateMatchTemplateCommand {
            host_user_id,
            name: raw.name,
            description: normalize_description(raw.description),
            mode: parsed.mode,
            scoring_mode: parsed.scoring_mode,
            max_rounds: raw.max_rounds,
            time_per_round_seconds: raw.time_per_round_seconds,
            initial_points: raw.initial_points.unwrap_or_default(),
            remote_enabled: raw.remote_enabled.unwrap_or(true),
            capacity: raw.capacity,
            buzzer_arm_delay_seconds: raw.buzzer_arm_delay_seconds.unwrap_or(0).min(5),
            ranking_display: parsed.ranking_display,
            present_screen: normalize_screen(raw.present_screen),
            questions: parsed.questions,
        })
    }

    pub fn into_update_command(
        self,
        id: String,
        host_user_id: String,
    ) -> Result<UpdateMatchTemplateCommand, AppError> {
        let (raw, parsed) = self.parse()?;
        Ok(UpdateMatchTemplateCommand {
            id,
            host_user_id,
            name: raw.name,
            description: normalize_description(raw.description),
            mode: parsed.mode,
            scoring_mode: parsed.scoring_mode,
            max_rounds: raw.max_rounds,
            time_per_round_seconds: raw.time_per_round_seconds,
            initial_points: raw.initial_points.unwrap_or_default(),
            remote_enabled: raw.remote_enabled.unwrap_or(true),
            capacity: raw.capacity,
            buzzer_arm_delay_seconds: raw.buzzer_arm_delay_seconds.unwrap_or(0).min(5),
            ranking_display: parsed.ranking_display,
            present_screen: normalize_screen(raw.present_screen),
            questions: parsed.questions,
        })
    }
}

fn normalize_screen(value: Option<String>) -> Option<String> {
    value.filter(|value| !value.trim().is_empty())
}

fn normalize_description(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[derive(Debug, Serialize)]
pub struct MatchTemplateSummaryResponse {
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

impl From<MatchTemplateSummaryState> for MatchTemplateSummaryResponse {
    fn from(value: MatchTemplateSummaryState) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            mode: value.mode,
            scoring_mode: value.scoring_mode,
            mode_label: value.mode_label,
            scoring_mode_label: value.scoring_mode_label,
            question_count: value.question_count,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MatchTemplateQuestionResponse {
    pub id: String,
    pub source_question_id: Option<String>,
    pub question_type: String,
    pub prompt: String,
    pub expected_answer: String,
    pub options: Vec<McOptionResponse>,
    pub points: i32,
    pub position: u32,
}

impl From<MatchTemplateQuestionState> for MatchTemplateQuestionResponse {
    fn from(value: MatchTemplateQuestionState) -> Self {
        Self {
            id: value.id,
            source_question_id: value.source_question_id,
            question_type: value.question_type,
            prompt: value.prompt,
            expected_answer: value.expected_answer,
            options: value
                .options
                .into_iter()
                .map(|option| McOptionResponse {
                    text: option.text,
                    correct: option.correct,
                })
                .collect(),
            points: value.points,
            position: value.position,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MatchTemplateResponse {
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
    pub present_screen: Option<String>,
    pub questions: Vec<MatchTemplateQuestionResponse>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<MatchTemplateState> for MatchTemplateResponse {
    fn from(value: MatchTemplateState) -> Self {
        Self {
            id: value.id,
            host_user_id: value.host_user_id,
            name: value.name,
            description: value.description,
            mode: value.mode,
            scoring_mode: value.scoring_mode,
            mode_label: value.mode_label,
            scoring_mode_label: value.scoring_mode_label,
            max_rounds: value.max_rounds,
            time_per_round_seconds: value.time_per_round_seconds,
            initial_points: value.initial_points,
            remote_enabled: value.remote_enabled,
            capacity: value.capacity,
            buzzer_arm_delay_seconds: value.buzzer_arm_delay_seconds,
            ranking_display: value.ranking_display,
            present_screen: value.present_screen,
            questions: value.questions.into_iter().map(Into::into).collect(),
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
