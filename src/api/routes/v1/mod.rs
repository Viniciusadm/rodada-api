pub mod auth;
pub mod matches;
pub(crate) mod mod_helpers;
pub mod notifications;

use axum::Router;

use crate::api::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(auth::router())
        .merge(matches::router())
        .merge(notifications::router())
}
