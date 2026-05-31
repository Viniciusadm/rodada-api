use axum::extract::State;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::api::dto::auth::{LoginRequest, LoginResponse, MeOrganization, MeResponse, MeUser};
use crate::api::extractors::AuthenticatedUser;
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::application::auth::{AuthUseCases, UserRepository};
use crate::application::organizations::OrganizationRepository;
use crate::domain::users::UserRoleKind;
use crate::infrastructure::auth::{Argon2PasswordHasher, JwtTokenService};
use crate::infrastructure::repositories::mysql_organizations::MySqlOrganizationRepository;
use crate::infrastructure::repositories::mysql_users::MySqlUserRepository;

fn role_kind_str(kind: UserRoleKind) -> &'static str {
    match kind {
        UserRoleKind::Owner => "owner",
        UserRoleKind::Admin => "admin",
        UserRoleKind::Host => "host",
        UserRoleKind::Member => "member",
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/me", get(me))
}

async fn me(State(state): State<AppState>, auth: AuthenticatedUser) -> Response {
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let users = MySqlUserRepository::new(db.clone());
    let organizations_repo = MySqlOrganizationRepository::new(db);

    let user = match users.find_by_id(&auth.user_id).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return app_error(crate::errors::AppError::NotFound("user was not found".to_owned()))
        }
        Err(error) => return app_error(error),
    };

    let roles = match users.list_roles_by_user_id(&auth.user_id).await {
        Ok(roles) => roles,
        Err(error) => return app_error(error),
    };

    let mut organizations = Vec::with_capacity(roles.len());
    for role in &roles {
        match organizations_repo.find_by_id(role.organization_id.as_str()).await {
            Ok(Some(org)) => organizations.push(MeOrganization {
                id: org.id.as_str().to_owned(),
                name: org.name.as_str().to_owned(),
                slug: org.slug.as_str().to_owned(),
                role: role_kind_str(role.role).to_owned(),
            }),
            Ok(None) => {}
            Err(error) => return app_error(error),
        }
    }

    let active_organization_id = organizations.first().map(|org| org.id.clone());

    Json(MeResponse {
        user: MeUser {
            id: user.id.as_str().to_owned(),
            display_name: user.display_name.as_str().to_owned(),
            email: user.email.as_str().to_owned(),
        },
        organizations,
        active_organization_id,
    })
    .into_response()
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
