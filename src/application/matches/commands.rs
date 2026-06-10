use crate::domain::games::{MatchMode, McOption, QuestionType, RankingDisplay, ScoringMode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchQuestionDraftCommand {
    pub source_question_id: Option<String>,
    pub question_type: QuestionType,
    pub prompt: String,
    pub expected_answer: String,
    pub options: Vec<McOption>,
    pub points: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateMatchCommand {
    pub mode: MatchMode,
    pub scoring_mode: ScoringMode,
    pub host_user_id: String,
    pub name: String,
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
pub struct AddParticipantCommand {
    pub match_id: String,
    pub display_name: String,
    pub join_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddMatchQuestionCommand {
    pub match_id: String,
    pub host_user_id: String,
    pub source_question_id: Option<String>,
    pub question_type: Option<QuestionType>,
    pub prompt: String,
    pub expected_answer: String,
    pub options: Vec<McOption>,
    pub points: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateMatchQuestionCommand {
    pub match_id: String,
    pub question_id: String,
    pub host_user_id: String,
    pub question_type: Option<QuestionType>,
    pub prompt: String,
    pub expected_answer: String,
    pub options: Vec<McOption>,
    pub points: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteMatchQuestionCommand {
    pub match_id: String,
    pub question_id: String,
    pub host_user_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportMatchQuestionsCommand {
    pub match_id: String,
    pub host_user_id: String,
    pub csv: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawRoundCommand {
    pub match_id: String,
    pub host_user_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartMatchCommand {
    pub match_id: String,
    pub host_user_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinByPinCommand {
    pub pin: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinMatchByIdCommand {
    pub match_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerAnswerCommand {
    pub match_id: String,
    pub round_id: String,
    pub participant_token: String,
    pub answer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuzzCommand {
    pub match_id: String,
    pub round_id: String,
    pub participant_id: String,
    pub presence_session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealAnswerCommand {
    pub match_id: String,
    pub round_id: String,
    pub host_user_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectParticipantCommand {
    pub match_id: String,
    pub round_id: String,
    pub participant_id: String,
    pub host_user_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterAnswerCommand {
    pub match_id: String,
    pub round_id: String,
    pub participant_id: Option<String>,
    pub result: String,
    pub actor_user_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdjustScoreCommand {
    pub match_id: String,
    pub participant_id: String,
    pub actor_user_id: String,
    pub delta: i32,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndMatchCommand {
    pub match_id: String,
    pub host_user_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResetMatchCommand {
    pub match_id: String,
    pub host_user_id: String,
}
