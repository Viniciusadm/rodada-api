use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Json;
use axum::Router;

use crate::api::dto::games::{GameModeResponse, GameResponse};
use crate::api::extractors::AuthenticatedUser;
use crate::api::middleware::auth::require_organization_role;
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::domain::games::catalog::{self, CatalogGame};
use crate::domain::users::UserRoleKind;
use crate::infrastructure::repositories::static_games::StaticGameRepository;

pub fn router() -> Router<AppState> {
    Router::new().route("/games", get(list_games)).route(
        "/organizations/:organization_id/games",
        get(list_games_admin),
    )
}

async fn list_games(State(state): State<AppState>) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let player_counts = match StaticGameRepository::new(db).player_count_by_game().await {
        Ok(counts) => counts,
        Err(error) => return app_error(error),
    };
    let responses: Vec<GameResponse> = catalog::active_games()
        .map(|game| game_response(game, &player_counts))
        .collect();
    Json(responses).into_response()
}

async fn list_games_admin(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(organization_id): Path<String>,
) -> Response {
    if let Err(error) = require_organization_role(
        &state,
        &auth,
        &organization_id,
        &[UserRoleKind::Owner, UserRoleKind::Admin],
    )
    .await
    {
        return app_error(error);
    }
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let player_counts = match StaticGameRepository::new(db).player_count_by_game().await {
        Ok(counts) => counts,
        Err(error) => return app_error(error),
    };
    let responses: Vec<GameResponse> = catalog::active_games()
        .map(|game| game_response(game, &player_counts))
        .collect();
    Json(responses).into_response()
}

fn game_response(
    game: &CatalogGame,
    player_counts: &std::collections::HashMap<String, i64>,
) -> GameResponse {
    GameResponse {
        id: game.id.to_owned(),
        organization_id: None,
        slug: game.slug.to_owned(),
        name: game.name.to_owned(),
        description: game.description.to_owned(),
        kind: game.kind.as_code().to_owned(),
        icon_key: Some(game.icon_key.to_owned()),
        player_count: player_counts.get(game.id).copied().unwrap_or_default(),
        modes: game
            .modes
            .iter()
            .map(|mode| GameModeResponse {
                id: mode.id.to_owned(),
                code: catalog::mode_code_of(mode.kind).to_owned(),
                name: mode.name.to_owned(),
                description: Some(mode.description.to_owned()),
                default_initial_points: mode.default_initial_points,
                default_time_per_question_seconds: mode.default_time_per_question_seconds,
                is_active: true,
            })
            .collect(),
    }
}
