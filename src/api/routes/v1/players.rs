use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::api::dto::players::{CreatePlayerRequest, PlayerResponse};
use crate::api::extractors::AuthenticatedUser;
use crate::api::middleware::auth::require_organization_role;
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::application::players::PlayerUseCases;
use crate::domain::users::UserRoleKind;
use crate::infrastructure::repositories::mysql_players::MySqlPlayerRepository;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/organizations/:organization_id/players", get(list_players))
        .route("/players", post(create_player))
}

async fn list_players(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(organization_id): Path<String>,
) -> Response {
    if let Err(error) = require_member_access(&state, &auth, &organization_id).await {
        return app_error(error);
    }
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let repository = MySqlPlayerRepository::new(db);
    match repository
        .list_by_organization_with_stats(&organization_id)
        .await
    {
        Ok(players) => Json(
            players
                .into_iter()
                .map(|entry| PlayerResponse {
                    id: entry.player.id.as_str().to_owned(),
                    display_name: entry.player.display_name.as_str().to_owned(),
                    ministry_id: entry.player.ministry_id.map(|id| id.as_str().to_owned()),
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
    if let Err(error) = require_host_access(&state, &auth, &request.organization_id).await {
        return app_error(error);
    }
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let use_cases = PlayerUseCases::new(MySqlPlayerRepository::new(db));
    match use_cases.create_player(request.into()).await {
        Ok(player) => (
            axum::http::StatusCode::CREATED,
            Json(player_response(player)),
        )
            .into_response(),
        Err(error) => app_error(error),
    }
}

async fn require_member_access(
    state: &AppState,
    auth: &AuthenticatedUser,
    organization_id: &str,
) -> Result<(), crate::errors::AppError> {
    require_organization_role(
        state,
        auth,
        organization_id,
        &[
            UserRoleKind::Owner,
            UserRoleKind::Admin,
            UserRoleKind::Host,
            UserRoleKind::Member,
        ],
    )
    .await
}

async fn require_host_access(
    state: &AppState,
    auth: &AuthenticatedUser,
    organization_id: &str,
) -> Result<(), crate::errors::AppError> {
    require_organization_role(
        state,
        auth,
        organization_id,
        &[UserRoleKind::Owner, UserRoleKind::Admin, UserRoleKind::Host],
    )
    .await
}

fn player_response(player: crate::domain::players::Player) -> PlayerResponse {
    PlayerResponse {
        id: player.id.as_str().to_owned(),
        display_name: player.display_name.as_str().to_owned(),
        ministry_id: player.ministry_id.map(|id| id.as_str().to_owned()),
        is_active: player.is_active,
        total_points: 0,
        matches_played: 0,
        wins: 0,
    }
}
