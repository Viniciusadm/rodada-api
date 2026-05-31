use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::games::{CreateGameCommand, CreateGameModeCommand, GameRepository};
use crate::domain::games::{Game, GameMode};
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub struct MySqlGameRepository {
    pool: MySqlPool,
}

impl MySqlGameRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GameRepository for MySqlGameRepository {
    async fn create_game(&self, id: &str, command: &CreateGameCommand) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO games (id, organization_id, slug, name, description, status, icon_key, sort_order) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&command.organization_id)
        .bind(&command.slug)
        .bind(&command.name)
        .bind(&command.description)
        .bind(&command.status)
        .bind(&command.icon_key)
        .bind(command.sort_order)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_game_mode(
        &self,
        id: &str,
        command: &CreateGameModeCommand,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO game_modes (id, game_id, code, name, description, default_initial_points, default_time_per_question_seconds) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&command.game_id)
        .bind(&command.code)
        .bind(&command.name)
        .bind(&command.description)
        .bind(command.default_initial_points)
        .bind(command.default_time_per_question_seconds)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn find_game_by_id(&self, id: &str) -> Result<Option<Game>, AppError> {
        let row = sqlx::query("SELECT id, organization_id, slug, name, description, status, created_at, updated_at FROM games WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_game).transpose()
    }

    async fn list_active_games(
        &self,
        organization_id: Option<&str>,
    ) -> Result<Vec<Game>, AppError> {
        let rows = if let Some(organization_id) = organization_id {
            sqlx::query("SELECT id, organization_id, slug, name, description, status, created_at, updated_at FROM games WHERE status IN ('active', 'beta') AND (organization_id IS NULL OR organization_id = ?) ORDER BY sort_order, name")
                .bind(organization_id)
                .fetch_all(&self.pool)
                .await?
        } else {
            sqlx::query("SELECT id, organization_id, slug, name, description, status, created_at, updated_at FROM games WHERE status IN ('active', 'beta') ORDER BY sort_order, name")
                .fetch_all(&self.pool)
                .await?
        };
        rows.into_iter().map(map_game).collect()
    }

    async fn list_modes(&self, game_id: &str) -> Result<Vec<GameMode>, AppError> {
        let rows = sqlx::query("SELECT id, game_id, code, name, is_active, created_at, updated_at FROM game_modes WHERE game_id = ? AND is_active = TRUE ORDER BY name")
            .bind(game_id)
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_game_mode).collect()
    }
}

fn map_game(row: sqlx::mysql::MySqlRow) -> Result<Game, AppError> {
    Ok(Game {
        id: mapper::id(row.try_get("id")?)?,
        organization_id: mapper::opt_id(row.try_get("organization_id")?)?,
        slug: mapper::slug(row.try_get("slug")?)?,
        name: mapper::non_empty(row.try_get("name")?, "game.name")?,
        description: mapper::non_empty(row.try_get("description")?, "game.description")?,
        status: mapper::game_status(row.try_get("status")?),
        created_at: mapper::datetime(row.try_get("created_at")?)?,
        updated_at: mapper::datetime(row.try_get("updated_at")?)?,
    })
}

fn map_game_mode(row: sqlx::mysql::MySqlRow) -> Result<GameMode, AppError> {
    Ok(GameMode {
        id: mapper::id(row.try_get("id")?)?,
        game_id: mapper::id(row.try_get("game_id")?)?,
        kind: mapper::game_mode_kind(row.try_get("code")?),
        name: mapper::non_empty(row.try_get("name")?, "game_mode.name")?,
        is_active: row.try_get("is_active")?,
        created_at: mapper::datetime(row.try_get("created_at")?)?,
        updated_at: mapper::datetime(row.try_get("updated_at")?)?,
    })
}
