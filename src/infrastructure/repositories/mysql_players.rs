use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::players::{CreatePlayerCommand, PlayerRepository, UpdatePlayerCommand};
use crate::domain::players::Player;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub struct MySqlPlayerRepository {
    pool: MySqlPool,
}

impl MySqlPlayerRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PlayerRepository for MySqlPlayerRepository {
    async fn create(&self, id: &str, command: &CreatePlayerCommand) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO players (id, organization_id, user_id, ministry_id, display_name, normalized_name) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&command.organization_id)
        .bind(&command.user_id)
        .bind(&command.ministry_id)
        .bind(&command.display_name)
        .bind(mapper::normalized_name(&command.display_name))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update(&self, command: &UpdatePlayerCommand) -> Result<(), AppError> {
        if let Some(display_name) = &command.display_name {
            sqlx::query(
                "UPDATE players SET ministry_id = COALESCE(?, ministry_id), display_name = ?, normalized_name = ?, is_active = COALESCE(?, is_active) WHERE id = ?",
            )
            .bind(&command.ministry_id)
            .bind(display_name)
            .bind(mapper::normalized_name(display_name))
            .bind(command.is_active)
            .bind(&command.player_id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query(
                "UPDATE players SET ministry_id = COALESCE(?, ministry_id), is_active = COALESCE(?, is_active) WHERE id = ?",
            )
            .bind(&command.ministry_id)
            .bind(command.is_active)
            .bind(&command.player_id)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Player>, AppError> {
        let row = sqlx::query(
            "SELECT id, organization_id, user_id, ministry_id, display_name, is_active, created_at, updated_at FROM players WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(map_player).transpose()
    }

    async fn list_by_organization(&self, organization_id: &str) -> Result<Vec<Player>, AppError> {
        let rows = sqlx::query(
            "SELECT id, organization_id, user_id, ministry_id, display_name, is_active, created_at, updated_at FROM players WHERE organization_id = ? ORDER BY display_name",
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_player).collect()
    }
}

fn map_player(row: sqlx::mysql::MySqlRow) -> Result<Player, AppError> {
    Ok(Player {
        id: mapper::id(row.try_get("id")?)?,
        organization_id: mapper::id(row.try_get("organization_id")?)?,
        user_id: mapper::opt_id(row.try_get("user_id")?)?,
        ministry_id: mapper::opt_id(row.try_get("ministry_id")?)?,
        display_name: mapper::non_empty(row.try_get("display_name")?, "player.display_name")?,
        is_active: row.try_get("is_active")?,
        created_at: mapper::datetime(row.try_get("created_at")?)?,
        updated_at: mapper::datetime(row.try_get("updated_at")?)?,
    })
}
