use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch};
use axum::{Json, Router};

use crate::api::dto::players::{CreatePlayerRequest, PlayerResponse, UpdatePlayerRequest};
use crate::api::extractors::AuthenticatedUser;
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::application::players::PlayerRepository;
use crate::application::players::PlayerUseCases;
use crate::infrastructure::repositories::mysql_players::MySqlPlayerRepository;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/players", get(list_players).post(create_player))
        .route("/players/:player_id", patch(update_player))
}

async fn list_players(State(state): State<AppState>, auth: AuthenticatedUser) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let repository = MySqlPlayerRepository::new(db);
    match repository.list_by_owner_with_stats(&auth.user_id).await {
        Ok(players) => Json(
            players
                .into_iter()
                .map(|entry| PlayerResponse {
                    id: entry.player.id.as_str().to_owned(),
                    display_name: entry.player.display_name.as_str().to_owned(),
                    is_active: entry.player.is_active,
                    total_points: entry.total_points,
                    matches_played: entry.matches_played,
                    wins: entry.wins,
                })
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn create_player(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(request): Json<CreatePlayerRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let use_cases = PlayerUseCases::new(MySqlPlayerRepository::new(db));
    match use_cases
        .create_player(request.into_command(auth.user_id))
        .await
    {
        Ok(player) => (
            axum::http::StatusCode::CREATED,
            Json(player_response(player)),
        )
            .into_response(),
        Err(error) => app_error(error),
    }
}

async fn update_player(
    State(state): State<AppState>,
    _auth: AuthenticatedUser,
    Path(player_id): Path<String>,
    Json(request): Json<UpdatePlayerRequest>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let repository = MySqlPlayerRepository::new(db);
    if let Err(error) = repository.find_by_id(&player_id).await {
        return app_error(error);
    }
    let read_repository = repository.clone();
    let use_cases = PlayerUseCases::new(repository);
    match use_cases
        .update_player(request.into_command(player_id.clone()))
        .await
    {
        Ok(()) => match read_repository.find_by_id(&player_id).await {
            Ok(Some(player)) => Json(player_response(player)).into_response(),
            Ok(None) => app_error(crate::errors::AppError::NotFound(
                "player was not found".to_owned(),
            )),
            Err(error) => app_error(error),
        },
        Err(error) => app_error(error),
    }
}

fn player_response(player: crate::domain::players::Player) -> PlayerResponse {
    PlayerResponse {
        id: player.id.as_str().to_owned(),
        display_name: player.display_name.as_str().to_owned(),
        is_active: player.is_active,
        total_points: 0,
        matches_played: 0,
        wins: 0,
    }
}
