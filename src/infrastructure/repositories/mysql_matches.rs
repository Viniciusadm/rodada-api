use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::matches::{
    AddMatchQuestionCommand, AddParticipantCommand, AdjustScoreCommand, BuzzCommand, BuzzOutcome,
    CreateMatchCommand, DeleteMatchQuestionCommand, DrawRoundCommand, EndMatchCommand,
    ImportMatchQuestionsCommand, JoinByPinCommand, JoinMatchByIdCommand, LiveRoomState,
    MatchQuestionState, MatchRepository, MatchRoundAnswerState, MatchRoundDetailState,
    MatchRoundState, MatchSettingsState, MatchState, MatchSummaryState, MatchTopScoreState,
    ParsedCsvQuestion, PlayerAnswerCommand, PlayerAnswerOutcome, RegisterAnswerCommand,
    ResetMatchCommand, RevealAnswerCommand, StartMatchCommand, UpdateMatchQuestionCommand,
};
use crate::domain::games::{MatchMode, McOption, McOptions, QuestionType};
use crate::domain::matches::{Match, MatchParticipant, ScoreEvent};
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub struct MySqlMatchRepository {
    pool: MySqlPool,
}

impl MySqlMatchRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    async fn create_participant_token(
        &self,
        match_id: &str,
        participant_id: &str,
        token: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO match_participant_tokens (id, match_id, participant_id, token)
             VALUES (UUID(), ?, ?, ?)",
        )
        .bind(match_id)
        .bind(participant_id)
        .bind(token)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn auto_answer(
        &self,
        command: &PlayerAnswerCommand,
        participant_id: &str,
        question_type: QuestionType,
    ) -> Result<PlayerAnswerOutcome, AppError> {
        let mut tx = self.pool.begin().await?;

        let eliminated: Option<chrono::NaiveDateTime> = sqlx::query_scalar(
            "SELECT eliminated_at FROM match_participants WHERE id = ? AND match_id = ?",
        )
        .bind(participant_id)
        .bind(&command.match_id)
        .fetch_optional(&mut *tx)
        .await?
        .flatten();
        if eliminated.is_some() {
            return Err(AppError::Conflict(
                "participant has been eliminated".to_owned(),
            ));
        }

        let round: Option<(String, Option<String>, Option<String>, Option<i32>, i64, Option<i32>)> =
            sqlx::query_as(
                "SELECT mr.status, mq.expected_answer_snapshot, mq.options_snapshot, mq.points,
                        TIMESTAMPDIFF(SECOND, mr.opened_at, CURRENT_TIMESTAMP(6)) AS elapsed,
                        ms.time_per_round_seconds
                 FROM match_rounds mr
                 JOIN match_settings ms ON ms.match_id = mr.match_id
                 LEFT JOIN match_questions mq ON mq.id = mr.match_question_id
                 WHERE mr.id = ? AND mr.match_id = ?",
            )
            .bind(&command.round_id)
            .bind(&command.match_id)
            .fetch_optional(&mut *tx)
            .await?;
        let Some((status, expected, options_raw, points, elapsed, limit)) = round else {
            return Err(AppError::NotFound("round was not found".to_owned()));
        };
        if status != "open" {
            return Err(AppError::Conflict("round is not open".to_owned()));
        }
        if let Some(limit) = limit {
            if elapsed > i64::from(limit) {
                return Err(AppError::Conflict(
                    "time for this question has expired".to_owned(),
                ));
            }
        }

        let submitted = command.answer.clone().unwrap_or_default();
        let expected = expected.unwrap_or_default();
        let points = points.unwrap_or_default();
        let options = deserialize_options(options_raw.as_deref());
        let correct = question_type.grade(&expected, Some(&options), &submitted);
        let result = if correct { "correct" } else { "incorrect" };
        let points_applied = if correct { points } else { 0 };

        let inserted = sqlx::query(
            "INSERT INTO match_answers (id, match_id, match_round_id, participant_id, result, player_answer, points_applied) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&command.match_id)
        .bind(&command.round_id)
        .bind(participant_id)
        .bind(result)
        .bind(&submitted)
        .bind(points_applied)
        .execute(&mut *tx)
        .await;
        if let Err(sqlx::Error::Database(db_err)) = &inserted {
            if db_err.is_unique_violation() {
                tx.rollback().await?;
                return Ok(PlayerAnswerOutcome {
                    accepted: false,
                    correct: None,
                    early: false,
                });
            }
        }
        inserted?;

        let current: Option<i32> = sqlx::query_scalar(
            "SELECT current_score FROM match_participants WHERE id = ? AND match_id = ? FOR UPDATE",
        )
        .bind(participant_id)
        .bind(&command.match_id)
        .fetch_optional(&mut *tx)
        .await?;
        let score_after = current.unwrap_or_default() + points_applied;
        sqlx::query("UPDATE match_participants SET current_score = ? WHERE id = ?")
            .bind(score_after)
            .bind(participant_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO score_events (id, match_id, participant_id, match_round_id, event_type, delta, score_after) VALUES (UUID(), ?, ?, ?, ?, ?, ?)",
        )
        .bind(&command.match_id)
        .bind(participant_id)
        .bind(&command.round_id)
        .bind(if correct { "correct_answer" } else { "incorrect_answer" })
        .bind(points_applied)
        .bind(score_after)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(PlayerAnswerOutcome {
            accepted: true,
            correct: Some(correct),
            early: false,
        })
    }
}

#[async_trait]
impl MatchRepository for MySqlMatchRepository {
    async fn create_from_template(
        &self,
        id: &str,
        command: &CreateMatchCommand,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        let template_name: Option<String> = sqlx::query_scalar(
            "SELECT name FROM match_templates WHERE id = ? AND host_user_id = ?",
        )
        .bind(&command.template_id)
        .bind(&command.host_user_id)
        .fetch_optional(&mut *tx)
        .await?;
        let Some(template_name) = template_name else {
            return Err(diagnose_template_failure(&mut tx, &command.template_id, &command.host_user_id).await?);
        };

        let question_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM match_template_questions WHERE template_id = ?",
        )
        .bind(&command.template_id)
        .fetch_one(&mut *tx)
        .await?;
        if question_count == 0 {
            return Err(AppError::conflict_code(
                "template_no_questions",
                "template has no questions",
            ));
        }

        let name = command
            .name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .unwrap_or(template_name);

        sqlx::query(
            "INSERT INTO matches (id, mode, template_id, host_user_id, name, status)
             SELECT ?, mt.mode, mt.id, mt.host_user_id, ?, 'waiting'
             FROM match_templates mt WHERE mt.id = ?",
        )
        .bind(id)
        .bind(&name)
        .bind(&command.template_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO match_settings (match_id, max_rounds, time_per_round_seconds, initial_points, scoring_mode, remote_enabled, capacity, buzzer_arm_delay_seconds, ranking_display)
             SELECT ?, mt.max_rounds, mt.time_per_round_seconds, mt.initial_points, mt.scoring_mode, mt.remote_enabled, mt.capacity, mt.buzzer_arm_delay_seconds, mt.ranking_display
             FROM match_templates mt WHERE mt.id = ?",
        )
        .bind(id)
        .bind(&command.template_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO match_questions (id, match_id, source_question_id, position, question_type, prompt_snapshot, expected_answer_snapshot, options_snapshot, points)
             SELECT UUID(), ?, mtq.source_question_id, mtq.position, mtq.question_type, mtq.prompt, mtq.expected_answer, mtq.options, mtq.points
             FROM match_template_questions mtq WHERE mtq.template_id = ?
             ORDER BY mtq.position",
        )
        .bind(id)
        .bind(&command.template_id)
        .execute(&mut *tx)
        .await?;

        let remote_enabled: bool = sqlx::query_scalar(
            "SELECT remote_enabled FROM match_settings WHERE match_id = ?",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        if remote_enabled {
            sqlx::query(
                "INSERT INTO room_sessions (id, match_id, pin, status, capacity)
                 SELECT UUID(), ?, LPAD(FLOOR(RAND() * 1000000), 6, '0'), 'waiting', capacity
                 FROM match_settings WHERE match_id = ?",
            )
            .bind(id)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn start(&self, command: &StartMatchCommand) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query(
            "UPDATE matches m
             SET m.status = 'in_progress', m.started_at = CURRENT_TIMESTAMP(6)
             WHERE m.id = ? AND m.host_user_id = ? AND m.status = 'waiting'
               AND EXISTS (SELECT 1 FROM match_participants mp WHERE mp.match_id = m.id)
               AND EXISTS (SELECT 1 FROM match_questions mq WHERE mq.match_id = m.id)",
        )
        .bind(&command.match_id)
        .bind(&command.host_user_id)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            return Err(diagnose_start_failure(&mut tx, &command.match_id, &command.host_user_id).await?);
        }
        sqlx::query(
            "UPDATE room_sessions SET status = 'live' WHERE match_id = ? AND status = 'waiting'",
        )
        .bind(&command.match_id)
        .execute(&mut *tx)
        .await?;

        let mode = fetch_scoring_mode(&mut tx, &command.match_id).await?;
        let initial_points: i32 =
            sqlx::query_scalar("SELECT initial_points FROM match_settings WHERE match_id = ?")
                .bind(&command.match_id)
                .fetch_one(&mut *tx)
                .await?;
        match mode.as_str() {
            "initial_points" => {
                seed_initial_points(&mut tx, &command.match_id, initial_points).await?
            }
            "elimination" => seed_lives(&mut tx, &command.match_id, initial_points).await?,
            _ => {}
        }

        tx.commit().await?;
        Ok(())
    }

    async fn draw_round(&self, round_id: &str, command: &DrawRoundCommand) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        let max_rounds: Option<i32> =
            sqlx::query_scalar("SELECT max_rounds FROM match_settings WHERE match_id = ?")
                .bind(&command.match_id)
                .fetch_optional(&mut *tx)
                .await?
                .flatten();
        if let Some(max) = max_rounds {
            let played: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM match_rounds WHERE match_id = ?")
                    .bind(&command.match_id)
                    .fetch_one(&mut *tx)
                    .await?;
            if played >= max as i64 {
                return Err(AppError::conflict_code(
                    "match_round_limit",
                    "round limit reached",
                ));
            }
        }

        let question_id: Option<String> = sqlx::query_scalar(
            "SELECT mq.id
             FROM match_questions mq
             JOIN matches m ON m.id = mq.match_id
             WHERE mq.match_id = ? AND mq.used_at IS NULL AND m.host_user_id = ? AND m.status = 'in_progress'
               AND m.current_round_id IS NULL
             ORDER BY mq.position
             LIMIT 1
             FOR UPDATE",
        )
        .bind(&command.match_id)
        .bind(&command.host_user_id)
        .fetch_optional(&mut *tx)
        .await?;
        // The single query above can fail for several distinct reasons; when it
        // returns nothing, diagnose the specific cause so the host gets an
        // actionable, coded error instead of a catch-all message.
        let question_id = match question_id {
            Some(id) => id,
            None => return Err(diagnose_draw_failure(&mut tx, &command.match_id, &command.host_user_id).await?),
        };
        let next_round: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(round_number), 0) + 1 FROM match_rounds WHERE match_id = ?",
        )
        .bind(&command.match_id)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO match_rounds (id, match_id, match_question_id, round_number, status) VALUES (?, ?, ?, ?, 'open')",
        )
        .bind(round_id)
        .bind(&command.match_id)
        .bind(&question_id)
        .bind(next_round)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE matches SET current_round_id = ? WHERE id = ?")
            .bind(round_id)
            .bind(&command.match_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE match_questions SET used_at = CURRENT_TIMESTAMP(6) WHERE id = ?")
            .bind(&question_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn join_by_pin(
        &self,
        participant_id: &str,
        token: &str,
        command: &JoinByPinCommand,
    ) -> Result<MatchParticipant, AppError> {
        let match_id: Option<String> = sqlx::query_scalar(
            "SELECT rs.match_id FROM room_sessions rs WHERE rs.pin = ? AND rs.status IN ('waiting', 'live') LIMIT 1",
        )
        .bind(&command.pin)
        .fetch_optional(&self.pool)
        .await?;
        let match_id =
            match_id.ok_or_else(|| AppError::NotFound("room pin was not found".to_owned()))?;
        let add = AddParticipantCommand {
            match_id: match_id.clone(),
            display_name: command.display_name.clone(),
            join_source: "remote".to_owned(),
        };
        self.add_participant(participant_id, &add).await?;
        self.create_participant_token(&match_id, participant_id, token)
            .await?;
        let participants = self.list_participants(&match_id).await?;
        participants
            .into_iter()
            .find(|participant| participant.id.as_str() == participant_id)
            .ok_or_else(|| AppError::NotFound("joined participant was not found".to_owned()))
    }

    async fn add_participant(
        &self,
        id: &str,
        command: &AddParticipantCommand,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        let capacity: Option<i32> =
            sqlx::query_scalar("SELECT capacity FROM match_settings WHERE match_id = ?")
                .bind(&command.match_id)
                .fetch_optional(&mut *tx)
                .await?
                .flatten();
        if let Some(capacity) = capacity {
            let current: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM match_participants WHERE match_id = ? AND left_at IS NULL",
            )
            .bind(&command.match_id)
            .fetch_one(&mut *tx)
            .await?;
            if current >= i64::from(capacity) {
                return Err(AppError::Conflict("room is full".to_owned()));
            }
        }

        let inserted = sqlx::query(
            "INSERT INTO match_participants (id, match_id, display_name_snapshot, normalized_name_snapshot, join_source) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&command.match_id)
        .bind(&command.display_name)
        .bind(mapper::normalized_name(&command.display_name))
        .bind(&command.join_source)
        .execute(&mut *tx)
        .await;
        if let Err(sqlx::Error::Database(db_err)) = &inserted {
            if db_err.is_unique_violation() {
                return Err(AppError::Conflict("name already in use".to_owned()));
            }
        }
        inserted?;

        let status: Option<String> = sqlx::query_scalar("SELECT status FROM matches WHERE id = ?")
            .bind(&command.match_id)
            .fetch_optional(&mut *tx)
            .await?;
        if status.as_deref() == Some("in_progress") {
            let mode = fetch_scoring_mode(&mut tx, &command.match_id).await?;
            let initial_points: i32 =
                sqlx::query_scalar("SELECT initial_points FROM match_settings WHERE match_id = ?")
                    .bind(&command.match_id)
                    .fetch_one(&mut *tx)
                    .await?;
            match mode.as_str() {
                "initial_points" => {
                    sqlx::query("UPDATE match_participants SET current_score = ? WHERE id = ?")
                        .bind(initial_points)
                        .bind(id)
                        .execute(&mut *tx)
                        .await?;
                    sqlx::query(
                        "INSERT INTO score_events (id, match_id, participant_id, event_type, delta, score_after) VALUES (UUID(), ?, ?, 'initial_points', ?, ?)",
                    )
                    .bind(&command.match_id)
                    .bind(id)
                    .bind(initial_points)
                    .bind(initial_points)
                    .execute(&mut *tx)
                    .await?;
                }
                "elimination" => {
                    sqlx::query("UPDATE match_participants SET lives_remaining = ? WHERE id = ?")
                        .bind(initial_points)
                        .bind(id)
                        .execute(&mut *tx)
                        .await?;
                }
                _ => {}
            }
        }

        tx.commit().await?;
        Ok(())
    }

    async fn join_match_by_id(
        &self,
        participant_id: &str,
        token: &str,
        command: &JoinMatchByIdCommand,
    ) -> Result<MatchParticipant, AppError> {
        let exists: Option<String> = sqlx::query_scalar(
            "SELECT m.id
             FROM matches m
             JOIN match_settings ms ON ms.match_id = m.id
             WHERE m.id = ? AND ms.remote_enabled = TRUE AND m.status IN ('waiting', 'in_progress')",
        )
        .bind(&command.match_id)
        .fetch_optional(&self.pool)
        .await?;
        let match_id =
            exists.ok_or_else(|| AppError::NotFound("joinable match was not found".to_owned()))?;
        let add = AddParticipantCommand {
            match_id: match_id.clone(),
            display_name: command.display_name.clone(),
            join_source: "remote".to_owned(),
        };
        self.add_participant(participant_id, &add).await?;
        self.create_participant_token(&match_id, participant_id, token)
            .await?;
        let participants = self.list_participants(&match_id).await?;
        participants
            .into_iter()
            .find(|participant| participant.id.as_str() == participant_id)
            .ok_or_else(|| AppError::NotFound("joined participant was not found".to_owned()))
    }

    async fn add_match_question(
        &self,
        id: &str,
        command: &AddMatchQuestionCommand,
    ) -> Result<(), AppError> {
        let mode: Option<String> =
            sqlx::query_scalar("SELECT mode FROM matches WHERE id = ? AND host_user_id = ?")
                .bind(&command.match_id)
                .bind(&command.host_user_id)
                .fetch_optional(&self.pool)
                .await?;
        let Some(mode) = mode else {
            return Err(AppError::Forbidden(
                "host does not own this match".to_owned(),
            ));
        };
        let question_type = resolve_question_type(&mode, command.question_type);
        let next_position: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(position), 0) + 1 FROM match_questions WHERE match_id = ?",
        )
        .bind(&command.match_id)
        .fetch_one(&self.pool)
        .await?;
        sqlx::query(
            "INSERT INTO match_questions (id, match_id, source_question_id, position, question_type, prompt_snapshot, expected_answer_snapshot, options_snapshot, points)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&command.match_id)
        .bind(&command.source_question_id)
        .bind(next_position)
        .bind(question_type.as_code())
        .bind(&command.prompt)
        .bind(&command.expected_answer)
        .bind(serialize_options(&command.options))
        .bind(command.points)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_match_question(
        &self,
        command: &UpdateMatchQuestionCommand,
    ) -> Result<(), AppError> {
        let mode: Option<String> =
            sqlx::query_scalar("SELECT mode FROM matches WHERE id = ? AND host_user_id = ?")
                .bind(&command.match_id)
                .bind(&command.host_user_id)
                .fetch_optional(&self.pool)
                .await?;
        let Some(mode) = mode else {
            return Err(AppError::Conflict("question cannot be updated".to_owned()));
        };
        let question_type = resolve_question_type(&mode, command.question_type);
        let result = sqlx::query(
            "UPDATE match_questions mq
             JOIN matches m ON m.id = mq.match_id
             SET mq.question_type = ?, mq.prompt_snapshot = ?, mq.expected_answer_snapshot = ?, mq.options_snapshot = ?, mq.points = ?
             WHERE mq.id = ? AND mq.match_id = ? AND m.host_user_id = ? AND mq.used_at IS NULL",
        )
        .bind(question_type.as_code())
        .bind(&command.prompt)
        .bind(&command.expected_answer)
        .bind(serialize_options(&command.options))
        .bind(command.points)
        .bind(&command.question_id)
        .bind(&command.match_id)
        .bind(&command.host_user_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::Conflict("question cannot be updated".to_owned()));
        }
        Ok(())
    }

    async fn delete_match_question(
        &self,
        command: &DeleteMatchQuestionCommand,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            "DELETE mq FROM match_questions mq
             JOIN matches m ON m.id = mq.match_id
             WHERE mq.id = ? AND mq.match_id = ? AND m.host_user_id = ? AND mq.used_at IS NULL",
        )
        .bind(&command.question_id)
        .bind(&command.match_id)
        .bind(&command.host_user_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::Conflict("question cannot be deleted".to_owned()));
        }
        Ok(())
    }

    async fn import_match_questions(
        &self,
        command: &ImportMatchQuestionsCommand,
        rows: &[ParsedCsvQuestion],
    ) -> Result<(), AppError> {
        if rows.is_empty() {
            return Ok(());
        }
        let mut tx = self.pool.begin().await?;
        let mode: Option<String> =
            sqlx::query_scalar("SELECT mode FROM matches WHERE id = ? AND host_user_id = ?")
                .bind(&command.match_id)
                .bind(&command.host_user_id)
                .fetch_optional(&mut *tx)
                .await?;
        let Some(mode) = mode else {
            return Err(AppError::Forbidden(
                "host does not own this match".to_owned(),
            ));
        };
        let mut position: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(position), 0) FROM match_questions WHERE match_id = ?",
        )
        .bind(&command.match_id)
        .fetch_one(&mut *tx)
        .await?;

        for row in rows {
            position += 1;
            let question_type = resolve_question_type(&mode, Some(row.question_type));
            let options = if question_type == QuestionType::MultipleChoice {
                serialize_options(&row.options)
            } else {
                None
            };
            sqlx::query(
                "INSERT INTO match_questions (id, match_id, position, question_type, prompt_snapshot, expected_answer_snapshot, options_snapshot, points)
                 VALUES (UUID(), ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&command.match_id)
            .bind(position)
            .bind(question_type.as_code())
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

    async fn player_answer(
        &self,
        command: &PlayerAnswerCommand,
    ) -> Result<PlayerAnswerOutcome, AppError> {
        let participant_id: Option<String> = sqlx::query_scalar(
            "SELECT participant_id
             FROM match_participant_tokens
             WHERE match_id = ? AND token = ? AND revoked_at IS NULL
             LIMIT 1",
        )
        .bind(&command.match_id)
        .bind(&command.participant_token)
        .fetch_optional(&self.pool)
        .await?;
        let participant_id = participant_id
            .ok_or_else(|| AppError::Unauthorized("participant token is invalid".to_owned()))?;

        let routing: Option<(String, Option<String>)> = sqlx::query_as(
            "SELECT m.mode, mq.question_type
             FROM match_rounds mr
             JOIN matches m ON m.id = mr.match_id
             LEFT JOIN match_questions mq ON mq.id = mr.match_question_id
             WHERE mr.id = ? AND mr.match_id = ?",
        )
        .bind(&command.round_id)
        .bind(&command.match_id)
        .fetch_optional(&self.pool)
        .await?;
        let (mode, question_type) =
            routing.ok_or_else(|| AppError::NotFound("round was not found".to_owned()))?;
        let mode = MatchMode::from_code(&mode).unwrap_or(MatchMode::Buzzer);

        if mode == MatchMode::Quiz {
            let question_type = question_type
                .as_deref()
                .and_then(QuestionType::from_code)
                .unwrap_or(QuestionType::FreeText);
            self.auto_answer(command, &participant_id, question_type).await
        } else {
            let outcome = self
                .buzz(
                    &uuid::Uuid::new_v4().to_string(),
                    &BuzzCommand {
                        match_id: command.match_id.clone(),
                        round_id: command.round_id.clone(),
                        participant_id,
                        presence_session_id: None,
                    },
                )
                .await?;
            Ok(PlayerAnswerOutcome {
                accepted: matches!(outcome, BuzzOutcome::Accepted),
                correct: None,
                early: matches!(outcome, BuzzOutcome::Early),
            })
        }
    }

    async fn buzz(
        &self,
        buzzer_event_id: &str,
        command: &BuzzCommand,
    ) -> Result<BuzzOutcome, AppError> {
        let mut tx = self.pool.begin().await?;

        let eliminated: Option<chrono::NaiveDateTime> = sqlx::query_scalar(
            "SELECT eliminated_at FROM match_participants WHERE id = ? AND match_id = ?",
        )
        .bind(&command.participant_id)
        .bind(&command.match_id)
        .fetch_optional(&mut *tx)
        .await?
        .flatten();
        if eliminated.is_some() {
            return Err(AppError::Conflict(
                "participant has been eliminated".to_owned(),
            ));
        }

        let already_early: Option<String> = sqlx::query_scalar(
            "SELECT id FROM buzzer_events
             WHERE match_round_id = ? AND participant_id = ? AND decision = 'early'
             LIMIT 1",
        )
        .bind(&command.round_id)
        .bind(&command.participant_id)
        .fetch_optional(&mut *tx)
        .await?;
        if already_early.is_some() {
            return Err(AppError::Conflict(
                "participant has been eliminated from this round".to_owned(),
            ));
        }

        let timing: Option<(i64, Option<i32>, i32)> = sqlx::query_as(
            "SELECT TIMESTAMPDIFF(SECOND, mr.opened_at, CURRENT_TIMESTAMP(6)) AS elapsed,
                    ms.time_per_round_seconds,
                    ms.buzzer_arm_delay_seconds
             FROM match_rounds mr
             JOIN match_settings ms ON ms.match_id = mr.match_id
             WHERE mr.id = ? AND mr.match_id = ?",
        )
        .bind(&command.round_id)
        .bind(&command.match_id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some((elapsed, time_limit, arm_delay)) = timing {
            if arm_delay > 0 && elapsed < arm_delay as i64 {
                sqlx::query(
                    "INSERT INTO buzzer_events (id, match_id, match_round_id, participant_id, presence_session_id, decision) VALUES (?, ?, ?, ?, ?, 'early')",
                )
                .bind(buzzer_event_id)
                .bind(&command.match_id)
                .bind(&command.round_id)
                .bind(&command.participant_id)
                .bind(&command.presence_session_id)
                .execute(&mut *tx)
                .await?;
                tx.commit().await?;
                return Ok(BuzzOutcome::Early);
            }
            if let Some(limit) = time_limit {
                if elapsed > limit as i64 {
                    return Err(AppError::Conflict(
                        "time for this question has expired".to_owned(),
                    ));
                }
            }
        }

        let result = sqlx::query(
            "UPDATE match_rounds
             SET selected_participant_id = ?
             WHERE id = ? AND match_id = ? AND status = 'open' AND selected_participant_id IS NULL",
        )
        .bind(&command.participant_id)
        .bind(&command.round_id)
        .bind(&command.match_id)
        .execute(&mut *tx)
        .await?;
        let accepted = result.rows_affected() == 1;
        let decision = if accepted { "accepted" } else { "duplicate" };
        sqlx::query(
            "INSERT INTO buzzer_events (id, match_id, match_round_id, participant_id, presence_session_id, decision) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(buzzer_event_id)
        .bind(&command.match_id)
        .bind(&command.round_id)
        .bind(&command.participant_id)
        .bind(&command.presence_session_id)
        .bind(decision)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(if accepted {
            BuzzOutcome::Accepted
        } else {
            BuzzOutcome::Duplicate
        })
    }

    async fn reveal_answer(&self, command: &RevealAnswerCommand) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE match_rounds mr
             JOIN matches m ON m.id = mr.match_id
             SET mr.status = 'answer_revealed', mr.answer_revealed_at = CURRENT_TIMESTAMP(6)
             WHERE mr.id = ? AND mr.match_id = ? AND m.host_user_id = ? AND mr.status = 'open'",
        )
        .bind(&command.round_id)
        .bind(&command.match_id)
        .bind(&command.host_user_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::conflict_code(
                "match_no_open_round",
                "no open round to reveal",
            ));
        }
        Ok(())
    }

    async fn register_answer(
        &self,
        answer_id: &str,
        command: &RegisterAnswerCommand,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        if let Some(participant_id) = &command.participant_id {
            let points: i32 = if command.result == "correct" {
                sqlx::query_scalar(
                    "SELECT mq.points
                     FROM match_rounds mr
                     JOIN match_questions mq ON mq.id = mr.match_question_id
                     WHERE mr.id = ? AND mr.match_id = ?",
                )
                .bind(&command.round_id)
                .bind(&command.match_id)
                .fetch_optional(&mut *tx)
                .await?
                .unwrap_or_default()
            } else {
                0
            };
            sqlx::query(
                "INSERT INTO match_answers (id, match_id, match_round_id, participant_id, result, points_applied) VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(answer_id)
            .bind(&command.match_id)
            .bind(&command.round_id)
            .bind(participant_id)
            .bind(&command.result)
            .bind(points)
            .execute(&mut *tx)
            .await?;

            let current: Option<i32> = sqlx::query_scalar(
                "SELECT current_score FROM match_participants WHERE id = ? AND match_id = ? FOR UPDATE",
            )
            .bind(participant_id)
            .bind(&command.match_id)
            .fetch_optional(&mut *tx)
            .await?;
            let score_after = current.unwrap_or_default() + points;
            sqlx::query("UPDATE match_participants SET current_score = ? WHERE id = ?")
                .bind(score_after)
                .bind(participant_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query(
                "INSERT INTO score_events (id, match_id, participant_id, match_round_id, actor_user_id, event_type, delta, score_after) VALUES (UUID(), ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&command.match_id)
            .bind(participant_id)
            .bind(&command.round_id)
            .bind(&command.actor_user_id)
            .bind(if command.result == "correct" { "correct_answer" } else { "incorrect_answer" })
            .bind(points)
            .bind(score_after)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query("UPDATE match_rounds SET status = 'closed', closed_at = CURRENT_TIMESTAMP(6) WHERE id = ?")
            .bind(&command.round_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE matches SET current_round_id = NULL WHERE id = ? AND current_round_id = ?",
        )
        .bind(&command.match_id)
        .bind(&command.round_id)
        .execute(&mut *tx)
        .await?;

        let mode = fetch_scoring_mode(&mut tx, &command.match_id).await?;
        if mode == "elimination" {
            if command.result != "correct" {
                if let Some(participant_id) = &command.participant_id {
                    deduct_life_and_maybe_eliminate(
                        &mut tx,
                        &command.match_id,
                        participant_id,
                        &command.round_id,
                        Some(command.actor_user_id.as_str()),
                    )
                    .await?;
                }
            }
            maybe_finish_elimination(&mut tx, &command.match_id, &mode).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn record_score_adjustment(
        &self,
        event_id: &str,
        command: &AdjustScoreCommand,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;
        let current: Option<i32> = sqlx::query_scalar(
            "SELECT current_score FROM match_participants WHERE id = ? AND match_id = ? FOR UPDATE",
        )
        .bind(&command.participant_id)
        .bind(&command.match_id)
        .fetch_optional(&mut *tx)
        .await?;
        let score_after = current.unwrap_or_default() + command.delta;

        sqlx::query("UPDATE match_participants SET current_score = ? WHERE id = ?")
            .bind(score_after)
            .bind(&command.participant_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query(
            "INSERT INTO score_events (id, match_id, participant_id, actor_user_id, event_type, delta, score_after, reason) VALUES (?, ?, ?, ?, 'manual_adjustment', ?, ?, ?)",
        )
        .bind(event_id)
        .bind(&command.match_id)
        .bind(&command.participant_id)
        .bind(&command.actor_user_id)
        .bind(command.delta)
        .bind(score_after)
        .bind(&command.reason)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn end(&self, command: &EndMatchCommand) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;
        let mode = fetch_scoring_mode(&mut tx, &command.match_id).await?;
        let winner_id = determine_winner(&mut tx, &command.match_id, &mode).await?;
        let result = sqlx::query(
            "UPDATE matches SET status = 'finished', ended_at = CURRENT_TIMESTAMP(6), current_round_id = NULL, winner_participant_id = ? WHERE id = ? AND host_user_id = ? AND status IN ('waiting', 'in_progress')",
        )
        .bind(winner_id)
        .bind(&command.match_id)
        .bind(&command.host_user_id)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            let is_host: Option<i64> =
                sqlx::query_scalar("SELECT (host_user_id = ?) FROM matches WHERE id = ?")
                    .bind(&command.host_user_id)
                    .bind(&command.match_id)
                    .fetch_optional(&mut *tx)
                    .await?;
            return Err(match is_host {
                None => AppError::not_found_code("match_not_found", "match was not found"),
                Some(0) => {
                    AppError::forbidden_code("match_not_host", "only the host can control this match")
                }
                _ => AppError::conflict_code("match_already_finished", "match has already ended"),
            });
        }
        sqlx::query(
            "UPDATE room_sessions SET status = 'closed', closed_at = CURRENT_TIMESTAMP(6)
             WHERE match_id = ? AND status IN ('waiting', 'live')",
        )
        .bind(&command.match_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn reset(&self, command: &ResetMatchCommand) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query(
            "UPDATE matches SET status = 'waiting', current_round_id = NULL, winner_participant_id = NULL, started_at = NULL, ended_at = NULL WHERE id = ? AND host_user_id = ?",
        )
        .bind(&command.match_id)
        .bind(&command.host_user_id)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            let is_host: Option<i64> =
                sqlx::query_scalar("SELECT (host_user_id = ?) FROM matches WHERE id = ?")
                    .bind(&command.host_user_id)
                    .bind(&command.match_id)
                    .fetch_optional(&mut *tx)
                    .await?;
            return Err(match is_host {
                None => AppError::not_found_code("match_not_found", "match was not found"),
                Some(0) => {
                    AppError::forbidden_code("match_not_host", "only the host can control this match")
                }
                _ => AppError::conflict_code("match_cannot_reset", "match cannot be reset"),
            });
        }
        sqlx::query("UPDATE match_questions SET used_at = NULL WHERE match_id = ?")
            .bind(&command.match_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE match_participants SET current_score = 0, lives_remaining = NULL, eliminated_at = NULL WHERE match_id = ?",
        )
        .bind(&command.match_id)
        .execute(&mut *tx)
        .await?;
        sqlx::query("DELETE FROM score_events WHERE match_id = ?")
            .bind(&command.match_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM match_answers WHERE match_id = ?")
            .bind(&command.match_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM match_rounds WHERE match_id = ?")
            .bind(&command.match_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE room_sessions SET status = 'waiting', closed_at = NULL WHERE match_id = ?",
        )
        .bind(&command.match_id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Match>, AppError> {
        let row = sqlx::query(
            "SELECT id, mode, host_user_id, name, status, current_round_id, winner_participant_id, created_at, started_at, ended_at, updated_at FROM matches WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(map_match).transpose()
    }

    async fn list_by_owner(&self, owner_user_id: &str) -> Result<Vec<Match>, AppError> {
        let rows = sqlx::query(
            "SELECT id, mode, host_user_id, name, status, current_round_id, winner_participant_id, created_at, started_at, ended_at, updated_at FROM matches WHERE host_user_id = ? ORDER BY created_at DESC",
        )
        .bind(owner_user_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_match).collect()
    }

    async fn list_summaries_by_owner(
        &self,
        owner_user_id: &str,
    ) -> Result<Vec<MatchSummaryState>, AppError> {
        let rows = sqlx::query(
            "SELECT m.id, m.mode, ms.scoring_mode, m.host_user_id, m.name,
                    m.status, m.current_round_id, m.winner_participant_id, m.created_at,
                    m.started_at, m.ended_at, m.updated_at,
                    COUNT(DISTINCT mq.id) AS question_count,
                    COUNT(DISTINCT mp.id) AS participant_count,
                    COUNT(DISTINCT mr2.id) AS rounds_played,
                    mr.round_number AS current_round_number,
                    mr.status AS current_round_status,
                    CASE
                        WHEN m.started_at IS NULL THEN NULL
                        ELSE TIMESTAMPDIFF(SECOND, m.started_at, COALESCE(m.ended_at, CURRENT_TIMESTAMP(6)))
                    END AS duration_seconds,
                    winner.display_name_snapshot AS winner_name,
                    winner.current_score AS winner_score
             FROM matches m
             JOIN match_settings ms ON ms.match_id = m.id
             LEFT JOIN match_questions mq ON mq.match_id = m.id
             LEFT JOIN match_participants mp ON mp.match_id = m.id AND mp.left_at IS NULL
             LEFT JOIN match_rounds mr2 ON mr2.match_id = m.id
             LEFT JOIN match_rounds mr ON mr.id = m.current_round_id
             LEFT JOIN match_participants winner ON winner.id = m.winner_participant_id
             WHERE m.host_user_id = ?
             GROUP BY m.id, m.mode, ms.scoring_mode, m.host_user_id, m.name,
                    m.status, m.current_round_id, m.winner_participant_id, m.created_at,
                    m.started_at, m.ended_at, m.updated_at, mr.round_number,
                    mr.status, winner.display_name_snapshot, winner.current_score
             ORDER BY m.created_at DESC",
        )
        .bind(owner_user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut summaries = Vec::with_capacity(rows.len());
        for row in rows {
            let match_id: String = row.try_get("id")?;
            let top_scores = load_top_scores(&self.pool, &match_id).await?;
            summaries.push(map_match_summary(row, top_scores)?);
        }
        Ok(summaries)
    }

    async fn list_participants(&self, match_id: &str) -> Result<Vec<MatchParticipant>, AppError> {
        let rows = sqlx::query(
            "SELECT id, match_id, display_name_snapshot, join_source, current_score, lives_remaining, eliminated_at, joined_at, left_at FROM match_participants WHERE match_id = ? ORDER BY current_score DESC, joined_at",
        )
        .bind(match_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_participant).collect()
    }

    async fn list_score_events(&self, match_id: &str) -> Result<Vec<ScoreEvent>, AppError> {
        let rows = sqlx::query(
            "SELECT id, match_id, participant_id, match_round_id, actor_user_id, event_type, delta, created_at FROM score_events WHERE match_id = ? ORDER BY created_at",
        )
        .bind(match_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_score_event).collect()
    }

    async fn list_round_details(
        &self,
        match_id: &str,
    ) -> Result<Vec<MatchRoundDetailState>, AppError> {
        let rows = sqlx::query(
            "SELECT mr.round_number, mr.status,
                    mq.prompt_snapshot, mq.expected_answer_snapshot,
                    ma.participant_id, ma.result, ma.points_applied,
                    mp.display_name_snapshot
             FROM match_rounds mr
             LEFT JOIN match_questions mq ON mq.id = mr.match_question_id
             LEFT JOIN match_answers ma ON ma.match_round_id = mr.id
             LEFT JOIN match_participants mp ON mp.id = ma.participant_id
             WHERE mr.match_id = ?
             ORDER BY mr.round_number, ma.answered_at",
        )
        .bind(match_id)
        .fetch_all(&self.pool)
        .await?;

        let mut details: Vec<MatchRoundDetailState> = Vec::new();
        for row in rows {
            let round_number: i32 = row.try_get("round_number")?;
            let round_number = round_number.max(0) as u32;
            if details.last().map(|d| d.round_number) != Some(round_number) {
                details.push(MatchRoundDetailState {
                    round_number,
                    status: row.try_get("status")?,
                    prompt: row.try_get("prompt_snapshot")?,
                    expected_answer: row.try_get("expected_answer_snapshot")?,
                    answers: Vec::new(),
                });
            }
            let result: Option<String> = row.try_get("result")?;
            if let Some(result) = result {
                let points_applied: i32 = row.try_get("points_applied")?;
                if let Some(detail) = details.last_mut() {
                    detail.answers.push(MatchRoundAnswerState {
                        participant_id: row.try_get("participant_id")?,
                        display_name: row.try_get("display_name_snapshot")?,
                        result,
                        points_applied,
                    });
                }
            }
        }
        Ok(details)
    }

    async fn get_state(&self, match_id: &str) -> Result<Option<MatchState>, AppError> {
        let Some(match_entity) = self.find_by_id(match_id).await? else {
            return Ok(None);
        };
        let settings = load_settings(&self.pool, match_id).await?;
        let questions = load_questions(&self.pool, match_id).await?;
        let participants = self.list_participants(match_id).await?;
        let current_round = if let Some(round_id) = &match_entity.current_round_id {
            load_round(&self.pool, round_id.as_str()).await?
        } else {
            None
        };
        let room = load_room(&self.pool, match_id).await?;
        let winner = match_entity
            .winner_participant_id
            .as_ref()
            .and_then(|winner_id| {
                participants
                    .iter()
                    .find(|participant| participant.id.as_str() == winner_id.as_str())
                    .cloned()
            });
        Ok(Some(MatchState {
            match_entity,
            settings,
            questions,
            participants,
            current_round,
            room,
            winner,
        }))
    }

}

/// Classifies why `draw_round`'s pick query found no drawable question, into a
/// specific coded error. The `Ok` value is the error to return to the caller;
/// `Err` is a real database failure.
async fn diagnose_draw_failure(
    conn: &mut sqlx::mysql::MySqlConnection,
    match_id: &str,
    host_user_id: &str,
) -> Result<AppError, AppError> {
    let row: Option<(i64, String, i64, i64, i64)> = sqlx::query_as(
        "SELECT (m.host_user_id = ?) AS is_host, m.status,
                (m.current_round_id IS NOT NULL) AS has_open_round,
                (SELECT COUNT(*) FROM match_questions q WHERE q.match_id = m.id) AS total_q,
                (SELECT COUNT(*) FROM match_questions q WHERE q.match_id = m.id AND q.used_at IS NULL) AS unused_q
         FROM matches m WHERE m.id = ?",
    )
    .bind(host_user_id)
    .bind(match_id)
    .fetch_optional(&mut *conn)
    .await?;

    let error = match row {
        None => AppError::not_found_code("match_not_found", "match was not found"),
        Some((is_host, ..)) if is_host == 0 => {
            AppError::forbidden_code("match_not_host", "only the host can control this match")
        }
        Some((_, ref status, ..)) if status == "finished" || status == "cancelled" => {
            AppError::conflict_code("match_finished", "match has already ended")
        }
        Some((_, ref status, ..)) if status != "in_progress" => {
            AppError::conflict_code("match_not_started", "match has not started")
        }
        Some((_, _, has_open_round, _, _)) if has_open_round != 0 => {
            AppError::conflict_code("match_round_open", "an open round already exists")
        }
        Some((_, _, _, total_q, _)) if total_q == 0 => {
            AppError::conflict_code("match_no_questions", "match has no questions")
        }
        Some((_, _, _, _, unused_q)) if unused_q == 0 => {
            AppError::conflict_code("match_all_questions_used", "all questions have been used")
        }
        Some(_) => AppError::conflict_code("match_cannot_draw", "no drawable question"),
    };
    Ok(error)
}

/// Classifies why `start`'s conditional UPDATE matched no row, into a specific
/// coded error. `Ok` is the error to return; `Err` is a real database failure.
async fn diagnose_start_failure(
    conn: &mut sqlx::mysql::MySqlConnection,
    match_id: &str,
    host_user_id: &str,
) -> Result<AppError, AppError> {
    let row: Option<(i64, String, i64, i64)> = sqlx::query_as(
        "SELECT (m.host_user_id = ?) AS is_host, m.status,
                (SELECT COUNT(*) FROM match_participants p WHERE p.match_id = m.id) AS parts,
                (SELECT COUNT(*) FROM match_questions q WHERE q.match_id = m.id) AS qs
         FROM matches m WHERE m.id = ?",
    )
    .bind(host_user_id)
    .bind(match_id)
    .fetch_optional(&mut *conn)
    .await?;

    let error = match row {
        None => AppError::not_found_code("match_not_found", "match was not found"),
        Some((is_host, ..)) if is_host == 0 => {
            AppError::forbidden_code("match_not_host", "only the host can control this match")
        }
        Some((_, ref status, ..)) if status != "waiting" => {
            AppError::conflict_code("match_already_started", "match has already started")
        }
        Some((_, _, parts, _)) if parts == 0 => {
            AppError::conflict_code("match_no_participants", "match has no participants")
        }
        Some((_, _, _, qs)) if qs == 0 => {
            AppError::conflict_code("match_no_questions", "match has no questions")
        }
        Some(_) => AppError::conflict_code("match_cannot_start", "match cannot be started"),
    };
    Ok(error)
}

/// Classifies why creating a match from a template found no usable template,
/// into a specific coded error. `Ok` is the error to return; `Err` is a real
/// database failure.
async fn diagnose_template_failure(
    conn: &mut sqlx::mysql::MySqlConnection,
    template_id: &str,
    host_user_id: &str,
) -> Result<AppError, AppError> {
    let owner: Option<i64> =
        sqlx::query_scalar("SELECT (host_user_id = ?) FROM match_templates WHERE id = ?")
            .bind(host_user_id)
            .bind(template_id)
            .fetch_optional(&mut *conn)
            .await?;
    Ok(match owner {
        None => AppError::not_found_code("template_not_found", "match template was not found"),
        Some(0) => {
            AppError::forbidden_code("template_not_owner", "you do not own this match template")
        }
        _ => AppError::conflict_code("template_cannot_create", "match cannot be created"),
    })
}

async fn fetch_scoring_mode(
    conn: &mut sqlx::mysql::MySqlConnection,
    match_id: &str,
) -> Result<String, AppError> {
    let scoring_mode: Option<String> =
        sqlx::query_scalar("SELECT scoring_mode FROM match_settings WHERE match_id = ?")
            .bind(match_id)
            .fetch_optional(&mut *conn)
            .await?;
    Ok(scoring_mode.unwrap_or_else(|| "classic".to_owned()))
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

fn resolve_question_type(mode: &str, provided: Option<QuestionType>) -> QuestionType {
    if MatchMode::from_code(mode) == Some(MatchMode::Buzzer) {
        QuestionType::Buzzer
    } else {
        provided.unwrap_or(QuestionType::FreeText)
    }
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

async fn seed_initial_points(
    conn: &mut sqlx::mysql::MySqlConnection,
    match_id: &str,
    points: i32,
) -> Result<(), AppError> {
    sqlx::query("UPDATE match_participants SET current_score = ? WHERE match_id = ?")
        .bind(points)
        .bind(match_id)
        .execute(&mut *conn)
        .await?;
    let ids: Vec<String> =
        sqlx::query_scalar("SELECT id FROM match_participants WHERE match_id = ?")
            .bind(match_id)
            .fetch_all(&mut *conn)
            .await?;
    for id in ids {
        sqlx::query(
            "INSERT INTO score_events (id, match_id, participant_id, event_type, delta, score_after) VALUES (UUID(), ?, ?, 'initial_points', ?, ?)",
        )
        .bind(match_id)
        .bind(&id)
        .bind(points)
        .bind(points)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

async fn seed_lives(
    conn: &mut sqlx::mysql::MySqlConnection,
    match_id: &str,
    lives: i32,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE match_participants SET lives_remaining = ?, current_score = 0, eliminated_at = NULL WHERE match_id = ?",
    )
    .bind(lives)
    .bind(match_id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

async fn determine_winner(
    conn: &mut sqlx::mysql::MySqlConnection,
    match_id: &str,
    mode: &str,
) -> Result<Option<String>, AppError> {
    if mode == "elimination" {
        let survivor: Option<String> = sqlx::query_scalar(
            "SELECT id FROM match_participants WHERE match_id = ? AND eliminated_at IS NULL AND left_at IS NULL ORDER BY current_score DESC, joined_at LIMIT 1",
        )
        .bind(match_id)
        .fetch_optional(&mut *conn)
        .await?;
        if survivor.is_some() {
            return Ok(survivor);
        }
    }
    let winner: Option<String> = sqlx::query_scalar(
        "SELECT id FROM match_participants WHERE match_id = ? ORDER BY current_score DESC, joined_at LIMIT 1",
    )
    .bind(match_id)
    .fetch_optional(&mut *conn)
    .await?;
    Ok(winner)
}

async fn deduct_life_and_maybe_eliminate(
    conn: &mut sqlx::mysql::MySqlConnection,
    match_id: &str,
    participant_id: &str,
    round_id: &str,
    actor_user_id: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE match_participants SET lives_remaining = GREATEST(COALESCE(lives_remaining, 0) - 1, 0) WHERE id = ? AND match_id = ?",
    )
    .bind(participant_id)
    .bind(match_id)
    .execute(&mut *conn)
    .await?;
    let lives: i32 = sqlx::query_scalar(
        "SELECT COALESCE(lives_remaining, 0) FROM match_participants WHERE id = ?",
    )
    .bind(participant_id)
    .fetch_one(&mut *conn)
    .await?;
    if lives <= 0 {
        let score_after: i32 =
            sqlx::query_scalar("SELECT current_score FROM match_participants WHERE id = ?")
                .bind(participant_id)
                .fetch_one(&mut *conn)
                .await?;
        sqlx::query(
            "UPDATE match_participants SET eliminated_at = CURRENT_TIMESTAMP(6) WHERE id = ? AND eliminated_at IS NULL",
        )
        .bind(participant_id)
        .execute(&mut *conn)
        .await?;
        sqlx::query(
            "INSERT INTO score_events (id, match_id, participant_id, match_round_id, actor_user_id, event_type, delta, score_after, reason) VALUES (UUID(), ?, ?, ?, ?, 'penalty', 0, ?, 'eliminated')",
        )
        .bind(match_id)
        .bind(participant_id)
        .bind(round_id)
        .bind(actor_user_id)
        .bind(score_after)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

async fn maybe_finish_elimination(
    conn: &mut sqlx::mysql::MySqlConnection,
    match_id: &str,
    mode: &str,
) -> Result<(), AppError> {
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM match_participants WHERE match_id = ? AND left_at IS NULL",
    )
    .bind(match_id)
    .fetch_one(&mut *conn)
    .await?;
    let survivors: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM match_participants WHERE match_id = ? AND left_at IS NULL AND eliminated_at IS NULL",
    )
    .bind(match_id)
    .fetch_one(&mut *conn)
    .await?;
    if total >= 2 && survivors <= 1 {
        let winner = determine_winner(conn, match_id, mode).await?;
        auto_finalize(conn, match_id, winner).await?;
    }
    Ok(())
}

async fn auto_finalize(
    conn: &mut sqlx::mysql::MySqlConnection,
    match_id: &str,
    winner_id: Option<String>,
) -> Result<(), AppError> {
    sqlx::query(
        "UPDATE matches SET status = 'finished', ended_at = CURRENT_TIMESTAMP(6), current_round_id = NULL, winner_participant_id = ? WHERE id = ? AND status = 'in_progress'",
    )
    .bind(&winner_id)
    .bind(match_id)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "UPDATE room_sessions SET status = 'closed', closed_at = CURRENT_TIMESTAMP(6) WHERE match_id = ? AND status IN ('waiting', 'live')",
    )
    .bind(match_id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

async fn load_settings(pool: &MySqlPool, match_id: &str) -> Result<MatchSettingsState, AppError> {
    let row = sqlx::query(
        "SELECT ms.max_rounds, ms.time_per_round_seconds, ms.initial_points, ms.remote_enabled,
                ms.capacity, ms.buzzer_arm_delay_seconds, ms.scoring_mode, ms.ranking_display, m.mode
         FROM match_settings ms
         JOIN matches m ON m.id = ms.match_id
         WHERE ms.match_id = ?",
    )
    .bind(match_id)
    .fetch_one(pool)
    .await?;
    Ok(MatchSettingsState {
        mode: row.try_get("mode")?,
        scoring_mode: row.try_get("scoring_mode")?,
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
        buzzer_arm_delay_seconds: row
            .try_get::<i32, _>("buzzer_arm_delay_seconds")?
            .max(0) as u32,
        ranking_display: row.try_get("ranking_display")?,
    })
}

async fn load_questions(
    pool: &MySqlPool,
    match_id: &str,
) -> Result<Vec<MatchQuestionState>, AppError> {
    let rows = sqlx::query(
        "SELECT id, source_question_id, position, question_type, prompt_snapshot, expected_answer_snapshot, options_snapshot, points, used_at
         FROM match_questions WHERE match_id = ? ORDER BY position",
    )
    .bind(match_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(map_question_state).collect()
}

async fn load_round(pool: &MySqlPool, round_id: &str) -> Result<Option<MatchRoundState>, AppError> {
    let row = sqlx::query(
        "SELECT mr.id, mr.match_question_id, mr.round_number, mr.status, mr.selected_participant_id,
                mr.answer_revealed_at, mq.id AS question_id, mq.position, mq.question_type, mq.prompt_snapshot,
                mq.expected_answer_snapshot, mq.options_snapshot, mq.points, mq.used_at,
                CASE
                    WHEN mr.status = 'open' AND ms.time_per_round_seconds IS NOT NULL
                    THEN GREATEST(ms.time_per_round_seconds - TIMESTAMPDIFF(SECOND, mr.opened_at, CURRENT_TIMESTAMP(6)), 0)
                    ELSE NULL
                END AS time_remaining_seconds
         FROM match_rounds mr
         JOIN match_settings ms ON ms.match_id = mr.match_id
         LEFT JOIN match_questions mq ON mq.id = mr.match_question_id
         WHERE mr.id = ?",
    )
    .bind(round_id)
    .fetch_optional(pool)
    .await?;
    let Some(mut state) = row.map(map_round_state).transpose()? else {
        return Ok(None);
    };
    state.early_participant_ids = sqlx::query_scalar(
        "SELECT DISTINCT participant_id FROM buzzer_events
         WHERE match_round_id = ? AND decision = 'early'",
    )
    .bind(round_id)
    .fetch_all(pool)
    .await?;
    Ok(Some(state))
}

async fn load_room(pool: &MySqlPool, match_id: &str) -> Result<Option<LiveRoomState>, AppError> {
    let row = sqlx::query(
        "SELECT rs.id, rs.match_id, m.name AS match_name, m.mode,
                rs.pin, rs.status, rs.capacity, u.display_name AS host_name,
                COUNT(mp.id) AS participants,
                TIMESTAMPDIFF(MINUTE, rs.opened_at, CURRENT_TIMESTAMP(6)) AS started_minutes_ago
         FROM room_sessions rs
         JOIN matches m ON m.id = rs.match_id
         JOIN users u ON u.id = m.host_user_id
         LEFT JOIN match_participants mp ON mp.match_id = m.id AND mp.left_at IS NULL
         WHERE rs.match_id = ?
         GROUP BY rs.id, rs.match_id, m.name, m.mode, rs.pin, rs.status, rs.capacity, u.display_name, rs.opened_at",
    )
    .bind(match_id)
    .fetch_optional(pool)
    .await?;
    row.map(map_live_room).transpose()
}

async fn load_top_scores(
    pool: &MySqlPool,
    match_id: &str,
) -> Result<Vec<MatchTopScoreState>, AppError> {
    let rows = sqlx::query(
        "SELECT id, display_name_snapshot, current_score
         FROM match_participants
         WHERE match_id = ? AND left_at IS NULL
         ORDER BY current_score DESC, joined_at
         LIMIT 5",
    )
    .bind(match_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter().map(map_top_score).collect()
}

fn map_match(row: sqlx::mysql::MySqlRow) -> Result<Match, AppError> {
    let mode: String = row.try_get("mode")?;
    Ok(Match {
        id: mapper::id(row.try_get("id")?)?,
        mode: MatchMode::from_code(&mode).unwrap_or(MatchMode::Buzzer),
        host_user_id: mapper::id(row.try_get("host_user_id")?)?,
        name: mapper::non_empty(row.try_get("name")?, "match.name")?,
        status: mapper::match_status(row.try_get("status")?),
        current_round_id: mapper::opt_id(row.try_get("current_round_id")?)?,
        winner_participant_id: mapper::opt_id(row.try_get("winner_participant_id")?)?,
        created_at: mapper::datetime(row.try_get("created_at")?)?,
        started_at: mapper::opt_datetime(row.try_get("started_at")?)?,
        ended_at: mapper::opt_datetime(row.try_get("ended_at")?)?,
        updated_at: mapper::datetime(row.try_get("updated_at")?)?,
    })
}

fn map_match_summary(
    row: sqlx::mysql::MySqlRow,
    top_scores: Vec<MatchTopScoreState>,
) -> Result<MatchSummaryState, AppError> {
    let mode: String = row.try_get("mode")?;
    let scoring_mode: String = row.try_get("scoring_mode")?;
    let mode_label = mode_label(&mode);
    let scoring_mode_label = scoring_mode_label(&scoring_mode);
    let question_count: i64 = row.try_get("question_count")?;
    let participant_count: i64 = row.try_get("participant_count")?;
    let rounds_played: i64 = row.try_get("rounds_played")?;
    let current_round_number: Option<i32> = row.try_get("current_round_number")?;
    let current_round_status: Option<String> = row.try_get("current_round_status")?;
    let duration_seconds: Option<i64> = row.try_get("duration_seconds")?;
    let winner_name: Option<String> = row.try_get("winner_name")?;
    let winner_score: Option<i32> = row.try_get("winner_score")?;
    Ok(MatchSummaryState {
        match_entity: map_match(row)?,
        mode_label,
        scoring_mode_label,
        question_count: question_count.max(0) as u32,
        participant_count: participant_count.max(0) as u32,
        rounds_played: rounds_played.max(0) as u32,
        current_round_number: current_round_number.map(|value| value as u32),
        current_round_status,
        duration_seconds: duration_seconds.map(|value| value.max(0) as u32),
        winner_name,
        winner_score,
        top_scores,
    })
}

fn map_top_score(row: sqlx::mysql::MySqlRow) -> Result<MatchTopScoreState, AppError> {
    Ok(MatchTopScoreState {
        participant_id: row.try_get("id")?,
        display_name: row.try_get("display_name_snapshot")?,
        score: row.try_get("current_score")?,
    })
}

fn map_participant(row: sqlx::mysql::MySqlRow) -> Result<MatchParticipant, AppError> {
    let eliminated_at: Option<chrono::NaiveDateTime> = row.try_get("eliminated_at")?;
    Ok(MatchParticipant {
        id: mapper::id(row.try_get("id")?)?,
        match_id: mapper::id(row.try_get("match_id")?)?,
        display_name_snapshot: mapper::non_empty(
            row.try_get("display_name_snapshot")?,
            "match_participant.display_name_snapshot",
        )?,
        join_source: mapper::join_source(row.try_get("join_source")?),
        current_score: mapper::points(row.try_get("current_score")?),
        lives_remaining: row.try_get("lives_remaining")?,
        eliminated: eliminated_at.is_some(),
        joined_at: mapper::datetime(row.try_get("joined_at")?)?,
        left_at: mapper::opt_datetime(row.try_get("left_at")?)?,
    })
}

fn map_question_state(row: sqlx::mysql::MySqlRow) -> Result<MatchQuestionState, AppError> {
    let used_at: Option<chrono::NaiveDateTime> = row.try_get("used_at")?;
    let position: i32 = row.try_get("position")?;
    let options_raw: Option<String> = row.try_get("options_snapshot")?;
    Ok(MatchQuestionState {
        id: row.try_get("id")?,
        source_question_id: row.try_get("source_question_id")?,
        question_type: row.try_get("question_type")?,
        prompt: row.try_get("prompt_snapshot")?,
        expected_answer: row.try_get("expected_answer_snapshot")?,
        options: deserialize_options(options_raw.as_deref()),
        points: row.try_get("points")?,
        used: used_at.is_some(),
        position: position as u32,
    })
}

fn map_round_state(row: sqlx::mysql::MySqlRow) -> Result<MatchRoundState, AppError> {
    let answer_revealed_at: Option<chrono::NaiveDateTime> = row.try_get("answer_revealed_at")?;
    let question_id: Option<String> = row.try_get("question_id")?;
    let question = if let Some(id) = question_id {
        let used_at: Option<chrono::NaiveDateTime> = row.try_get("used_at")?;
        let position: i32 = row.try_get("position")?;
        let options_raw: Option<String> = row.try_get("options_snapshot")?;
        Some(MatchQuestionState {
            id,
            source_question_id: None,
            question_type: row.try_get("question_type")?,
            prompt: row.try_get("prompt_snapshot")?,
            expected_answer: row.try_get("expected_answer_snapshot")?,
            options: deserialize_options(options_raw.as_deref()),
            points: row.try_get("points")?,
            used: used_at.is_some(),
            position: position as u32,
        })
    } else {
        None
    };
    let round_number: i32 = row.try_get("round_number")?;
    let time_remaining_seconds: Option<i64> = row.try_get("time_remaining_seconds")?;
    Ok(MatchRoundState {
        id: row.try_get("id")?,
        match_question_id: row.try_get("match_question_id")?,
        round_number: round_number as u32,
        status: row.try_get("status")?,
        selected_participant_id: row.try_get("selected_participant_id")?,
        answer_revealed: answer_revealed_at.is_some(),
        time_remaining_seconds: time_remaining_seconds.map(|value| value.max(0) as u32),
        question,
        early_participant_ids: Vec::new(),
    })
}

fn map_live_room(row: sqlx::mysql::MySqlRow) -> Result<LiveRoomState, AppError> {
    let participants: i64 = row.try_get("participants")?;
    let started_minutes_ago: Option<i64> = row.try_get("started_minutes_ago")?;
    let mode: String = row.try_get("mode")?;
    let mode_label = mode_label(&mode);
    Ok(LiveRoomState {
        id: row.try_get("id")?,
        match_id: row.try_get("match_id")?,
        match_name: row.try_get("match_name")?,
        mode,
        mode_label,
        pin: row.try_get("pin")?,
        status: row.try_get("status")?,
        participants: participants.max(0) as u32,
        capacity: row
            .try_get::<Option<i32>, _>("capacity")?
            .map(|value| value as u32),
        host_name: row.try_get("host_name")?,
        started_minutes_ago: started_minutes_ago.unwrap_or_default().max(0) as u32,
    })
}

fn map_score_event(row: sqlx::mysql::MySqlRow) -> Result<ScoreEvent, AppError> {
    Ok(ScoreEvent {
        id: mapper::id(row.try_get("id")?)?,
        match_id: mapper::id(row.try_get("match_id")?)?,
        participant_id: mapper::id(row.try_get("participant_id")?)?,
        round_id: mapper::opt_id(row.try_get("match_round_id")?)?,
        kind: mapper::score_event_kind(row.try_get("event_type")?),
        delta: mapper::points(row.try_get("delta")?),
        recorded_by_user_id: mapper::opt_id(row.try_get("actor_user_id")?)?,
        occurred_at: mapper::datetime(row.try_get("created_at")?)?,
    })
}
