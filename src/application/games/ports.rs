use async_trait::async_trait;

use crate::application::games::{
    CreateGameCommand, CreateGameModeCommand, UpdateGameCommand, UpdateGameModeCommand,
};
use crate::domain::games::{Game, GameMode};
use crate::errors::AppError;

#[async_trait]
pub trait GameRepository: Send + Sync {
    async fn create_game(&self, id: &str, command: &CreateGameCommand) -> Result<(), AppError>;
    async fn update_game(&self, command: &UpdateGameCommand) -> Result<(), AppError>;
    async fn archive_game(&self, id: &str) -> Result<(), AppError>;
    async fn create_game_mode(
        &self,
        id: &str,
        command: &CreateGameModeCommand,
    ) -> Result<(), AppError>;
    async fn update_game_mode(&self, command: &UpdateGameModeCommand) -> Result<(), AppError>;
    async fn archive_game_mode(&self, id: &str, game_id: &str) -> Result<(), AppError>;
    async fn find_game_by_id(&self, id: &str) -> Result<Option<Game>, AppError>;
    async fn list_active_games(&self, organization_id: Option<&str>)
        -> Result<Vec<Game>, AppError>;
    /// All non-archived games for an org (plus global catalog games), regardless
    /// of status — the admin catalog view.
    async fn list_games_for_admin(&self, organization_id: &str) -> Result<Vec<Game>, AppError>;
    async fn list_modes(&self, game_id: &str) -> Result<Vec<GameMode>, AppError>;
    /// All modes for a game including inactive ones — the admin mode view.
    async fn list_all_modes(&self, game_id: &str) -> Result<Vec<GameMode>, AppError>;
}
