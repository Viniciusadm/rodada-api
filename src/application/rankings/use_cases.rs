use crate::application::rankings::{RankingQuery, RankingRepository};
use crate::domain::rankings::RankingEntry;
use crate::errors::AppError;

pub struct RankingUseCases<R> {
    repository: R,
}

impl<R> RankingUseCases<R>
where
    R: RankingRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn list_rankings(&self, query: RankingQuery) -> Result<Vec<RankingEntry>, AppError> {
        self.repository.list_rankings(&query).await
    }
}
