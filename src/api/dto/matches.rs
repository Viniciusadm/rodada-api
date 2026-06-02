use serde::{Deserialize, Serialize};

use crate::application::matches::{
    AddMatchQuestionCommand, AddParticipantCommand, AdjustScoreCommand, CreateMatchCommand,
    DeleteMatchQuestionCommand, ImportMatchQuestionsCommand, JoinMatchByIdCommand,
    MatchQuestionDraftCommand, PlayerAnswerCommand, RegisterAnswerCommand,
    SelectParticipantCommand, UpdateMatchQuestionCommand,
};

#[derive(Debug, Deserialize)]
pub struct MatchQuestionDraftRequest {
    pub source_question_id: Option<String>,
    pub prompt: String,
    pub expected_answer: String,
    pub points: Option<i32>,
}

impl From<MatchQuestionDraftRequest> for MatchQuestionDraftCommand {
    fn from(value: MatchQuestionDraftRequest) -> Self {
        Self {
            source_question_id: value.source_question_id,
            prompt: value.prompt,
            expected_answer: value.expected_answer,
            points: value.points.unwrap_or(100),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateMatchRequest {
    pub organization_id: String,
    pub game_id: String,
    pub game_mode_id: String,
    pub host_user_id: String,
    pub name: String,
    pub max_rounds: Option<u32>,
    pub time_per_round_seconds: Option<u32>,
    pub initial_points: Option<i32>,
    pub remote_enabled: Option<bool>,
    pub capacity: Option<u32>,
    pub questions: Vec<MatchQuestionDraftRequest>,
}

impl From<CreateMatchRequest> for CreateMatchCommand {
    fn from(value: CreateMatchRequest) -> Self {
        Self {
            organization_id: value.organization_id,
            game_id: value.game_id,
            game_mode_id: value.game_mode_id,
            host_user_id: value.host_user_id,
            name: value.name,
            max_rounds: value.max_rounds,
            time_per_round_seconds: value.time_per_round_seconds,
            initial_points: value.initial_points.unwrap_or_default(),
            remote_enabled: value.remote_enabled.unwrap_or(true),
            capacity: value.capacity,
            questions: value.questions.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AddParticipantRequest {
    pub player_id: Option<String>,
    pub display_name: String,
    pub join_source: String,
}

#[derive(Debug, Deserialize)]
pub struct MatchQuestionRequest {
    pub prompt: String,
    pub expected_answer: String,
    pub points: Option<i32>,
}

impl MatchQuestionRequest {
    pub fn into_add_command(
        self,
        match_id: String,
        host_user_id: String,
    ) -> AddMatchQuestionCommand {
        AddMatchQuestionCommand {
            match_id,
            host_user_id,
            prompt: self.prompt,
            expected_answer: self.expected_answer,
            points: self.points.unwrap_or(100),
        }
    }

    pub fn into_update_command(
        self,
        match_id: String,
        question_id: String,
        host_user_id: String,
    ) -> UpdateMatchQuestionCommand {
        UpdateMatchQuestionCommand {
            match_id,
            question_id,
            host_user_id,
            prompt: self.prompt,
            expected_answer: self.expected_answer,
            points: self.points.unwrap_or(100),
        }
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
    pub player_id: Option<String>,
}

impl JoinMatchRequest {
    pub fn into_command(self, match_id: String) -> JoinMatchByIdCommand {
        JoinMatchByIdCommand {
            match_id,
            player_id: self.player_id,
            display_name: self.participant_name,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct PlayerAnswerRequest {
    pub participant_token: String,
}

impl PlayerAnswerRequest {
    pub fn into_command(self, match_id: String, round_id: String) -> PlayerAnswerCommand {
        PlayerAnswerCommand {
            match_id,
            round_id,
            participant_token: self.participant_token,
        }
    }
}

impl AddParticipantRequest {
    pub fn into_command(self, match_id: String) -> AddParticipantCommand {
        AddParticipantCommand {
            match_id,
            player_id: self.player_id,
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
    pub organization_id: String,
    pub game_id: String,
    pub game_mode_id: String,
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
    pub organization_id: String,
    pub game_id: String,
    pub game_mode_id: String,
    pub host_user_id: String,
    pub name: String,
    pub status: String,
    pub current_round_id: Option<String>,
    pub winner_participant_id: Option<String>,
    pub game_name: String,
    pub mode_name: String,
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
    pub player_id: Option<String>,
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
    pub max_rounds: Option<u32>,
    pub time_per_round_seconds: Option<u32>,
    pub initial_points: i32,
    pub remote_enabled: bool,
    pub capacity: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct MatchQuestionResponse {
    pub id: String,
    pub prompt: String,
    pub expected_answer: String,
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
}

#[derive(Debug, Serialize)]
pub struct LiveRoomResponse {
    pub id: String,
    pub match_id: String,
    pub match_name: String,
    pub game_id: String,
    pub game_name: String,
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
