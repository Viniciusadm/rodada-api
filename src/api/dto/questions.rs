use serde::{Deserialize, Serialize};

use crate::application::questions::{
    CreateQuestionBankCommand, CreateQuestionCommand, ImportQuestionsCommand, QuestionDraftCommand,
};

#[derive(Debug, Deserialize)]
pub struct CreateQuestionBankRequest {
    pub game_id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_by_user_id: String,
}

impl From<CreateQuestionBankRequest> for CreateQuestionBankCommand {
    fn from(value: CreateQuestionBankRequest) -> Self {
        Self {
            game_id: value.game_id,
            name: value.name,
            description: value.description,
            created_by_user_id: value.created_by_user_id,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct QuestionDraftRequest {
    pub prompt: String,
    pub expected_answer: String,
    pub default_points: Option<i32>,
    pub difficulty: Option<String>,
}

impl From<QuestionDraftRequest> for QuestionDraftCommand {
    fn from(value: QuestionDraftRequest) -> Self {
        Self {
            prompt: value.prompt,
            expected_answer: value.expected_answer,
            default_points: value.default_points.unwrap_or(100),
            difficulty: value.difficulty,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateQuestionRequest {
    pub question_bank_id: String,
    pub question: QuestionDraftRequest,
}

impl From<CreateQuestionRequest> for CreateQuestionCommand {
    fn from(value: CreateQuestionRequest) -> Self {
        Self {
            question_bank_id: value.question_bank_id,
            question: value.question.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ImportQuestionsRequest {
    pub question_bank_id: String,
    pub imported_by_user_id: String,
    pub questions: Vec<QuestionDraftRequest>,
}

impl From<ImportQuestionsRequest> for ImportQuestionsCommand {
    fn from(value: ImportQuestionsRequest) -> Self {
        Self {
            question_bank_id: value.question_bank_id,
            imported_by_user_id: value.imported_by_user_id,
            questions: value.questions.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct QuestionResponse {
    pub id: String,
    pub question_bank_id: String,
    pub prompt: String,
    pub expected_answer: String,
    pub default_points: i32,
}
