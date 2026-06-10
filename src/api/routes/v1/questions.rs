use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};

use crate::api::dto::questions::{
    GeneralQuestionListResponse, GeneralQuestionRequest, GeneralQuestionResponse,
    ListGeneralQuestionsParams,
};
use crate::api::extractors::AuthenticatedUser;
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::application::questions::{DeleteGeneralQuestionCommand, GeneralQuestionUseCases};
use crate::infrastructure::repositories::mysql_questions::MySqlQuestionRepository;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/general-questions",
            get(list_general_questions).post(create_general_question),
        )
        .route(
            "/general-questions/:question_id",
            axum::routing::patch(update_general_question).delete(delete_general_question),
        )
}

fn use_cases(state: AppState) -> Result<GeneralQuestionUseCases<MySqlQuestionRepository>, Response> {
    let db = state.db().map_err(app_error)?;
    Ok(GeneralQuestionUseCases::new(MySqlQuestionRepository::new(db)))
}

async fn list_general_questions(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(params): Query<ListGeneralQuestionsParams>,
) -> Response {
    let use_cases = match use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(response) => return response,
    };
    let query = params.into_query(auth.user_id);
    let (page, page_size) = (query.page, query.page_size);
    match use_cases.list(query).await {
        Ok((items, total)) => Json(GeneralQuestionListResponse {
            items: items.into_iter().map(GeneralQuestionResponse::from).collect(),
            total,
            page,
            page_size,
        })
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn create_general_question(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Json(request): Json<GeneralQuestionRequest>,
) -> Response {
    let use_cases = match use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(response) => return response,
    };
    let command = match request.into_create_command(auth.user_id) {
        Ok(command) => command,
        Err(error) => return app_error(error),
    };
    match use_cases.create(command).await {
        Ok(question) => {
            (StatusCode::CREATED, Json(GeneralQuestionResponse::from(question))).into_response()
        }
        Err(error) => app_error(error),
    }
}

async fn update_general_question(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(question_id): Path<String>,
    Json(request): Json<GeneralQuestionRequest>,
) -> Response {
    let use_cases = match use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(response) => return response,
    };
    let command = match request.into_update_command(question_id, auth.user_id) {
        Ok(command) => command,
        Err(error) => return app_error(error),
    };
    match use_cases.update(command).await {
        Ok(question) => Json(GeneralQuestionResponse::from(question)).into_response(),
        Err(error) => app_error(error),
    }
}

async fn delete_general_question(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(question_id): Path<String>,
) -> Response {
    let use_cases = match use_cases(state) {
        Ok(use_cases) => use_cases,
        Err(response) => return response,
    };
    match use_cases
        .delete(DeleteGeneralQuestionCommand {
            id: question_id,
            owner_user_id: auth.user_id,
        })
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}
