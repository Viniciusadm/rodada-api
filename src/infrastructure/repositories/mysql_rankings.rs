use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::rankings::{RankingQuery, RankingRepository};
use crate::domain::rankings::RankingEntry;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub struct MySqlRankingRepository {
    pool: MySqlPool,
}

impl MySqlRankingRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RankingRepository for MySqlRankingRepository {
    async fn list_rankings(&self, query: &RankingQuery) -> Result<Vec<RankingEntry>, AppError> {
        let limit = query.per_page.min(100);
        let offset = query.page.saturating_sub(1) * limit;
        let rows = sqlx::query(
            "SELECT mp.player_id, p.display_name, m.game_id, CAST(SUM(se.delta) AS SIGNED) AS total_points, COUNT(DISTINCT mp.match_id) AS matches_played, COALESCE(MAX(w.wins), 0) AS wins
             FROM score_events se
             JOIN match_participants mp ON mp.id = se.participant_id
             JOIN matches m ON m.id = se.match_id
             JOIN players p ON p.id = mp.player_id
             LEFT JOIN (
                 SELECT mp2.player_id, m2.game_id, COUNT(DISTINCT m2.id) AS wins
                 FROM matches m2
                 JOIN match_participants mp2 ON m2.winner_participant_id = mp2.id
                 GROUP BY mp2.player_id, m2.game_id
             ) w ON w.player_id = mp.player_id AND w.game_id = m.game_id
             WHERE m.host_user_id = ?
               AND (? IS NULL OR m.game_id = ?)
               AND (? IS NULL OR se.created_at >= ?)
               AND (? IS NULL OR se.created_at <= ?)
             GROUP BY mp.player_id, p.display_name, m.game_id
             ORDER BY total_points DESC
             LIMIT ? OFFSET ?",
        )
        .bind(&query.owner_user_id)
        .bind(&query.game_id)
        .bind(&query.game_id)
        .bind(&query.from)
        .bind(&query.from)
        .bind(&query.to)
        .bind(&query.to)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .enumerate()
            .map(|(index, row)| {
                let total_points: Option<i64> = row.try_get("total_points")?;
                let matches_played: i64 = row.try_get("matches_played")?;
                let wins: i64 = row.try_get("wins")?;
                Ok(RankingEntry {
                    player_id: mapper::id(row.try_get("player_id")?)?,
                    display_name: row.try_get("display_name")?,
                    game_id: mapper::opt_id(row.try_get("game_id")?)?,
                    position: offset + index as u32 + 1,
                    total_points: mapper::points(total_points.unwrap_or_default() as i32),
                    matches_played: matches_played as u32,
                    wins: wins as u32,
                })
            })
            .collect()
    }
}
