use async_trait::async_trait;

use crate::application::csv_questions::ParsedCsvQuestion;
use crate::application::questions::{
    CreateGeneralQuestionCommand, DeleteGeneralQuestionCommand, ListGeneralQuestionsQuery,
    UpdateGeneralQuestionCommand,
};
use crate::domain::questions::Question;
use crate::errors::AppError;

#[async_trait]
pub trait QuestionRepository: Send + Sync {
    async fn find_or_create_default_bank(&self, owner_user_id: &str) -> Result<String, AppError>;
    async fn create_question(
        &self,
        id: &str,
        question_bank_id: &str,
        command: &CreateGeneralQuestionCommand,
    ) -> Result<(), AppError>;
    async fn import_general_questions(
        &self,
        question_bank_id: &str,
        rows: &[ParsedCsvQuestion],
    ) -> Result<(), AppError>;
    async fn update_question(
        &self,
        command: &UpdateGeneralQuestionCommand,
    ) -> Result<bool, AppError>;
    async fn delete_question(
        &self,
        command: &DeleteGeneralQuestionCommand,
    ) -> Result<bool, AppError>;
    async fn get_question(
        &self,
        owner_user_id: &str,
        id: &str,
    ) -> Result<Option<Question>, AppError>;
    async fn list_questions(
        &self,
        query: &ListGeneralQuestionsQuery,
    ) -> Result<(Vec<Question>, u64), AppError>;
}
