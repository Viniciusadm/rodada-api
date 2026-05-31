use uuid::Uuid;

use crate::application::players::{CreatePlayerCommand, PlayerRepository, UpdatePlayerCommand};
use crate::domain::players::Player;
use crate::errors::AppError;

pub struct PlayerUseCases<R> {
    repository: R,
}

impl<R> PlayerUseCases<R>
where
    R: PlayerRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn create_player(&self, command: CreatePlayerCommand) -> Result<Player, AppError> {
        let id = Uuid::new_v4().to_string();
        self.repository.create(&id, &command).await?;
        self.repository
            .find_by_id(&id)
            .await?
            .ok_or_else(|| AppError::NotFound("created player was not found".to_owned()))
    }

    pub async fn update_player(&self, command: UpdatePlayerCommand) -> Result<(), AppError> {
        self.repository.update(&command).await
    }

    pub async fn list_players(&self, organization_id: &str) -> Result<Vec<Player>, AppError> {
        self.repository.list_by_organization(organization_id).await
    }
}
