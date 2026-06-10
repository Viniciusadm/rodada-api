use uuid::Uuid;

use crate::application::csv_questions::{parse_csv, ImportQuestionsReport};
use crate::application::questions::{
    validate_question_payload, CreateGeneralQuestionCommand, DeleteGeneralQuestionCommand,
    ImportGeneralQuestionsCommand, ListGeneralQuestionsQuery, QuestionRepository,
    UpdateGeneralQuestionCommand,
};
use crate::domain::questions::Question;
use crate::errors::AppError;

pub struct GeneralQuestionUseCases<R> {
    repository: R,
}

impl<R> GeneralQuestionUseCases<R>
where
    R: QuestionRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn create(
        &self,
        command: CreateGeneralQuestionCommand,
    ) -> Result<Question, AppError> {
        validate_question_payload(
            command.question_type,
            &command.prompt,
            &command.expected_answer,
            &command.options,
        )?;
        let bank_id = self
            .repository
            .find_or_create_default_bank(&command.owner_user_id)
            .await?;
        let id = new_id();
        self.repository
            .create_question(&id, &bank_id, &command)
            .await?;
        self.fetch(&command.owner_user_id, &id).await
    }

    pub async fn import(
        &self,
        command: ImportGeneralQuestionsCommand,
    ) -> Result<ImportQuestionsReport, AppError> {
        let (rows, invalid_rows) = parse_csv(&command.csv);
        if rows.is_empty() && invalid_rows.is_empty() {
            return Err(AppError::Validation("CSV has no question rows".to_owned()));
        }
        let bank_id = self
            .repository
            .find_or_create_default_bank(&command.owner_user_id)
            .await?;
        self.repository
            .import_general_questions(&bank_id, &rows)
            .await?;
        Ok(ImportQuestionsReport {
            created: rows.len(),
            invalid_rows,
        })
    }

    pub async fn update(
        &self,
        command: UpdateGeneralQuestionCommand,
    ) -> Result<Question, AppError> {
        validate_question_payload(
            command.question_type,
            &command.prompt,
            &command.expected_answer,
            &command.options,
        )?;
        let updated = self.repository.update_question(&command).await?;
        if !updated {
            return Err(AppError::NotFound("question was not found".to_owned()));
        }
        self.fetch(&command.owner_user_id, &command.id).await
    }

    pub async fn delete(
        &self,
        command: DeleteGeneralQuestionCommand,
    ) -> Result<(), AppError> {
        let deleted = self.repository.delete_question(&command).await?;
        if !deleted {
            return Err(AppError::NotFound("question was not found".to_owned()));
        }
        Ok(())
    }

    pub async fn list(
        &self,
        query: ListGeneralQuestionsQuery,
    ) -> Result<(Vec<Question>, u64), AppError> {
        self.repository.list_questions(&query).await
    }

    async fn fetch(&self, owner_user_id: &str, id: &str) -> Result<Question, AppError> {
        self.repository
            .get_question(owner_user_id, id)
            .await?
            .ok_or_else(|| AppError::NotFound("question was not found".to_owned()))
    }
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}
