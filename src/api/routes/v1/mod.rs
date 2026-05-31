pub mod auth;
pub mod games;
pub mod matches;
pub(crate) mod mod_helpers;
pub mod players;
pub mod rankings;

use axum::Router;

use crate::api::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(auth::router())
        .merge(games::router())
        .merge(matches::router())
        .merge(players::router())
        .merge(rankings::router())
}
