use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::match_templates::{
    CreateMatchTemplateCommand, DeleteMatchTemplateCommand, MatchTemplateQuestionState,
    MatchTemplateRepository, MatchTemplateState, MatchTemplateSummaryState,
    UpdateMatchTemplateCommand,
};
use crate::application::matches::MatchQuestionDraftCommand;
use crate::domain::games::{McOption, McOptions};
use crate::errors::AppError;

#[derive(Clone)]
pub struct MySqlMatchTemplateRepository {
    pool: MySqlPool,
}

impl MySqlMatchTemplateRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    async fn insert_questions(
        &self,
        tx: &mut sqlx::mysql::MySqlConnection,
        template_id: &str,
        questions: &[MatchQuestionDraftCommand],
    ) -> Result<(), AppError> {
        for (index, question) in questions.iter().enumerate() {
            sqlx::query(
                "INSERT INTO match_template_questions (id, template_id, source_question_id, position, question_type, prompt, expected_answer, options, points)
                 VALUES (UUID(), ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(template_id)
            .bind(&question.source_question_id)
            .bind(index as i32 + 1)
            .bind(question.question_type.as_code())
            .bind(&question.prompt)
            .bind(&question.expected_answer)
            .bind(serialize_options(&question.options))
            .bind(question.points)
            .execute(&mut *tx)
            .await?;
        }
        Ok(())
    }
}

#[async_trait]
impl MatchTemplateRepository for MySqlMatchTemplateRepository {
    async fn create(&self, id: &str, command: &CreateMatchTemplateCommand) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO match_templates (id, host_user_id, name, description, mode, scoring_mode, max_rounds, time_per_round_seconds, initial_points, remote_enabled, capacity, buzzer_arm_delay_seconds, ranking_display, present_screen)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&command.host_user_id)
        .bind(&command.name)
        .bind(&command.description)
        .bind(command.mode.as_code())
        .bind(command.scoring_mode.as_code())
        .bind(command.max_rounds)
        .bind(command.time_per_round_seconds)
        .bind(command.initial_points)
        .bind(command.remote_enabled)
        .bind(command.capacity)
        .bind(command.buzzer_arm_delay_seconds)
        .bind(command.ranking_display.as_code())
        .bind(&command.present_screen)
        .execute(&mut *tx)
        .await?;

        self.insert_questions(&mut tx, id, &command.questions)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn update(&self, command: &UpdateMatchTemplateCommand) -> Result<bool, AppError> {
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query(
            "UPDATE match_templates
             SET name = ?, description = ?, mode = ?, scoring_mode = ?, max_rounds = ?, time_per_round_seconds = ?, initial_points = ?, remote_enabled = ?, capacity = ?, buzzer_arm_delay_seconds = ?, ranking_display = ?, present_screen = ?
             WHERE id = ? AND host_user_id = ?",
        )
        .bind(&command.name)
        .bind(&command.description)
        .bind(command.mode.as_code())
        .bind(command.scoring_mode.as_code())
        .bind(command.max_rounds)
        .bind(command.time_per_round_seconds)
        .bind(command.initial_points)
        .bind(command.remote_enabled)
        .bind(command.capacity)
        .bind(command.buzzer_arm_delay_seconds)
        .bind(command.ranking_display.as_code())
        .bind(&command.present_screen)
        .bind(&command.id)
        .bind(&command.host_user_id)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            return Ok(false);
        }

        sqlx::query("DELETE FROM match_template_questions WHERE template_id = ?")
            .bind(&command.id)
            .execute(&mut *tx)
            .await?;
        self.insert_questions(&mut tx, &command.id, &command.questions)
            .await?;

        tx.commit().await?;
        Ok(true)
    }

    async fn delete(&self, command: &DeleteMatchTemplateCommand) -> Result<bool, AppError> {
        let result = sqlx::query("DELETE FROM match_templates WHERE id = ? AND host_user_id = ?")
            .bind(&command.id)
            .bind(&command.host_user_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn list_summaries_by_owner(
        &self,
        owner_user_id: &str,
        search: Option<&str>,
    ) -> Result<Vec<MatchTemplateSummaryState>, AppError> {
        let pattern = search
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("%{value}%"));
        let rows = sqlx::query(
            "SELECT mt.id, mt.name, mt.description, mt.mode, mt.scoring_mode, mt.created_at, mt.updated_at,
                    COUNT(mtq.id) AS question_count
             FROM match_templates mt
             LEFT JOIN match_template_questions mtq ON mtq.template_id = mt.id
             WHERE mt.host_user_id = ? AND (? IS NULL OR mt.name LIKE ?)
             GROUP BY mt.id, mt.name, mt.description, mt.mode, mt.scoring_mode, mt.created_at, mt.updated_at
             ORDER BY mt.updated_at DESC",
        )
        .bind(owner_user_id)
        .bind(pattern.as_deref())
        .bind(pattern.as_deref())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_summary).collect()
    }

    async fn get_with_questions(
        &self,
        owner_user_id: &str,
        id: &str,
    ) -> Result<Option<MatchTemplateState>, AppError> {
        let row = sqlx::query(
            "SELECT id, host_user_id, name, description, mode, scoring_mode, max_rounds, time_per_round_seconds, initial_points, remote_enabled, capacity, buzzer_arm_delay_seconds, ranking_display, present_screen, created_at, updated_at
             FROM match_templates WHERE id = ? AND host_user_id = ?",
        )
        .bind(id)
        .bind(owner_user_id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(None);
        };

        let question_rows = sqlx::query(
            "SELECT id, source_question_id, position, question_type, prompt, expected_answer, options, points
             FROM match_template_questions WHERE template_id = ? ORDER BY position",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;
        let questions = question_rows
            .into_iter()
            .map(map_question)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Some(map_template(row, questions)?))
    }
}

fn mode_label(mode: &str) -> String {
    match mode {
        "quiz" => "Quiz",
        _ => "Buzzer",
    }
    .to_owned()
}

fn scoring_mode_label(code: &str) -> String {
    match code {
        "initial_points" => "Pontos Iniciais",
        "elimination" => "Eliminação",
        _ => "Clássico",
    }
    .to_owned()
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

fn map_summary(row: sqlx::mysql::MySqlRow) -> Result<MatchTemplateSummaryState, AppError> {
    let mode: String = row.try_get("mode")?;
    let scoring_mode: String = row.try_get("scoring_mode")?;
    let question_count: i64 = row.try_get("question_count")?;
    let created_at: chrono::NaiveDateTime = row.try_get("created_at")?;
    let updated_at: chrono::NaiveDateTime = row.try_get("updated_at")?;
    Ok(MatchTemplateSummaryState {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        mode_label: mode_label(&mode),
        scoring_mode_label: scoring_mode_label(&scoring_mode),
        mode,
        scoring_mode,
        question_count: question_count.max(0) as u32,
        created_at: created_at.to_string(),
        updated_at: updated_at.to_string(),
    })
}

fn map_template(
    row: sqlx::mysql::MySqlRow,
    questions: Vec<MatchTemplateQuestionState>,
) -> Result<MatchTemplateState, AppError> {
    let mode: String = row.try_get("mode")?;
    let scoring_mode: String = row.try_get("scoring_mode")?;
    let created_at: chrono::NaiveDateTime = row.try_get("created_at")?;
    let updated_at: chrono::NaiveDateTime = row.try_get("updated_at")?;
    Ok(MatchTemplateState {
        id: row.try_get("id")?,
        host_user_id: row.try_get("host_user_id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        mode_label: mode_label(&mode),
        scoring_mode_label: scoring_mode_label(&scoring_mode),
        mode,
        scoring_mode,
        max_rounds: row
            .try_get::<Option<i32>, _>("max_rounds")?
            .map(|value| value as u32),
        time_per_round_seconds: row
            .try_get::<Option<i32>, _>("time_per_round_seconds")?
            .map(|value| value as u32),
        initial_points: row.try_get("initial_points")?,
        remote_enabled: row.try_get("remote_enabled")?,
        capacity: row
            .try_get::<Option<i32>, _>("capacity")?
            .map(|value| value as u32),
        buzzer_arm_delay_seconds: row.try_get::<i32, _>("buzzer_arm_delay_seconds")?.max(0) as u32,
        ranking_display: row.try_get("ranking_display")?,
        present_screen: row.try_get("present_screen")?,
        questions,
        created_at: created_at.to_string(),
        updated_at: updated_at.to_string(),
    })
}

fn map_question(row: sqlx::mysql::MySqlRow) -> Result<MatchTemplateQuestionState, AppError> {
    let position: i32 = row.try_get("position")?;
    let options_raw: Option<String> = row.try_get("options")?;
    Ok(MatchTemplateQuestionState {
        id: row.try_get("id")?,
        source_question_id: row.try_get("source_question_id")?,
        question_type: row.try_get("question_type")?,
        prompt: row.try_get("prompt")?,
        expected_answer: row.try_get("expected_answer")?,
        options: deserialize_options(options_raw.as_deref()),
        points: row.try_get("points")?,
        position: position.max(0) as u32,
    })
}
