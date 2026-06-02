use async_trait::async_trait;

use crate::application::matches::{
    AddMatchQuestionCommand, AddParticipantCommand, AdjustScoreCommand, BuzzCommand,
    CreateMatchCommand, DeleteMatchQuestionCommand, DrawRoundCommand, EndMatchCommand,
    ImportMatchQuestionsCommand, JoinByPinCommand, JoinMatchByIdCommand, PlayerAnswerCommand,
    RegisterAnswerCommand, ResetMatchCommand, RevealAnswerCommand, StartMatchCommand,
    UpdateMatchQuestionCommand,
};
use crate::domain::matches::{Match, MatchParticipant, ScoreEvent};
use crate::errors::AppError;

#[async_trait]
pub trait MatchRepository: Send + Sync {
    async fn create(&self, id: &str, command: &CreateMatchCommand) -> Result<(), AppError>;
    async fn start(&self, command: &StartMatchCommand) -> Result<(), AppError>;
    async fn draw_round(&self, round_id: &str, command: &DrawRoundCommand) -> Result<(), AppError>;
    async fn join_by_pin(
        &self,
        participant_id: &str,
        token: &str,
        command: &JoinByPinCommand,
    ) -> Result<MatchParticipant, AppError>;
    async fn add_participant(
        &self,
        id: &str,
        command: &AddParticipantCommand,
    ) -> Result<(), AppError>;
    async fn join_match_by_id(
        &self,
        participant_id: &str,
        token: &str,
        command: &JoinMatchByIdCommand,
    ) -> Result<MatchParticipant, AppError>;
    async fn add_match_question(
        &self,
        id: &str,
        command: &AddMatchQuestionCommand,
    ) -> Result<(), AppError>;
    async fn update_match_question(
        &self,
        command: &UpdateMatchQuestionCommand,
    ) -> Result<(), AppError>;
    async fn delete_match_question(
        &self,
        command: &DeleteMatchQuestionCommand,
    ) -> Result<(), AppError>;
    async fn import_match_questions(
        &self,
        command: &ImportMatchQuestionsCommand,
        rows: &[ParsedCsvQuestion],
    ) -> Result<(), AppError>;
    async fn player_answer(
        &self,
        command: &PlayerAnswerCommand,
    ) -> Result<PlayerAnswerOutcome, AppError>;
    async fn buzz(&self, buzzer_event_id: &str, command: &BuzzCommand) -> Result<bool, AppError>;
    async fn reveal_answer(&self, command: &RevealAnswerCommand) -> Result<(), AppError>;
    async fn register_answer(
        &self,
        answer_id: &str,
        command: &RegisterAnswerCommand,
    ) -> Result<(), AppError>;
    async fn record_score_adjustment(
        &self,
        event_id: &str,
        command: &AdjustScoreCommand,
    ) -> Result<(), AppError>;
    async fn end(&self, command: &EndMatchCommand) -> Result<(), AppError>;
    async fn reset(&self, command: &ResetMatchCommand) -> Result<(), AppError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Match>, AppError>;
    async fn list_by_organization(&self, organization_id: &str) -> Result<Vec<Match>, AppError>;
    async fn list_summaries_by_organization(
        &self,
        organization_id: &str,
    ) -> Result<Vec<MatchSummaryState>, AppError> {
        let matches = self.list_by_organization(organization_id).await?;
        Ok(matches
            .into_iter()
            .map(|match_entity| MatchSummaryState {
                match_entity,
                game_name: String::new(),
                mode_name: String::new(),
                question_count: 0,
                participant_count: 0,
                rounds_played: 0,
                current_round_number: None,
                current_round_status: None,
                duration_seconds: None,
                winner_name: None,
                winner_score: None,
                top_scores: Vec::new(),
            })
            .collect())
    }
    async fn list_participants(&self, match_id: &str) -> Result<Vec<MatchParticipant>, AppError>;
    async fn list_score_events(&self, match_id: &str) -> Result<Vec<ScoreEvent>, AppError>;
    async fn list_round_details(
        &self,
        match_id: &str,
    ) -> Result<Vec<MatchRoundDetailState>, AppError>;
    async fn get_state(&self, match_id: &str) -> Result<Option<MatchState>, AppError>;
    async fn list_live_rooms(&self, organization_id: &str) -> Result<Vec<LiveRoomState>, AppError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerAnswerOutcome {
    pub accepted: bool,
    pub correct: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCsvQuestion {
    pub prompt: String,
    pub expected_answer: String,
    pub points: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportQuestionsReport {
    pub created: usize,
    pub invalid_rows: Vec<CsvInvalidRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvInvalidRow {
    pub line: usize,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinMatchResult {
    pub participant: MatchParticipant,
    pub participant_token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchState {
    pub match_entity: Match,
    pub settings: MatchSettingsState,
    pub questions: Vec<MatchQuestionState>,
    pub participants: Vec<MatchParticipant>,
    pub current_round: Option<MatchRoundState>,
    pub room: Option<LiveRoomState>,
    pub winner: Option<MatchParticipant>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchSettingsState {
    pub mode: String,
    pub game_kind: String,
    pub max_rounds: Option<u32>,
    pub time_per_round_seconds: Option<u32>,
    pub initial_points: i32,
    pub remote_enabled: bool,
    pub capacity: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchQuestionState {
    pub id: String,
    pub prompt: String,
    pub expected_answer: String,
    pub points: i32,
    pub used: bool,
    pub position: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchRoundState {
    pub id: String,
    pub match_question_id: Option<String>,
    pub round_number: u32,
    pub status: String,
    pub selected_participant_id: Option<String>,
    pub answer_revealed: bool,
    pub time_remaining_seconds: Option<u32>,
    pub question: Option<MatchQuestionState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveRoomState {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchTopScoreState {
    pub participant_id: String,
    pub display_name: String,
    pub score: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchSummaryState {
    pub match_entity: Match,
    pub game_name: String,
    pub mode_name: String,
    pub question_count: u32,
    pub participant_count: u32,
    pub rounds_played: u32,
    pub current_round_number: Option<u32>,
    pub current_round_status: Option<String>,
    pub duration_seconds: Option<u32>,
    pub winner_name: Option<String>,
    pub winner_score: Option<i32>,
    pub top_scores: Vec<MatchTopScoreState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchRoundDetailState {
    pub round_number: u32,
    pub status: String,
    pub prompt: Option<String>,
    pub expected_answer: Option<String>,
    pub answers: Vec<MatchRoundAnswerState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchRoundAnswerState {
    pub participant_id: Option<String>,
    pub display_name: Option<String>,
    pub result: String,
    pub points_applied: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchRealtimeEventKind {
    Created,
    Updated,
    BuzzerUpdated,
    RoomUpdated,
    RankingUpdated,
}

#[async_trait]
pub trait RealtimeMatchPublisher: Send + Sync {
    async fn publish_match_event(
        &self,
        match_id: &str,
        kind: MatchRealtimeEventKind,
    ) -> Result<(), AppError>;

    async fn publish_organization_event(
        &self,
        organization_id: &str,
        kind: MatchRealtimeEventKind,
    ) -> Result<(), AppError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NoopRealtimeMatchPublisher;

#[async_trait]
impl RealtimeMatchPublisher for NoopRealtimeMatchPublisher {
    async fn publish_match_event(
        &self,
        _match_id: &str,
        _kind: MatchRealtimeEventKind,
    ) -> Result<(), AppError> {
        Ok(())
    }

    async fn publish_organization_event(
        &self,
        _organization_id: &str,
        _kind: MatchRealtimeEventKind,
    ) -> Result<(), AppError> {
        Ok(())
    }
}
