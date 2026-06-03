use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};

use crate::api::dto::rankings::{RankingEntryResponse, RankingQueryParams};
use crate::api::extractors::AuthenticatedUser;
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::application::rankings::RankingUseCases;
use crate::infrastructure::repositories::mysql_rankings::MySqlRankingRepository;

pub fn router() -> Router<AppState> {
    Router::new().route("/rankings", get(list_rankings))
}

async fn list_rankings(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(query): Query<RankingQueryParams>,
) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let use_cases = RankingUseCases::new(MySqlRankingRepository::new(db));
    match use_cases.list_rankings(query.into_query(auth.user_id)).await {
        Ok(entries) => Json(
            entries
                .into_iter()
                .map(|entry| RankingEntryResponse {
                    position: entry.position,
                    player_id: entry.player_id.as_str().to_owned(),
                    display_name: entry.display_name,
                    total_points: entry.total_points.value(),
                    matches_played: entry.matches_played,
                    wins: entry.wins,
                })
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => app_error(error),
    }
}
