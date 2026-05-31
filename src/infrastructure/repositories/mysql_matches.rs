use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::matches::{
    AddMatchQuestionCommand, AddParticipantCommand, AdjustScoreCommand, BuzzCommand,
    CreateMatchCommand, DeleteMatchQuestionCommand, DrawRoundCommand, EndMatchCommand,
    ImportMatchQuestionsCommand, JoinByPinCommand, JoinMatchByIdCommand, LiveRoomState,
    MatchQuestionState, MatchRepository, MatchRoundState, MatchSettingsState, MatchState,
    ParsedCsvQuestion, PlayerAnswerCommand, RegisterAnswerCommand, ResetMatchCommand,
    RevealAnswerCommand, StartMatchCommand, UpdateMatchQuestionCommand,
};
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
}

#[async_trait]
impl MatchRepository for MySqlMatchRepository {
    async fn create(&self, id: &str, command: &CreateMatchCommand) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO matches (id, organization_id, game_id, game_mode_id, host_user_id, name, status) VALUES (?, ?, ?, ?, ?, ?, 'waiting')",
        )
        .bind(id)
        .bind(&command.organization_id)
        .bind(&command.game_id)
        .bind(&command.game_mode_id)
        .bind(&command.host_user_id)
        .bind(&command.name)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO match_settings (match_id, max_rounds, time_per_round_seconds, initial_points, remote_enabled, capacity) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(command.max_rounds)
        .bind(command.time_per_round_seconds)
        .bind(command.initial_points)
        .bind(command.remote_enabled)
        .bind(command.capacity)
        .execute(&mut *tx)
        .await?;

        for (index, question) in command.questions.iter().enumerate() {
            sqlx::query(
                "INSERT INTO match_questions (id, match_id, source_question_id, position, prompt_snapshot, expected_answer_snapshot, points) VALUES (UUID(), ?, ?, ?, ?, ?, ?)",
            )
            .bind(id)
            .bind(&question.source_question_id)
            .bind(index as i32 + 1)
            .bind(&question.prompt)
            .bind(&question.expected_answer)
            .bind(question.points)
            .execute(&mut *tx)
            .await?;
        }

        if command.remote_enabled {
            sqlx::query(
                "INSERT INTO room_sessions (id, organization_id, match_id, pin, status, capacity)
                 VALUES (UUID(), ?, ?, LPAD(FLOOR(RAND() * 1000000), 6, '0'), 'waiting', ?)",
            )
            .bind(&command.organization_id)
            .bind(id)
            .bind(command.capacity)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn start(&self, command: &StartMatchCommand) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE matches m
             SET m.status = 'in_progress', m.started_at = CURRENT_TIMESTAMP(6)
             WHERE m.id = ? AND m.host_user_id = ? AND m.status = 'waiting'
               AND EXISTS (SELECT 1 FROM match_participants mp WHERE mp.match_id = m.id)
               AND EXISTS (SELECT 1 FROM match_questions mq WHERE mq.match_id = m.id)",
        )
        .bind(&command.match_id)
        .bind(&command.host_user_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::Conflict("match cannot be started".to_owned()));
        }
        sqlx::query(
            "UPDATE room_sessions SET status = 'live' WHERE match_id = ? AND status = 'waiting'",
        )
        .bind(&command.match_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn draw_round(&self, round_id: &str, command: &DrawRoundCommand) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;
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
        let question_id = question_id.ok_or_else(|| {
            AppError::Conflict("no drawable question or an open round already exists".to_owned())
        })?;
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
            player_id: command.player_id.clone(),
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
        sqlx::query(
            "INSERT INTO match_participants (id, match_id, player_id, display_name_snapshot, normalized_name_snapshot, join_source) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&command.match_id)
        .bind(&command.player_id)
        .bind(&command.display_name)
        .bind(mapper::normalized_name(&command.display_name))
        .bind(&command.join_source)
        .execute(&self.pool)
        .await?;
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
            player_id: command.player_id.clone(),
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
        let authorized: Option<String> =
            sqlx::query_scalar("SELECT id FROM matches WHERE id = ? AND host_user_id = ?")
                .bind(&command.match_id)
                .bind(&command.host_user_id)
                .fetch_optional(&self.pool)
                .await?;
        if authorized.is_none() {
            return Err(AppError::Forbidden("host does not own this match".to_owned()));
        }
        let next_position: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(position), 0) + 1 FROM match_questions WHERE match_id = ?",
        )
        .bind(&command.match_id)
        .fetch_one(&self.pool)
        .await?;
        sqlx::query(
            "INSERT INTO match_questions (id, match_id, position, prompt_snapshot, expected_answer_snapshot, points)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&command.match_id)
        .bind(next_position)
        .bind(&command.prompt)
        .bind(&command.expected_answer)
        .bind(command.points)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_match_question(
        &self,
        command: &UpdateMatchQuestionCommand,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE match_questions mq
             JOIN matches m ON m.id = mq.match_id
             SET mq.prompt_snapshot = ?, mq.expected_answer_snapshot = ?, mq.points = ?
             WHERE mq.id = ? AND mq.match_id = ? AND m.host_user_id = ? AND mq.used_at IS NULL",
        )
        .bind(&command.prompt)
        .bind(&command.expected_answer)
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
        let authorized: Option<String> =
            sqlx::query_scalar("SELECT id FROM matches WHERE id = ? AND host_user_id = ?")
                .bind(&command.match_id)
                .bind(&command.host_user_id)
                .fetch_optional(&mut *tx)
                .await?;
        if authorized.is_none() {
            return Err(AppError::Forbidden("host does not own this match".to_owned()));
        }
        let mut position: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(position), 0) FROM match_questions WHERE match_id = ?",
        )
        .bind(&command.match_id)
        .fetch_one(&mut *tx)
        .await?;

        for row in rows {
            position += 1;
            sqlx::query(
                "INSERT INTO match_questions (id, match_id, position, prompt_snapshot, expected_answer_snapshot, points)
                 VALUES (UUID(), ?, ?, ?, ?, ?)",
            )
            .bind(&command.match_id)
            .bind(position)
            .bind(&row.prompt)
            .bind(&row.expected_answer)
            .bind(row.points)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn player_answer(&self, command: &PlayerAnswerCommand) -> Result<bool, AppError> {
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
        self.buzz(
            &uuid::Uuid::new_v4().to_string(),
            &BuzzCommand {
                match_id: command.match_id.clone(),
                round_id: command.round_id.clone(),
                participant_id,
                presence_session_id: None,
            },
        )
        .await
    }

    async fn buzz(&self, buzzer_event_id: &str, command: &BuzzCommand) -> Result<bool, AppError> {
        let mut tx = self.pool.begin().await?;
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
        Ok(accepted)
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
            return Err(AppError::Conflict("answer cannot be revealed".to_owned()));
        }
        Ok(())
    }

    async fn register_answer(
        &self,
        answer_id: &str,
        command: &RegisterAnswerCommand,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;
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
        .bind(&command.participant_id)
        .bind(&command.result)
        .bind(points)
        .execute(&mut *tx)
        .await?;

        if let Some(participant_id) = &command.participant_id {
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
        let winner_id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM match_participants WHERE match_id = ? ORDER BY current_score DESC, joined_at LIMIT 1",
        )
        .bind(&command.match_id)
        .fetch_optional(&self.pool)
        .await?;
        let result = sqlx::query(
            "UPDATE matches SET status = 'finished', ended_at = CURRENT_TIMESTAMP(6), current_round_id = NULL, winner_participant_id = ? WHERE id = ? AND host_user_id = ? AND status IN ('waiting', 'in_progress')",
        )
        .bind(winner_id)
        .bind(&command.match_id)
        .bind(&command.host_user_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::Conflict("match cannot be ended".to_owned()));
        }
        sqlx::query(
            "UPDATE room_sessions SET status = 'closed', closed_at = CURRENT_TIMESTAMP(6)
             WHERE match_id = ? AND status IN ('waiting', 'live')",
        )
        .bind(&command.match_id)
        .execute(&self.pool)
        .await?;
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
            return Err(AppError::Conflict("match cannot be reset".to_owned()));
        }
        sqlx::query("UPDATE match_questions SET used_at = NULL WHERE match_id = ?")
            .bind(&command.match_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE match_participants SET current_score = 0 WHERE match_id = ?")
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
            "SELECT id, organization_id, game_id, game_mode_id, host_user_id, name, status, current_round_id, winner_participant_id, created_at, started_at, ended_at, updated_at FROM matches WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(map_match).transpose()
    }

    async fn list_by_organization(&self, organization_id: &str) -> Result<Vec<Match>, AppError> {
        let rows = sqlx::query(
            "SELECT id, organization_id, game_id, game_mode_id, host_user_id, name, status, current_round_id, winner_participant_id, created_at, started_at, ended_at, updated_at FROM matches WHERE organization_id = ? ORDER BY created_at DESC",
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_match).collect()
    }

    async fn list_participants(&self, match_id: &str) -> Result<Vec<MatchParticipant>, AppError> {
        let rows = sqlx::query(
            "SELECT id, match_id, player_id, display_name_snapshot, join_source, current_score, joined_at, left_at FROM match_participants WHERE match_id = ? ORDER BY current_score DESC, joined_at",
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

    async fn list_live_rooms(&self, organization_id: &str) -> Result<Vec<LiveRoomState>, AppError> {
        let rows = sqlx::query(
            "SELECT rs.id, rs.match_id, m.name AS match_name, m.game_id, g.name AS game_name,
                    rs.pin, rs.status, rs.capacity, u.display_name AS host_name,
                    COUNT(mp.id) AS participants,
                    TIMESTAMPDIFF(MINUTE, rs.opened_at, CURRENT_TIMESTAMP(6)) AS started_minutes_ago
             FROM room_sessions rs
             JOIN matches m ON m.id = rs.match_id
             JOIN games g ON g.id = m.game_id
             JOIN users u ON u.id = m.host_user_id
             LEFT JOIN match_participants mp ON mp.match_id = m.id AND mp.left_at IS NULL
             WHERE rs.organization_id = ? AND rs.status IN ('waiting', 'live')
             GROUP BY rs.id, rs.match_id, m.name, m.game_id, g.name, rs.pin, rs.status, rs.capacity, u.display_name, rs.opened_at
             ORDER BY rs.opened_at DESC",
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_live_room).collect()
    }
}

async fn load_settings(pool: &MySqlPool, match_id: &str) -> Result<MatchSettingsState, AppError> {
    let row = sqlx::query(
        "SELECT max_rounds, time_per_round_seconds, initial_points, remote_enabled, capacity
         FROM match_settings WHERE match_id = ?",
    )
    .bind(match_id)
    .fetch_one(pool)
    .await?;
    Ok(MatchSettingsState {
        max_rounds: row.try_get::<Option<i32>, _>("max_rounds")?.map(|value| value as u32),
        time_per_round_seconds: row
            .try_get::<Option<i32>, _>("time_per_round_seconds")?
            .map(|value| value as u32),
        initial_points: row.try_get("initial_points")?,
        remote_enabled: row.try_get("remote_enabled")?,
        capacity: row
            .try_get::<Option<i32>, _>("capacity")?
            .map(|value| value as u32),
    })
}

async fn load_questions(
    pool: &MySqlPool,
    match_id: &str,
) -> Result<Vec<MatchQuestionState>, AppError> {
    let rows = sqlx::query(
        "SELECT id, position, prompt_snapshot, expected_answer_snapshot, points, used_at
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
                mr.answer_revealed_at, mq.id AS question_id, mq.position, mq.prompt_snapshot,
                mq.expected_answer_snapshot, mq.points, mq.used_at
         FROM match_rounds mr
         LEFT JOIN match_questions mq ON mq.id = mr.match_question_id
         WHERE mr.id = ?",
    )
    .bind(round_id)
    .fetch_optional(pool)
    .await?;
    row.map(map_round_state).transpose()
}

async fn load_room(pool: &MySqlPool, match_id: &str) -> Result<Option<LiveRoomState>, AppError> {
    let row = sqlx::query(
        "SELECT rs.id, rs.match_id, m.name AS match_name, m.game_id, g.name AS game_name,
                rs.pin, rs.status, rs.capacity, u.display_name AS host_name,
                COUNT(mp.id) AS participants,
                TIMESTAMPDIFF(MINUTE, rs.opened_at, CURRENT_TIMESTAMP(6)) AS started_minutes_ago
         FROM room_sessions rs
         JOIN matches m ON m.id = rs.match_id
         JOIN games g ON g.id = m.game_id
         JOIN users u ON u.id = m.host_user_id
         LEFT JOIN match_participants mp ON mp.match_id = m.id AND mp.left_at IS NULL
         WHERE rs.match_id = ?
         GROUP BY rs.id, rs.match_id, m.name, m.game_id, g.name, rs.pin, rs.status, rs.capacity, u.display_name, rs.opened_at",
    )
    .bind(match_id)
    .fetch_optional(pool)
    .await?;
    row.map(map_live_room).transpose()
}

fn map_match(row: sqlx::mysql::MySqlRow) -> Result<Match, AppError> {
    Ok(Match {
        id: mapper::id(row.try_get("id")?)?,
        organization_id: mapper::id(row.try_get("organization_id")?)?,
        game_id: mapper::id(row.try_get("game_id")?)?,
        game_mode_id: mapper::id(row.try_get("game_mode_id")?)?,
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

fn map_participant(row: sqlx::mysql::MySqlRow) -> Result<MatchParticipant, AppError> {
    Ok(MatchParticipant {
        id: mapper::id(row.try_get("id")?)?,
        match_id: mapper::id(row.try_get("match_id")?)?,
        player_id: mapper::opt_id(row.try_get("player_id")?)?,
        display_name_snapshot: mapper::non_empty(
            row.try_get("display_name_snapshot")?,
            "match_participant.display_name_snapshot",
        )?,
        join_source: mapper::join_source(row.try_get("join_source")?),
        current_score: mapper::points(row.try_get("current_score")?),
        joined_at: mapper::datetime(row.try_get("joined_at")?)?,
        left_at: mapper::opt_datetime(row.try_get("left_at")?)?,
    })
}

fn map_question_state(row: sqlx::mysql::MySqlRow) -> Result<MatchQuestionState, AppError> {
    let used_at: Option<chrono::NaiveDateTime> = row.try_get("used_at")?;
    let position: i32 = row.try_get("position")?;
    Ok(MatchQuestionState {
        id: row.try_get("id")?,
        prompt: row.try_get("prompt_snapshot")?,
        expected_answer: row.try_get("expected_answer_snapshot")?,
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
        Some(MatchQuestionState {
            id,
            prompt: row.try_get("prompt_snapshot")?,
            expected_answer: row.try_get("expected_answer_snapshot")?,
            points: row.try_get("points")?,
            used: used_at.is_some(),
            position: position as u32,
        })
    } else {
        None
    };
    let round_number: i32 = row.try_get("round_number")?;
    Ok(MatchRoundState {
        id: row.try_get("id")?,
        match_question_id: row.try_get("match_question_id")?,
        round_number: round_number as u32,
        status: row.try_get("status")?,
        selected_participant_id: row.try_get("selected_participant_id")?,
        answer_revealed: answer_revealed_at.is_some(),
        question,
    })
}

fn map_live_room(row: sqlx::mysql::MySqlRow) -> Result<LiveRoomState, AppError> {
    let participants: i64 = row.try_get("participants")?;
    let started_minutes_ago: Option<i64> = row.try_get("started_minutes_ago")?;
    Ok(LiveRoomState {
        id: row.try_get("id")?,
        match_id: row.try_get("match_id")?,
        match_name: row.try_get("match_name")?,
        game_id: row.try_get("game_id")?,
        game_name: row.try_get("game_name")?,
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
