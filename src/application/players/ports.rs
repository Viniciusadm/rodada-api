use async_trait::async_trait;

use crate::application::players::{CreatePlayerCommand, UpdatePlayerCommand};
use crate::domain::players::Player;
use crate::errors::AppError;

#[async_trait]
pub trait PlayerRepository: Send + Sync {
    async fn create(&self, id: &str, command: &CreatePlayerCommand) -> Result<(), AppError>;
    async fn update(&self, command: &UpdatePlayerCommand) -> Result<(), AppError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Player>, AppError>;
    async fn list_by_owner(&self, owner_user_id: &str) -> Result<Vec<Player>, AppError>;
}
