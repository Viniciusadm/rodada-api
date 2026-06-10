pub mod auth;
pub mod match_templates;
pub mod matches;
pub(crate) mod mod_helpers;
pub mod notifications;
pub mod questions;

use axum::Router;

use crate::api::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(auth::router())
        .merge(match_templates::router())
        .merge(matches::router())
        .merge(notifications::router())
        .merge(questions::router())
}
