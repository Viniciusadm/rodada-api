use async_trait::async_trait;

use crate::application::games::{CreateGameCommand, CreateGameModeCommand};
use crate::domain::games::{Game, GameMode};
use crate::errors::AppError;

#[async_trait]
pub trait GameRepository: Send + Sync {
    async fn create_game(&self, id: &str, command: &CreateGameCommand) -> Result<(), AppError>;
    async fn create_game_mode(
        &self,
        id: &str,
        command: &CreateGameModeCommand,
    ) -> Result<(), AppError>;
    async fn find_game_by_id(&self, id: &str) -> Result<Option<Game>, AppError>;
    async fn list_active_games(&self, organization_id: Option<&str>)
        -> Result<Vec<Game>, AppError>;
    async fn list_modes(&self, game_id: &str) -> Result<Vec<GameMode>, AppError>;
}
