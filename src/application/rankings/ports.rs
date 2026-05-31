use async_trait::async_trait;

use crate::application::rankings::RankingQuery;
use crate::domain::rankings::RankingEntry;
use crate::errors::AppError;

#[async_trait]
pub trait RankingRepository: Send + Sync {
    async fn list_rankings(&self, query: &RankingQuery) -> Result<Vec<RankingEntry>, AppError>;
}
