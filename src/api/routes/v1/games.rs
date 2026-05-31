use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Json;
use axum::Router;

use crate::api::dto::games::{GameModeResponse, GameResponse};
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::application::games::GameRepository;
use crate::domain::games::{GameModeKind, GameStatus};
use crate::infrastructure::repositories::mysql_games::MySqlGameRepository;

pub fn router() -> Router<AppState> {
    Router::new().route("/games", get(list_games))
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
        Ok(games) => {
            let mut responses = Vec::with_capacity(games.len());
            for game in games {
                let modes = match repository.list_modes(game.id.as_str()).await {
                    Ok(modes) => modes,
                    Err(error) => return app_error(error),
                };
                responses.push(GameResponse {
                    id: game.id.as_str().to_owned(),
                    slug: game.slug.as_str().to_owned(),
                    name: game.name.as_str().to_owned(),
                    description: game.description.as_str().to_owned(),
                    status: game_status(game.status).to_owned(),
                    icon_key: None,
                    player_count: player_counts
                        .get(game.id.as_str())
                        .copied()
                        .unwrap_or_default(),
                    modes: modes
                        .into_iter()
                        .map(|mode| GameModeResponse {
                            id: mode.id.as_str().to_owned(),
                            code: game_mode_code(mode.kind).to_owned(),
                            name: mode.name.as_str().to_owned(),
                        })
                        .collect(),
                });
            }
            Json(responses).into_response()
        }
        Err(error) => app_error(error),
    }
}

fn game_status(status: GameStatus) -> &'static str {
    match status {
        GameStatus::Active => "active",
        GameStatus::Beta => "beta",
        GameStatus::Maintenance => "maintenance",
        GameStatus::ComingSoon => "coming_soon",
    }
}

fn game_mode_code(kind: GameModeKind) -> &'static str {
    match kind {
        GameModeKind::Classic => "classic",
        GameModeKind::InitialPoints => "initial_points",
        GameModeKind::Elimination => "elimination",
    }
}
