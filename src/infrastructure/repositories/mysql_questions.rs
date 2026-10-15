use async_trait::async_trait;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::application::csv_questions::ParsedCsvQuestion;
use crate::application::questions::{
    CreateGeneralQuestionCommand, DeleteGeneralQuestionCommand, ListGeneralQuestionsQuery,
    QuestionRepository, UpdateGeneralQuestionCommand,
};
use crate::domain::games::{McOption, McOptions, QuestionType};
use crate::domain::questions::Question;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

const DEFAULT_BANK_NAME: &str = "Perguntas Gerais";

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
    async fn find_or_create_default_bank(&self, owner_user_id: &str) -> Result<String, AppError> {
        let existing = sqlx::query(
            "SELECT id FROM question_banks WHERE created_by_user_id = ? AND name = ? AND game_id IS NULL LIMIT 1",
        )
        .bind(owner_user_id)
        .bind(DEFAULT_BANK_NAME)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = existing {
            return Ok(row.try_get::<String, _>("id")?);
        }

        let id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO question_banks (id, game_id, name, created_by_user_id) VALUES (?, NULL, ?, ?)",
        )
        .bind(&id)
        .bind(DEFAULT_BANK_NAME)
        .bind(owner_user_id)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    async fn create_question(
        &self,
        id: &str,
        question_bank_id: &str,
        command: &CreateGeneralQuestionCommand,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO questions (id, question_bank_id, question_type, prompt, expected_answer, options, default_points) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(question_bank_id)
        .bind(command.question_type.as_code())
        .bind(&command.prompt)
        .bind(&command.expected_answer)
        .bind(serialize_options(&command.options))
        .bind(command.default_points)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn import_general_questions(
        &self,
        question_bank_id: &str,
        rows: &[ParsedCsvQuestion],
    ) -> Result<(), AppError> {
        if rows.is_empty() {
            return Ok(());
        }
        let mut tx = self.pool.begin().await?;
        for row in rows {
            let options = if row.question_type == QuestionType::MultipleChoice {
                serialize_options(&row.options)
            } else {
                None
            };
            sqlx::query(
                "INSERT INTO questions (id, question_bank_id, question_type, prompt, expected_answer, options, default_points) VALUES (UUID(), ?, ?, ?, ?, ?, ?)",
            )
            .bind(question_bank_id)
            .bind(row.question_type.as_code())
            .bind(&row.prompt)
            .bind(&row.expected_answer)
            .bind(options)
            .bind(row.points)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn update_question(
        &self,
        command: &UpdateGeneralQuestionCommand,
    ) -> Result<bool, AppError> {
        let owned = sqlx::query(
            "SELECT 1 AS owned FROM questions q JOIN question_banks b ON q.question_bank_id = b.id WHERE q.id = ? AND b.created_by_user_id = ?",
        )
        .bind(&command.id)
        .bind(&command.owner_user_id)
        .fetch_optional(&self.pool)
        .await?;
        if owned.is_none() {
            return Ok(false);
        }

        sqlx::query(
            "UPDATE questions SET question_type = ?, prompt = ?, expected_answer = ?, options = ?, default_points = ? WHERE id = ?",
        )
        .bind(command.question_type.as_code())
        .bind(&command.prompt)
        .bind(&command.expected_answer)
        .bind(serialize_options(&command.options))
        .bind(command.default_points)
        .bind(&command.id)
        .execute(&self.pool)
        .await?;
        Ok(true)
    }

    async fn delete_question(
        &self,
        command: &DeleteGeneralQuestionCommand,
    ) -> Result<bool, AppError> {
        let result = sqlx::query(
            "DELETE q FROM questions q JOIN question_banks b ON q.question_bank_id = b.id WHERE q.id = ? AND b.created_by_user_id = ?",
        )
        .bind(&command.id)
        .bind(&command.owner_user_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn get_question(
        &self,
        owner_user_id: &str,
        id: &str,
    ) -> Result<Option<Question>, AppError> {
        let row = sqlx::query(
            "SELECT q.id, q.question_bank_id, q.question_type, q.prompt, q.expected_answer, q.options, q.default_points, q.is_active, q.created_at, q.updated_at \
             FROM questions q JOIN question_banks b ON q.question_bank_id = b.id \
             WHERE q.id = ? AND b.created_by_user_id = ?",
        )
        .bind(id)
        .bind(owner_user_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(map_question).transpose()
    }

    async fn list_questions(
        &self,
        query: &ListGeneralQuestionsQuery,
    ) -> Result<(Vec<Question>, u64), AppError> {
        let page = query.page.max(1);
        let page_size = query.page_size.clamp(1, 100);
        let offset = (page - 1) * page_size;
        let search = query
            .search
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));

        let (rows, total) = if let Some(pattern) = &search {
            let rows = sqlx::query(
                "SELECT q.id, q.question_bank_id, q.question_type, q.prompt, q.expected_answer, q.options, q.default_points, q.is_active, q.created_at, q.updated_at \
                 FROM questions q JOIN question_banks b ON q.question_bank_id = b.id \
                 WHERE b.created_by_user_id = ? AND q.prompt LIKE ? \
                 ORDER BY q.created_at DESC LIMIT ? OFFSET ?",
            )
            .bind(&query.owner_user_id)
            .bind(pattern)
            .bind(page_size)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
            let total: i64 = sqlx::query(
                "SELECT COUNT(*) AS total FROM questions q JOIN question_banks b ON q.question_bank_id = b.id WHERE b.created_by_user_id = ? AND q.prompt LIKE ?",
            )
            .bind(&query.owner_user_id)
            .bind(pattern)
            .fetch_one(&self.pool)
            .await?
            .try_get("total")?;
            (rows, total)
        } else {
            let rows = sqlx::query(
                "SELECT q.id, q.question_bank_id, q.question_type, q.prompt, q.expected_answer, q.options, q.default_points, q.is_active, q.created_at, q.updated_at \
                 FROM questions q JOIN question_banks b ON q.question_bank_id = b.id \
                 WHERE b.created_by_user_id = ? \
                 ORDER BY q.created_at DESC LIMIT ? OFFSET ?",
            )
            .bind(&query.owner_user_id)
            .bind(page_size)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
            let total: i64 = sqlx::query(
                "SELECT COUNT(*) AS total FROM questions q JOIN question_banks b ON q.question_bank_id = b.id WHERE b.created_by_user_id = ?",
            )
            .bind(&query.owner_user_id)
            .fetch_one(&self.pool)
            .await?
            .try_get("total")?;
            (rows, total)
        };

        let questions = rows
            .into_iter()
            .map(map_question)
            .collect::<Result<Vec<_>, _>>()?;
        Ok((questions, total.max(0) as u64))
    }
}

fn map_question(row: sqlx::mysql::MySqlRow) -> Result<Question, AppError> {
    let options_raw: Option<String> = row.try_get("options")?;
    let question_type =
        QuestionType::from_code(row.try_get::<String, _>("question_type")?.as_str())
            .unwrap_or(QuestionType::FreeText);
    Ok(Question {
        id: mapper::id(row.try_get("id")?)?,
        question_bank_id: mapper::id(row.try_get("question_bank_id")?)?,
        question_type,
        text: mapper::non_empty(row.try_get("prompt")?, "question.prompt")?,
        answer: mapper::non_empty(row.try_get("expected_answer")?, "question.expected_answer")?,
        options: deserialize_options(options_raw.as_deref()),
        default_points: mapper::points(row.try_get("default_points")?),
        is_active: row.try_get("is_active")?,
        created_at: mapper::datetime(row.try_get("created_at")?)?,
        updated_at: mapper::datetime(row.try_get("updated_at")?)?,
    })
}

fn serialize_options(options: &[McOption]) -> Option<String> {
    if options.is_empty() {
        return None;
    }
    serde_json::to_string(&McOptions {
        options: options.to_vec(),
    })
    .ok()
}

fn deserialize_options(raw: Option<&str>) -> Vec<McOption> {
    raw.and_then(|value| serde_json::from_str::<McOptions>(value).ok())
        .map(|parsed| parsed.options)
        .unwrap_or_default()
}
