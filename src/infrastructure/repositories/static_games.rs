use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::games::GameRepository;
use crate::domain::games::catalog;
use crate::domain::games::{Game, GameMode};
use crate::errors::AppError;

/// Serves the game catalog straight from code (`domain::games::catalog`).
/// Player counts still come from the database, so this keeps a pool only for
/// that aggregate query.
#[derive(Clone)]
pub struct StaticGameRepository {
    pool: MySqlPool,
}

impl StaticGameRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Number of distinct players that have participated in matches, keyed by game id.
    pub async fn player_count_by_game(
        &self,
    ) -> Result<std::collections::HashMap<String, i64>, AppError> {
        let rows = sqlx::query(
            "SELECT m.game_id, COUNT(DISTINCT mp.player_id) AS player_count
             FROM matches m
             JOIN match_participants mp ON mp.match_id = m.id
             WHERE mp.player_id IS NOT NULL
             GROUP BY m.game_id",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let game_id: String = row.try_get("game_id")?;
                let count: i64 = row.try_get("player_count")?;
                Ok((game_id, count))
            })
            .collect()
    }
}

#[async_trait]
impl GameRepository for StaticGameRepository {
    async fn find_game_by_id(&self, id: &str) -> Result<Option<Game>, AppError> {
        Ok(catalog::game_by_id(id).map(catalog::to_domain_game))
    }

    async fn list_active_games(
        &self,
        _organization_id: Option<&str>,
    ) -> Result<Vec<Game>, AppError> {
        Ok(catalog::active_games()
            .map(catalog::to_domain_game)
            .collect())
    }

    async fn list_games_for_admin(&self, _organization_id: &str) -> Result<Vec<Game>, AppError> {
        // The catalog has no archived games, so the admin view matches the active view.
        Ok(catalog::active_games()
            .map(catalog::to_domain_game)
            .collect())
    }

    async fn list_modes(&self, game_id: &str) -> Result<Vec<GameMode>, AppError> {
        Ok(catalog::game_by_id(game_id)
            .map(|game| {
                game.modes
                    .iter()
                    .map(|mode| catalog::to_domain_mode(game.id, mode))
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn list_all_modes(&self, game_id: &str) -> Result<Vec<GameMode>, AppError> {
        self.list_modes(game_id).await
    }
}
