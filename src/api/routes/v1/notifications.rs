use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};

use crate::api::dto::notifications::{
    NotificationPreferenceResponse, NotificationResponse, OrganizationScopeParams,
    UpdatePreferenceRequest,
};
use crate::api::extractors::AuthenticatedUser;
use crate::api::middleware::auth::require_organization_role;
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::domain::users::UserRoleKind;
use crate::infrastructure::repositories::mysql_notifications::MySqlNotificationRepository;

const NOTIFICATION_LIMIT: u32 = 30;
pub const PREFERENCE_KEYS: [&str; 3] = ["new_rooms", "ranking_updates", "weekly_summary"];

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/notifications", get(list_notifications))
        .route("/notifications/read-all", post(mark_all_read))
        .route("/notifications/:notification_id/read", post(mark_read))
        .route("/notification-preferences", get(list_preferences))
        .route("/notification-preferences/:key", put(update_preference))
}

async fn list_notifications(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(params): Query<OrganizationScopeParams>,
) -> Response {
    if let Err(error) = require_member_access(&state, &auth, &params.organization_id).await {
        return app_error(error);
    }
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let repository = MySqlNotificationRepository::new(db);
    match repository
        .list_for_user(&params.organization_id, &auth.user_id, NOTIFICATION_LIMIT)
        .await
    {
        Ok(rows) => Json(
            rows.into_iter()
                .map(|row| NotificationResponse {
                    id: row.id,
                    kind: row.kind,
                    title: row.title,
                    body: row.body,
                    link: row.link,
                    read: row.read,
                    created_at: row.created_at,
                })
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn mark_read(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(notification_id): Path<String>,
    Query(params): Query<OrganizationScopeParams>,
) -> Response {
    if let Err(error) = require_member_access(&state, &auth, &params.organization_id).await {
        return app_error(error);
    }
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let repository = MySqlNotificationRepository::new(db);
    match repository
        .mark_read(&notification_id, &params.organization_id, &auth.user_id)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn mark_all_read(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(params): Query<OrganizationScopeParams>,
) -> Response {
    if let Err(error) = require_member_access(&state, &auth, &params.organization_id).await {
        return app_error(error);
    }
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let repository = MySqlNotificationRepository::new(db);
    match repository
        .mark_all_read(&params.organization_id, &auth.user_id)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn list_preferences(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Query(params): Query<OrganizationScopeParams>,
) -> Response {
    if let Err(error) = require_member_access(&state, &auth, &params.organization_id).await {
        return app_error(error);
    }
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let repository = MySqlNotificationRepository::new(db);
    let stored = match repository
        .list_preferences(&params.organization_id, &auth.user_id)
        .await
    {
        Ok(rows) => rows,
        Err(error) => return app_error(error),
    };

    let responses = PREFERENCE_KEYS
        .iter()
        .map(|key| {
            let enabled = stored
                .iter()
                .find(|row| row.key == *key)
                .map(|row| row.enabled)
                .unwrap_or(true);
            NotificationPreferenceResponse {
                key: (*key).to_owned(),
                enabled,
            }
        })
        .collect::<Vec<_>>();

    Json(responses).into_response()
}

async fn update_preference(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(key): Path<String>,
    Json(request): Json<UpdatePreferenceRequest>,
) -> Response {
    if !PREFERENCE_KEYS.contains(&key.as_str()) {
        return app_error(crate::errors::AppError::Validation(format!(
            "unknown notification preference key: {key}"
        )));
    }
    if let Err(error) = require_member_access(&state, &auth, &request.organization_id).await {
        return app_error(error);
    }
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let repository = MySqlNotificationRepository::new(db);
    match repository
        .upsert_preference(
            &request.organization_id,
            &auth.user_id,
            &key,
            request.enabled,
        )
        .await
    {
        Ok(()) => Json(NotificationPreferenceResponse {
            key,
            enabled: request.enabled,
        })
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn require_member_access(
    state: &AppState,
    auth: &AuthenticatedUser,
    organization_id: &str,
) -> Result<(), crate::errors::AppError> {
    require_organization_role(
        state,
        auth,
        organization_id,
        &[
            UserRoleKind::Owner,
            UserRoleKind::Admin,
            UserRoleKind::Host,
            UserRoleKind::Member,
        ],
    )
    .await
}
