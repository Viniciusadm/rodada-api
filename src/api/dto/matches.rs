use serde::{Deserialize, Serialize};

use crate::application::matches::{
    AddMatchQuestionCommand, AddParticipantCommand, AdjustScoreCommand, CreateMatchCommand,
    DeleteMatchQuestionCommand, ImportMatchQuestionsCommand, JoinMatchByIdCommand,
    MatchQuestionDraftCommand, PlayerAnswerCommand, RegisterAnswerCommand,
    SelectParticipantCommand, UpdateMatchQuestionCommand,
};
use crate::domain::games::{MatchMode, McOption, QuestionType};
use crate::errors::AppError;

#[derive(Debug, Clone, Deserialize)]
pub struct McOptionRequest {
    pub text: String,
    pub correct: bool,
}

impl From<McOptionRequest> for McOption {
    fn from(value: McOptionRequest) -> Self {
        Self {
            text: value.text,
            correct: value.correct,
        }
    }
}

fn parse_question_type(code: &str) -> Result<QuestionType, AppError> {
    QuestionType::from_code(code)
        .ok_or_else(|| AppError::Validation(format!("unknown question type: {code}")))
}

fn map_options(options: Option<Vec<McOptionRequest>>) -> Vec<McOption> {
    options
        .unwrap_or_default()
        .into_iter()
        .map(Into::into)
        .collect()
}

#[derive(Debug, Clone, Deserialize)]
pub struct MatchQuestionDraftRequest {
    pub source_question_id: Option<String>,
    pub question_type: Option<String>,
    pub prompt: String,
    pub expected_answer: Option<String>,
    pub options: Option<Vec<McOptionRequest>>,
    pub points: Option<i32>,
}

impl MatchQuestionDraftRequest {
    pub fn into_command(self, mode: MatchMode) -> Result<MatchQuestionDraftCommand, AppError> {
        let question_type = match mode {
            MatchMode::Buzzer => QuestionType::Buzzer,
            MatchMode::Quiz => parse_question_type(
                self.question_type.as_deref().unwrap_or("free_text"),
            )?,
        };
        Ok(MatchQuestionDraftCommand {
            source_question_id: self.source_question_id,
            question_type,
            prompt: self.prompt,
            expected_answer: self.expected_answer.unwrap_or_default(),
            options: map_options(self.options),
            points: self.points.unwrap_or(100),
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateMatchRequest {
    pub template_id: String,
    pub name: Option<String>,
}

impl TryFrom<CreateMatchRequest> for CreateMatchCommand {
    type Error = AppError;

    fn try_from(value: CreateMatchRequest) -> Result<Self, Self::Error> {
        let name = value
            .name
            .map(|name| name.trim().to_owned())
            .filter(|name| !name.is_empty());
        Ok(Self {
            template_id: value.template_id,
            host_user_id: String::new(),
            name,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct AddParticipantRequest {
    pub display_name: String,
    pub join_source: String,
}

#[derive(Debug, Deserialize)]
pub struct MatchQuestionRequest {
    pub source_question_id: Option<String>,
    pub question_type: Option<String>,
    pub prompt: String,
    pub expected_answer: Option<String>,
    pub options: Option<Vec<McOptionRequest>>,
    pub points: Option<i32>,
}

impl MatchQuestionRequest {
    pub fn into_add_command(
        self,
        match_id: String,
        host_user_id: String,
    ) -> Result<AddMatchQuestionCommand, AppError> {
        let question_type = self
            .question_type
            .as_deref()
            .map(parse_question_type)
            .transpose()?;
        Ok(AddMatchQuestionCommand {
            match_id,
            host_user_id,
            source_question_id: self.source_question_id,
            question_type,
            prompt: self.prompt,
            expected_answer: self.expected_answer.unwrap_or_default(),
            options: map_options(self.options),
            points: self.points.unwrap_or(100),
        })
    }

    pub fn into_update_command(
        self,
        match_id: String,
        question_id: String,
        host_user_id: String,
    ) -> Result<UpdateMatchQuestionCommand, AppError> {
        let question_type = self
            .question_type
            .as_deref()
            .map(parse_question_type)
            .transpose()?;
        Ok(UpdateMatchQuestionCommand {
            match_id,
            question_id,
            host_user_id,
            question_type,
            prompt: self.prompt,
            expected_answer: self.expected_answer.unwrap_or_default(),
            options: map_options(self.options),
            points: self.points.unwrap_or(100),
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct ImportMatchQuestionsRequest {
    pub csv: String,
}

impl ImportMatchQuestionsRequest {
    pub fn into_command(
        self,
        match_id: String,
        host_user_id: String,
    ) -> ImportMatchQuestionsCommand {
        ImportMatchQuestionsCommand {
            match_id,
            host_user_id,
            csv: self.csv,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct JoinMatchRequest {
    pub participant_name: String,
}

impl JoinMatchRequest {
    pub fn into_command(self, match_id: String) -> JoinMatchByIdCommand {
        JoinMatchByIdCommand {
            match_id,
            display_name: self.participant_name,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PlayerAnswerRequest {
    pub participant_token: String,
    pub answer: Option<String>,
}

impl PlayerAnswerRequest {
    pub fn into_command(self, match_id: String, round_id: String) -> PlayerAnswerCommand {
        PlayerAnswerCommand {
            match_id,
            round_id,
            participant_token: self.participant_token,
            answer: self.answer,
        }
    }
}

impl AddParticipantRequest {
    pub fn into_command(self, match_id: String) -> AddParticipantCommand {
        AddParticipantCommand {
            match_id,
            display_name: self.display_name,
            join_source: self.join_source,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SelectParticipantRequest {
    pub participant_id: String,
    pub host_user_id: String,
}

impl SelectParticipantRequest {
    pub fn into_command(self, match_id: String, round_id: String) -> SelectParticipantCommand {
        SelectParticipantCommand {
            match_id,
            round_id,
            participant_id: self.participant_id,
            host_user_id: self.host_user_id,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RegisterAnswerRequest {
    pub participant_id: Option<String>,
    pub result: String,
    pub actor_user_id: String,
}

impl RegisterAnswerRequest {
    pub fn into_command(self, match_id: String, round_id: String) -> RegisterAnswerCommand {
        RegisterAnswerCommand {
            match_id,
            round_id,
            participant_id: self.participant_id,
            result: self.result,
            actor_user_id: self.actor_user_id,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AdjustScoreRequest {
    pub actor_user_id: String,
    pub delta: i32,
    pub reason: Option<String>,
}

impl AdjustScoreRequest {
    pub fn into_command(self, match_id: String, participant_id: String) -> AdjustScoreCommand {
        AdjustScoreCommand {
            match_id,
            participant_id,
            actor_user_id: self.actor_user_id,
            delta: self.delta,
            reason: self.reason,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MatchResponse {
    pub id: String,
    pub mode: String,
    pub host_user_id: String,
    pub name: String,
    pub status: String,
    pub current_round_id: Option<String>,
    pub winner_participant_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MatchTopScoreResponse {
    pub participant_id: String,
    pub display_name: String,
    pub score: i32,
}

#[derive(Debug, Serialize)]
pub struct MatchSummaryResponse {
    pub id: String,
    pub mode: String,
    pub host_user_id: String,
    pub name: String,
    pub status: String,
    pub current_round_id: Option<String>,
    pub winner_participant_id: Option<String>,
    pub mode_label: String,
    pub scoring_mode_label: String,
    pub question_count: u32,
    pub participant_count: u32,
    pub rounds_played: u32,
    pub current_round_number: Option<u32>,
    pub current_round_status: Option<String>,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub duration_seconds: Option<u32>,
    pub winner_name: Option<String>,
    pub winner_score: Option<i32>,
    pub top_scores: Vec<MatchTopScoreResponse>,
}

#[derive(Debug, Serialize)]
pub struct MatchRoundDetailResponse {
    pub round_number: u32,
    pub status: String,
    pub prompt: Option<String>,
    pub expected_answer: Option<String>,
    pub answers: Vec<MatchRoundAnswerResponse>,
}

#[derive(Debug, Serialize)]
pub struct MatchRoundAnswerResponse {
    pub participant_id: Option<String>,
    pub display_name: Option<String>,
    pub result: String,
    pub points_applied: i32,
}

#[derive(Debug, Serialize)]
pub struct MatchParticipantResponse {
    pub id: String,
    pub match_id: String,
    pub display_name: String,
    pub join_source: String,
    pub current_score: i32,
    pub lives_remaining: Option<i32>,
    pub eliminated: bool,
}

#[derive(Debug, Serialize)]
pub struct JoinMatchResponse {
    pub participant: MatchParticipantResponse,
    pub participant_token: String,
}

#[derive(Debug, Serialize)]
pub struct MatchSettingsResponse {
    pub mode: String,
    pub scoring_mode: String,
    pub max_rounds: Option<u32>,
    pub time_per_round_seconds: Option<u32>,
    pub initial_points: i32,
    pub remote_enabled: bool,
    pub capacity: Option<u32>,
    pub buzzer_arm_delay_seconds: u32,
    pub ranking_display: String,
}

#[derive(Debug, Serialize)]
pub struct McOptionResponse {
    pub text: String,
    pub correct: bool,
}

#[derive(Debug, Serialize)]
pub struct MatchQuestionResponse {
    pub id: String,
    pub source_question_id: Option<String>,
    pub question_type: String,
    pub prompt: String,
    pub expected_answer: String,
    pub options: Vec<McOptionResponse>,
    pub points: i32,
    pub used: bool,
    pub position: u32,
}

#[derive(Debug, Serialize)]
pub struct MatchRoundResponse {
    pub id: String,
    pub match_question_id: Option<String>,
    pub round_number: u32,
    pub status: String,
    pub selected_participant_id: Option<String>,
    pub answer_revealed: bool,
    pub time_remaining_seconds: Option<u32>,
    pub question: Option<MatchQuestionResponse>,
    pub early_participant_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct LiveRoomResponse {
    pub id: String,
    pub match_id: String,
    pub match_name: String,
    pub mode: String,
    pub mode_label: String,
    pub pin: String,
    pub status: String,
    pub participants: u32,
    pub capacity: Option<u32>,
    pub host_name: String,
    pub started_minutes_ago: u32,
}

#[derive(Debug, Serialize)]
pub struct MatchStateResponse {
    pub match_data: MatchResponse,
    pub settings: MatchSettingsResponse,
    pub questions: Vec<MatchQuestionResponse>,
    pub participants: Vec<MatchParticipantResponse>,
    pub current_round: Option<MatchRoundResponse>,
    pub room: Option<LiveRoomResponse>,
    pub winner: Option<MatchParticipantResponse>,
    pub last_round_result: Option<LastRoundResultResponse>,
}

#[derive(Debug, Serialize)]
pub struct LastRoundResultResponse {
    pub round_number: u32,
    pub participant_id: Option<String>,
    pub display_name: Option<String>,
    pub result: String,
    pub points_applied: i32,
}

#[derive(Debug, Serialize)]
pub struct ImportQuestionsResponse {
    pub created: usize,
    pub invalid_rows: Vec<CsvInvalidRowResponse>,
}

#[derive(Debug, Serialize)]
pub struct CsvInvalidRowResponse {
    pub line: usize,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct PlayerAnswerResponse {
    pub accepted: bool,
    pub correct: Option<bool>,
    pub early: bool,
}

pub fn delete_question_command(
    match_id: String,
    question_id: String,
    host_user_id: String,
) -> DeleteMatchQuestionCommand {
    DeleteMatchQuestionCommand {
        match_id,
        question_id,
        host_user_id,
    }
}
