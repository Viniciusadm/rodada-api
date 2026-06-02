use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::Json;
use axum::Router;

use crate::api::dto::games::{
    CreateGameModeRequest, CreateGameRequest, GameModeResponse, GameResponse, UpdateGameModeRequest,
    UpdateGameRequest,
};
use crate::api::extractors::AuthenticatedUser;
use crate::api::middleware::auth::require_organization_role;
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::application::games::{CreateGameModeCommand, GameRepository};
use crate::domain::games::{Game, GameMode, GameModeKind, GameStatus};
use crate::domain::users::UserRoleKind;
use crate::errors::AppError;
use crate::infrastructure::repositories::mysql_games::MySqlGameRepository;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/games", get(list_games).post(create_game))
        .route(
            "/organizations/:organization_id/games",
            get(list_games_admin),
        )
        .route("/games/:game_id", patch(update_game).delete(archive_game))
        .route("/games/:game_id/modes", post(create_game_mode))
        .route(
            "/games/:game_id/modes/:mode_id",
            patch(update_game_mode).delete(archive_game_mode),
        )
}

async fn list_games(State(state): State<AppState>) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let repository = MySqlGameRepository::new(db);
    let player_counts = match repository.player_count_by_game().await {
        Ok(counts) => counts,
        Err(error) => return app_error(error),
    };
    match repository.list_active_games(None).await {
        Ok(games) => match build_game_responses(&repository, games, &player_counts, false).await {
            Ok(responses) => Json(responses).into_response(),
            Err(error) => app_error(error),
        },
        Err(error) => app_error(error),
    }
}

/// Admin catalog: all non-archived games for the organization (plus global
/// seeded games), including inactive modes. Restricted to owner/admin.
async fn list_games_admin(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(organization_id): Path<String>,
) -> Response {
    if let Err(error) = require_admin_access(&state, &auth, &organization_id).await {
        return app_error(error);
    }
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let repository = MySqlGameRepository::new(db);
    let player_counts = match repository.player_count_by_game().await {
        Ok(counts) => counts,
        Err(error) => return app_error(error),
    };
    match repository.list_games_for_admin(&organization_id).await {
        Ok(games) => match build_game_responses(&repository, games, &player_counts, true).await {
            Ok(responses) => Json(responses).into_response(),
            Err(error) => app_error(error),
        },
        Err(error) => app_error(error),
    }
}

async fn create_game(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(request): Json<CreateGameRequest>,
) -> Response {
    let Some(organization_id) = request.organization_id.clone() else {
        return app_error(AppError::Validation(
            "organization_id is required to create a game".to_owned(),
        ));
    };
    if let Err(error) = require_admin_access(&state, &auth, &organization_id).await {
        return app_error(error);
    }
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let repository = MySqlGameRepository::new(db);
    let id = uuid::Uuid::new_v4().to_string();
    match repository.create_game(&id, &request.into()).await {
        Ok(()) => (StatusCode::CREATED, Json(serde_json::json!({ "id": id }))).into_response(),
        Err(error) => app_error(error),
    }
}

async fn update_game(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(game_id): Path<String>,
    Json(request): Json<UpdateGameRequest>,
) -> Response {
    let (repository, _game) = match authorize_game_admin(&state, &auth, &game_id).await {
        Ok(value) => value,
        Err(error) => return app_error(error),
    };
    match repository.update_game(&request.into_command(game_id)).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn archive_game(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(game_id): Path<String>,
) -> Response {
    let (repository, _game) = match authorize_game_admin(&state, &auth, &game_id).await {
        Ok(value) => value,
        Err(error) => return app_error(error),
    };
    match repository.archive_game(&game_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn create_game_mode(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(game_id): Path<String>,
    Json(request): Json<CreateGameModeRequest>,
) -> Response {
    let (repository, _game) = match authorize_game_admin(&state, &auth, &game_id).await {
        Ok(value) => value,
        Err(error) => return app_error(error),
    };
    let id = uuid::Uuid::new_v4().to_string();
    let command = CreateGameModeCommand {
        game_id: game_id.clone(),
        code: request.code,
        name: request.name,
        description: request.description,
        default_initial_points: request.default_initial_points,
        default_time_per_question_seconds: request.default_time_per_question_seconds,
    };
    match repository.create_game_mode(&id, &command).await {
        Ok(()) => (StatusCode::CREATED, Json(serde_json::json!({ "id": id }))).into_response(),
        Err(error) => app_error(error),
    }
}

async fn update_game_mode(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((game_id, mode_id)): Path<(String, String)>,
    Json(request): Json<UpdateGameModeRequest>,
) -> Response {
    let (repository, _game) = match authorize_game_admin(&state, &auth, &game_id).await {
        Ok(value) => value,
        Err(error) => return app_error(error),
    };
    match repository
        .update_game_mode(&request.into_command(mode_id, game_id))
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn archive_game_mode(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((game_id, mode_id)): Path<(String, String)>,
) -> Response {
    let (repository, _game) = match authorize_game_admin(&state, &auth, &game_id).await {
        Ok(value) => value,
        Err(error) => return app_error(error),
    };
    match repository.archive_game_mode(&mode_id, &game_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

/// Loads a game and verifies the caller is owner/admin of its organization.
/// Globally-seeded games (no organization) are read-only and cannot be edited.
async fn authorize_game_admin(
    state: &AppState,
    auth: &AuthenticatedUser,
    game_id: &str,
) -> Result<(MySqlGameRepository, Game), AppError> {
    let db = state.db()?;
    let repository = MySqlGameRepository::new(db);
    let game = repository
        .find_game_by_id(game_id)
        .await?
        .ok_or_else(|| AppError::NotFound("game was not found".to_owned()))?;
    let Some(organization_id) = game.organization_id.as_ref() else {
        return Err(AppError::Forbidden(
            "global catalog games cannot be edited".to_owned(),
        ));
    };
    require_organization_role(
        state,
        auth,
        organization_id.as_str(),
        &[UserRoleKind::Owner, UserRoleKind::Admin],
    )
    .await?;
    Ok((repository, game))
}

async fn require_admin_access(
    state: &AppState,
    auth: &AuthenticatedUser,
    organization_id: &str,
) -> Result<(), AppError> {
    require_organization_role(
        state,
        auth,
        organization_id,
        &[UserRoleKind::Owner, UserRoleKind::Admin],
    )
    .await
}

/// Builds the catalog response. When `admin` is true, inactive modes are included.
async fn build_game_responses(
    repository: &MySqlGameRepository,
    games: Vec<Game>,
    player_counts: &std::collections::HashMap<String, i64>,
    admin: bool,
) -> Result<Vec<GameResponse>, AppError> {
    let mut responses = Vec::with_capacity(games.len());
    for game in games {
        let modes = if admin {
            repository.list_all_modes(game.id.as_str()).await?
        } else {
            repository.list_modes(game.id.as_str()).await?
        };
        responses.push(GameResponse {
            id: game.id.as_str().to_owned(),
            organization_id: game.organization_id.as_ref().map(|id| id.as_str().to_owned()),
            slug: game.slug.as_str().to_owned(),
            name: game.name.as_str().to_owned(),
            description: game.description.as_str().to_owned(),
            status: game_status(game.status).to_owned(),
            icon_key: None,
            player_count: player_counts
                .get(game.id.as_str())
                .copied()
                .unwrap_or_default(),
            modes: modes.into_iter().map(game_mode_response).collect(),
        });
    }
    Ok(responses)
}

fn game_mode_response(mode: GameMode) -> GameModeResponse {
    GameModeResponse {
        id: mode.id.as_str().to_owned(),
        code: game_mode_code(mode.kind).to_owned(),
        name: mode.name.as_str().to_owned(),
        description: mode.description,
        default_initial_points: mode.default_initial_points,
        default_time_per_question_seconds: mode.default_time_per_question_seconds,
        is_active: mode.is_active,
    }
}

fn game_status(status: GameStatus) -> &'static str {
    match status {
        GameStatus::Active => "active",
        GameStatus::Beta => "beta",
        GameStatus::Maintenance => "maintenance",
        GameStatus::ComingSoon => "coming_soon",
        GameStatus::Archived => "archived",
    }
}

fn game_mode_code(kind: GameModeKind) -> &'static str {
    match kind {
        GameModeKind::Classic => "classic",
        GameModeKind::InitialPoints => "initial_points",
        GameModeKind::Elimination => "elimination",
    }
}
