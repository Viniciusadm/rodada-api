use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::questions::{
    CreateQuestionBankCommand, CreateQuestionCommand, ImportQuestionsCommand, QuestionRepository,
};
use crate::domain::questions::{Question, QuestionBank};
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub struct MySqlQuestionRepository {
    pool: MySqlPool,
}

impl MySqlQuestionRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl QuestionRepository for MySqlQuestionRepository {
    async fn create_bank(
        &self,
        id: &str,
        command: &CreateQuestionBankCommand,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO question_banks (id, organization_id, game_id, name, description, created_by_user_id) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&command.organization_id)
        .bind(&command.game_id)
        .bind(&command.name)
        .bind(&command.description)
        .bind(&command.created_by_user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_question(
        &self,
        id: &str,
        command: &CreateQuestionCommand,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO questions (id, question_bank_id, prompt, expected_answer, default_points, difficulty) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&command.question_bank_id)
        .bind(&command.question.prompt)
        .bind(&command.question.expected_answer)
        .bind(command.question.default_points)
        .bind(&command.question.difficulty)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn record_import(
        &self,
        id: &str,
        command: &ImportQuestionsCommand,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO content_imports (id, organization_id, question_bank_id, imported_by_user_id, status, valid_rows, invalid_rows, completed_at) VALUES (?, ?, ?, ?, 'completed', ?, 0, CURRENT_TIMESTAMP(6))",
        )
        .bind(id)
        .bind(&command.organization_id)
        .bind(&command.question_bank_id)
        .bind(&command.imported_by_user_id)
        .bind(command.questions.len() as u32)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_banks(
        &self,
        organization_id: &str,
        game_id: Option<&str>,
    ) -> Result<Vec<QuestionBank>, AppError> {
        let rows = if let Some(game_id) = game_id {
            sqlx::query("SELECT id, organization_id, game_id, name, created_by_user_id, created_at, updated_at FROM question_banks WHERE organization_id = ? AND game_id = ? ORDER BY name")
                .bind(organization_id)
                .bind(game_id)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query("SELECT id, organization_id, game_id, name, created_by_user_id, created_at, updated_at FROM question_banks WHERE organization_id = ? ORDER BY name")
                .bind(organization_id)
                .fetch_all(&self.pool)
                .await?
        };
        rows.into_iter().map(map_bank).collect()
    }

    async fn list_questions(&self, question_bank_id: &str) -> Result<Vec<Question>, AppError> {
        let rows = sqlx::query("SELECT id, question_bank_id, prompt, expected_answer, default_points, is_active, created_at, updated_at FROM questions WHERE question_bank_id = ? ORDER BY created_at")
            .bind(question_bank_id)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_question).collect()
    }
}

fn map_bank(row: sqlx::mysql::MySqlRow) -> Result<QuestionBank, AppError> {
    Ok(QuestionBank {
        id: mapper::id(row.try_get("id")?)?,
        organization_id: mapper::id(row.try_get("organization_id")?)?,
        game_id: mapper::id(row.try_get("game_id")?)?,
        name: mapper::non_empty(row.try_get("name")?, "question_bank.name")?,
        created_by_user_id: mapper::id(row.try_get("created_by_user_id")?)?,
        created_at: mapper::datetime(row.try_get("created_at")?)?,
        updated_at: mapper::datetime(row.try_get("updated_at")?)?,
    })
}

fn map_question(row: sqlx::mysql::MySqlRow) -> Result<Question, AppError> {
    Ok(Question {
        id: mapper::id(row.try_get("id")?)?,
        question_bank_id: mapper::id(row.try_get("question_bank_id")?)?,
        text: mapper::non_empty(row.try_get("prompt")?, "question.prompt")?,
        answer: mapper::non_empty(row.try_get("expected_answer")?, "question.expected_answer")?,
        default_points: mapper::points(row.try_get("default_points")?),
        is_active: row.try_get("is_active")?,
        created_at: mapper::datetime(row.try_get("created_at")?)?,
        updated_at: mapper::datetime(row.try_get("updated_at")?)?,
    })
}
