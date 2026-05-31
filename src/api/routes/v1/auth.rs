use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};

use crate::api::dto::auth::{LoginRequest, LoginResponse};
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::application::auth::AuthUseCases;
use crate::infrastructure::auth::{Argon2PasswordHasher, JwtTokenService};
use crate::infrastructure::repositories::mysql_users::MySqlUserRepository;

pub fn router() -> Router<AppState> {
    Router::new().route("/auth/login", post(login))
}

async fn login(State(state): State<AppState>, Json(request): Json<LoginRequest>) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let use_cases = AuthUseCases::new(
        MySqlUserRepository::new(db),
        Argon2PasswordHasher,
        JwtTokenService::new(state.jwt_secret),
    );

    match use_cases.login(request.into()).await {
        Ok(session) => Json(LoginResponse {
            access_token: session.access_token,
            token_type: "Bearer".to_owned(),
            user_id: session.user_id,
            display_name: session.display_name,
        })
        .into_response(),
        Err(error) => app_error(error),
    }
}
