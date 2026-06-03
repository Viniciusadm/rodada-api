use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::api::extractors::AuthenticatedUser;
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::application::players::PlayerRepository;
use crate::infrastructure::repositories::mysql_players::MySqlPlayerRepository;

const SEARCH_LIMIT: usize = 5;

#[derive(Debug, Deserialize)]
pub struct SearchQueryParams {
    pub q: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub games: Vec<SearchGameResult>,
    pub players: Vec<SearchPlayerResult>,
}

#[derive(Debug, Serialize)]
pub struct SearchGameResult {
    pub id: String,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Serialize)]
pub struct SearchPlayerResult {
    pub id: String,
    pub display_name: String,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/search", get(search))
}

async fn search(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(params): Query<SearchQueryParams>,
) -> Response {
    let term = params.q.unwrap_or_default().trim().to_lowercase();
    if term.is_empty() {
        return Json(SearchResponse {
            games: Vec::new(),
            players: Vec::new(),
        })
        .into_response();
    }

    let games: Vec<SearchGameResult> = Vec::new();

    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let players = match MySqlPlayerRepository::new(db)
        .list_by_owner(&auth.user_id)
        .await
    {
        Ok(players) => players
            .into_iter()
            .filter(|player| player.display_name.as_str().to_lowercase().contains(&term))
            .take(SEARCH_LIMIT)
            .map(|player| SearchPlayerResult {
                id: player.id.as_str().to_owned(),
                display_name: player.display_name.as_str().to_owned(),
            })
            .collect(),
        Err(error) => return app_error(error),
    };

    Json(SearchResponse { games, players }).into_response()
}
