use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::dto::match_templates::{
    MatchTemplatePayload, MatchTemplateResponse, MatchTemplateSummaryResponse,
};
use crate::api::extractors::AuthenticatedUser;
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::application::match_templates::{
    DeleteMatchTemplateCommand, DuplicateMatchTemplateCommand, MatchTemplateUseCases,
};
use crate::infrastructure::repositories::mysql_match_templates::MySqlMatchTemplateRepository;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/match-templates",
            get(list_templates).post(create_template),
        )
        .route(
            "/match-templates/:template_id",
            get(get_template)
                .patch(update_template)
                .delete(delete_template),
        )
        .route(
            "/match-templates/:template_id/duplicate",
            post(duplicate_template),
        )
}

#[derive(Debug, Deserialize)]
struct ListTemplatesParams {
    search: Option<String>,
}

fn use_cases(
    state: AppState,
) -> Result<MatchTemplateUseCases<MySqlMatchTemplateRepository>, Response> {
    let db = state.db().map_err(app_error)?;
    Ok(MatchTemplateUseCases::new(
        MySqlMatchTemplateRepository::new(db),
    ))
}

async fn list_templates(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(params): Query<ListTemplatesParams>,
) -> Response {
    let use_cases = match use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(response) => return response,
    };
    match use_cases
        .list(&auth.user_id, params.search.as_deref())
        .await
    {
        Ok(items) => Json(
            items
                .into_iter()
                .map(MatchTemplateSummaryResponse::from)
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn get_template(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(template_id): Path<String>,
) -> Response {
    let use_cases = match use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(response) => return response,
    };
    match use_cases.get(&auth.user_id, &template_id).await {
        Ok(template) => Json(MatchTemplateResponse::from(template)).into_response(),
        Err(error) => app_error(error),
    }
}

async fn create_template(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(payload): Json<MatchTemplatePayload>,
) -> Response {
    let use_cases = match use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(response) => return response,
    };
    let command = match payload.into_create_command(auth.user_id) {
        Ok(command) => command,
        Err(error) => return app_error(error),
    };
    match use_cases.create(command).await {
        Ok(template) => (
            StatusCode::CREATED,
            Json(MatchTemplateResponse::from(template)),
        )
            .into_response(),
        Err(error) => app_error(error),
    }
}

async fn update_template(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(template_id): Path<String>,
    Json(payload): Json<MatchTemplatePayload>,
) -> Response {
    let use_cases = match use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(response) => return response,
    };
    let command = match payload.into_update_command(template_id, auth.user_id) {
        Ok(command) => command,
        Err(error) => return app_error(error),
    };
    match use_cases.update(command).await {
        Ok(template) => Json(MatchTemplateResponse::from(template)).into_response(),
        Err(error) => app_error(error),
    }
}

async fn delete_template(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(template_id): Path<String>,
) -> Response {
    let use_cases = match use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(response) => return response,
    };
    match use_cases
        .delete(DeleteMatchTemplateCommand {
            id: template_id,
            host_user_id: auth.user_id,
        })
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn duplicate_template(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(template_id): Path<String>,
) -> Response {
    let use_cases = match use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(response) => return response,
    };
    match use_cases
        .duplicate(DuplicateMatchTemplateCommand {
            id: template_id,
            host_user_id: auth.user_id,
        })
        .await
    {
        Ok(template) => (
            StatusCode::CREATED,
            Json(MatchTemplateResponse::from(template)),
        )
            .into_response(),
        Err(error) => app_error(error),
    }
}
