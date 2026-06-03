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

/// A player enriched with aggregated match statistics (derived from `score_events`).
pub struct PlayerWithStats {
    pub player: Player,
    pub total_points: i64,
    pub matches_played: i64,
    pub wins: i64,
}

impl MySqlPlayerRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn list_by_owner_with_stats(
        &self,
        owner_user_id: &str,
    ) -> Result<Vec<PlayerWithStats>, AppError> {
        let rows = sqlx::query(
            "SELECT p.id, p.owner_user_id, p.user_id, p.display_name, p.is_active, p.created_at, p.updated_at,
                    stats.total_points AS total_points,
                    COALESCE(stats.matches_played, 0) AS matches_played,
                    COALESCE(won.wins, 0) AS wins
             FROM players p
             LEFT JOIN (
                 SELECT mp.player_id, CAST(SUM(se.delta) AS SIGNED) AS total_points, COUNT(DISTINCT mp.match_id) AS matches_played
                 FROM match_participants mp
                 JOIN score_events se ON se.participant_id = mp.id
                 GROUP BY mp.player_id
             ) stats ON stats.player_id = p.id
             LEFT JOIN (
                 SELECT mp.player_id, COUNT(DISTINCT m.id) AS wins
                 FROM match_participants mp
                 JOIN matches m ON m.winner_participant_id = mp.id
                 GROUP BY mp.player_id
             ) won ON won.player_id = p.id
             WHERE p.owner_user_id = ?
             ORDER BY p.display_name",
        )
        .bind(owner_user_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let total_points: Option<i64> = row.try_get("total_points")?;
                let matches_played: i64 = row.try_get("matches_played")?;
                let wins: i64 = row.try_get("wins")?;
                Ok(PlayerWithStats {
                    player: map_player(row)?,
                    total_points: total_points.unwrap_or_default(),
                    matches_played,
                    wins,
                })
            })
            .collect()
    }
}

#[async_trait]
impl PlayerRepository for MySqlPlayerRepository {
    async fn create(&self, id: &str, command: &CreatePlayerCommand) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO players (id, owner_user_id, user_id, display_name, normalized_name) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&command.owner_user_id)
        .bind(&command.user_id)
        .bind(&command.display_name)
        .bind(mapper::normalized_name(&command.display_name))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update(&self, command: &UpdatePlayerCommand) -> Result<(), AppError> {
        if let Some(display_name) = &command.display_name {
            sqlx::query(
                "UPDATE players SET display_name = ?, normalized_name = ?, is_active = COALESCE(?, is_active) WHERE id = ?",
            )
            .bind(display_name)
            .bind(mapper::normalized_name(display_name))
            .bind(command.is_active)
            .bind(&command.player_id)
            .execute(&self.pool)
            .await?;
        } else {
            sqlx::query("UPDATE players SET is_active = COALESCE(?, is_active) WHERE id = ?")
                .bind(command.is_active)
                .bind(&command.player_id)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Player>, AppError> {
        let row = sqlx::query(
            "SELECT id, owner_user_id, user_id, display_name, is_active, created_at, updated_at FROM players WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(map_player).transpose()
    }

    async fn list_by_owner(&self, owner_user_id: &str) -> Result<Vec<Player>, AppError> {
        let rows = sqlx::query(
            "SELECT id, owner_user_id, user_id, display_name, is_active, created_at, updated_at FROM players WHERE owner_user_id = ? ORDER BY display_name",
        )
        .bind(owner_user_id)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(map_player).collect()
    }
}

fn map_player(row: sqlx::mysql::MySqlRow) -> Result<Player, AppError> {
    Ok(Player {
        id: mapper::id(row.try_get("id")?)?,
        owner_user_id: mapper::id(row.try_get("owner_user_id")?)?,
        user_id: mapper::opt_id(row.try_get("user_id")?)?,
        display_name: mapper::non_empty(row.try_get("display_name")?, "player.display_name")?,
        is_active: row.try_get("is_active")?,
        created_at: mapper::datetime(row.try_get("created_at")?)?,
        updated_at: mapper::datetime(row.try_get("updated_at")?)?,
    })
}
