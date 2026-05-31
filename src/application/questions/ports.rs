use async_trait::async_trait;

use crate::application::questions::{
    CreateQuestionBankCommand, CreateQuestionCommand, ImportQuestionsCommand,
};
use crate::domain::questions::{Question, QuestionBank};
use crate::errors::AppError;

#[async_trait]
pub trait QuestionRepository: Send + Sync {
    async fn create_bank(
        &self,
        id: &str,
        command: &CreateQuestionBankCommand,
    ) -> Result<(), AppError>;
    async fn create_question(
        &self,
        id: &str,
        command: &CreateQuestionCommand,
    ) -> Result<(), AppError>;
    async fn record_import(
        &self,
        id: &str,
        command: &ImportQuestionsCommand,
    ) -> Result<(), AppError>;
    async fn list_banks(
        &self,
        organization_id: &str,
        game_id: Option<&str>,
    ) -> Result<Vec<QuestionBank>, AppError>;
    async fn list_questions(&self, question_bank_id: &str) -> Result<Vec<Question>, AppError>;
}
