use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::api::extractors::AuthenticatedUser;
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::errors::AppError;
use crate::infrastructure::repositories::mysql_matches::find_present_screen;
use crate::screens::{self, ScreenDiagnostic};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/screens/present", get(present_screen))
        .route("/screens/present/check", post(check_present_screen))
        .route("/screens/present/source", get(present_screen_source))
        .route("/screens/present/contract", get(present_screen_contract))
        .route("/screens/present/templates", get(present_screen_templates))
        .route("/matches/:match_id/screen", get(match_screen))
}

#[derive(Deserialize)]
struct CheckScreenRequest {
    source: String,
}

#[derive(Serialize)]
struct CheckScreenResponse {
    valid: bool,
    document: ribalta::Document,
    diagnostics: Vec<ScreenDiagnostic>,
}

async fn present_screen() -> Json<&'static ribalta::Document> {
    Json(&screens::present().document)
}

async fn present_screen_contract() -> Json<Vec<screens::ExternSuggestion>> {
    Json(screens::present_suggestions())
}

async fn present_screen_templates() -> Json<&'static [screens::TemplateSummary]> {
    Json(screens::present_templates())
}

async fn present_screen_source() -> &'static str {
    screens::DEFAULT_PRESENT
}

async fn match_screen(State(state): State<AppState>, Path(match_id): Path<String>) -> Response {
    let pool = match state.db() {
        Ok(pool) => pool,
        Err(error) => return app_error(error),
    };
    match find_present_screen(&pool, &match_id).await {
        Ok(Some(source)) => Json(screens::present_document(source.as_deref())).into_response(),
        Ok(None) => app_error(AppError::NotFound("match was not found".to_owned())),
        Err(error) => app_error(error),
    }
}

async fn check_present_screen(
    _auth: AuthenticatedUser,
    Json(request): Json<CheckScreenRequest>,
) -> Json<CheckScreenResponse> {
    let check = screens::check_present(&request.source);
    Json(CheckScreenResponse {
        valid: check.is_valid(),
        document: check.document,
        diagnostics: check.diagnostics,
    })
}
