use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch};
use axum::{Json, Router};

use crate::api::dto::organizations::{
    AddMemberRequest, CreateMinistryRequest, MemberResponse, MinistryResponse,
    OrganizationResponse, SearchGameResult, SearchPlayerResult, SearchQueryParams, SearchResponse,
    SearchRoomResult, UpdateMemberRoleRequest, UpdateMinistryRequest, UpdateOrganizationRequest,
};
use crate::api::extractors::AuthenticatedUser;
use crate::api::middleware::auth::require_organization_role;
use crate::api::routes::v1::mod_helpers::app_error;
use crate::api::state::AppState;
use crate::application::games::GameRepository;
use crate::application::matches::MatchRepository;
use crate::application::organizations::OrganizationRepository;
use crate::application::players::PlayerRepository;
use crate::domain::users::UserRoleKind;
use crate::errors::AppError;
use crate::infrastructure::repositories::mysql_matches::MySqlMatchRepository;
use crate::infrastructure::repositories::mysql_organizations::MySqlOrganizationRepository;
use crate::infrastructure::repositories::mysql_players::MySqlPlayerRepository;
use crate::infrastructure::repositories::static_games::StaticGameRepository;

const SEARCH_LIMIT: usize = 5;
const VALID_ROLES: [&str; 4] = ["owner", "admin", "host", "member"];

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/organizations/:organization_id",
            get(get_organization).patch(update_organization),
        )
        .route(
            "/organizations/:organization_id/ministries",
            get(list_ministries).post(create_ministry),
        )
        .route(
            "/organizations/:organization_id/ministries/:ministry_id",
            patch(update_ministry).delete(archive_ministry),
        )
        .route(
            "/organizations/:organization_id/members",
            get(list_members).post(add_member),
        )
        .route(
            "/organizations/:organization_id/members/:user_id",
            patch(update_member_role).delete(remove_member),
        )
        .route("/organizations/:organization_id/search", get(search))
}

async fn get_organization(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(organization_id): Path<String>,
) -> Response {
    if let Err(error) = require_member_access(&state, &auth, &organization_id).await {
        return app_error(error);
    }
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let repository = MySqlOrganizationRepository::new(db);
    match repository.find_by_id(&organization_id).await {
        Ok(Some(org)) => Json(OrganizationResponse {
            id: org.id.as_str().to_owned(),
            name: org.name.as_str().to_owned(),
            slug: org.slug.as_str().to_owned(),
            contact_email: org.contact_email.map(|email| email.as_str().to_owned()),
        })
        .into_response(),
        Ok(None) => app_error(crate::errors::AppError::NotFound(
            "organization was not found".to_owned(),
        )),
        Err(error) => app_error(error),
    }
}

async fn list_ministries(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(organization_id): Path<String>,
) -> Response {
    if let Err(error) = require_member_access(&state, &auth, &organization_id).await {
        return app_error(error);
    }
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let repository = MySqlOrganizationRepository::new(db);
    match repository.list_ministries(&organization_id).await {
        Ok(ministries) => Json(
            ministries
                .into_iter()
                .map(|ministry| MinistryResponse {
                    id: ministry.id.as_str().to_owned(),
                    name: ministry.name.as_str().to_owned(),
                    slug: ministry.slug.as_str().to_owned(),
                    is_active: ministry.is_active,
                })
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn search(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(organization_id): Path<String>,
    Query(params): Query<SearchQueryParams>,
) -> Response {
    if let Err(error) = require_member_access(&state, &auth, &organization_id).await {
        return app_error(error);
    }
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };

    let term = params.q.unwrap_or_default().trim().to_lowercase();
    if term.is_empty() {
        return Json(SearchResponse {
            games: Vec::new(),
            players: Vec::new(),
            rooms: Vec::new(),
        })
        .into_response();
    }

    let games_repo = StaticGameRepository::new(db.clone());
    let players_repo = MySqlPlayerRepository::new(db.clone());
    let matches_repo = MySqlMatchRepository::new(db);

    let games = match games_repo.list_active_games(Some(&organization_id)).await {
        Ok(games) => games
            .into_iter()
            .filter(|game| game.name.as_str().to_lowercase().contains(&term))
            .take(SEARCH_LIMIT)
            .map(|game| SearchGameResult {
                id: game.id.as_str().to_owned(),
                name: game.name.as_str().to_owned(),
                slug: game.slug.as_str().to_owned(),
            })
            .collect(),
        Err(error) => return app_error(error),
    };

    let players = match players_repo.list_by_organization(&organization_id).await {
        Ok(players) => players
            .into_iter()
            .filter(|player| {
                player.is_active && player.display_name.as_str().to_lowercase().contains(&term)
            })
            .take(SEARCH_LIMIT)
            .map(|player| SearchPlayerResult {
                id: player.id.as_str().to_owned(),
                display_name: player.display_name.as_str().to_owned(),
                ministry_id: player.ministry_id.map(|id| id.as_str().to_owned()),
            })
            .collect(),
        Err(error) => return app_error(error),
    };

    let rooms = match matches_repo.list_live_rooms(&organization_id).await {
        Ok(rooms) => rooms
            .into_iter()
            .filter(|room| room.match_name.to_lowercase().contains(&term))
            .take(SEARCH_LIMIT)
            .map(|room| SearchRoomResult {
                match_id: room.match_id,
                name: room.match_name,
                pin: room.pin,
            })
            .collect(),
        Err(error) => return app_error(error),
    };

    Json(SearchResponse {
        games,
        players,
        rooms,
    })
    .into_response()
}

async fn update_organization(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(organization_id): Path<String>,
    Json(request): Json<UpdateOrganizationRequest>,
) -> Response {
    if let Err(error) = require_admin_access(&state, &auth, &organization_id).await {
        return app_error(error);
    }
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let repository = MySqlOrganizationRepository::new(db);
    let contact = request
        .contact_email
        .as_deref()
        .filter(|value| !value.trim().is_empty());
    match repository
        .update_organization(&organization_id, request.name.trim(), contact)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn create_ministry(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(organization_id): Path<String>,
    Json(request): Json<CreateMinistryRequest>,
) -> Response {
    if let Err(error) = require_admin_access(&state, &auth, &organization_id).await {
        return app_error(error);
    }
    let name = request.name.trim().to_owned();
    if name.len() < 2 {
        return app_error(AppError::Validation("ministry name is required".to_owned()));
    }
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let repository = MySqlOrganizationRepository::new(db);
    let id = uuid::Uuid::new_v4().to_string();
    match repository
        .create_ministry(&id, &organization_id, &name)
        .await
    {
        Ok(()) => (StatusCode::CREATED, Json(serde_json::json!({ "id": id }))).into_response(),
        Err(error) => app_error(error),
    }
}

async fn update_ministry(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((organization_id, ministry_id)): Path<(String, String)>,
    Json(request): Json<UpdateMinistryRequest>,
) -> Response {
    if let Err(error) = require_admin_access(&state, &auth, &organization_id).await {
        return app_error(error);
    }
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let repository = MySqlOrganizationRepository::new(db);
    if let Some(name) = request.name.as_ref().map(|value| value.trim().to_owned()) {
        if name.len() < 2 {
            return app_error(AppError::Validation("ministry name is required".to_owned()));
        }
        if let Err(error) = repository
            .update_ministry(&ministry_id, &organization_id, &name)
            .await
        {
            return app_error(error);
        }
    }
    if let Some(is_active) = request.is_active {
        if let Err(error) = repository
            .set_ministry_active(&ministry_id, &organization_id, is_active)
            .await
        {
            return app_error(error);
        }
    }
    StatusCode::NO_CONTENT.into_response()
}

async fn archive_ministry(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((organization_id, ministry_id)): Path<(String, String)>,
) -> Response {
    if let Err(error) = require_admin_access(&state, &auth, &organization_id).await {
        return app_error(error);
    }
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let repository = MySqlOrganizationRepository::new(db);
    match repository
        .set_ministry_active(&ministry_id, &organization_id, false)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn list_members(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(organization_id): Path<String>,
) -> Response {
    if let Err(error) = require_admin_access(&state, &auth, &organization_id).await {
        return app_error(error);
    }
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let repository = MySqlOrganizationRepository::new(db);
    match repository.list_members(&organization_id).await {
        Ok(members) => Json(
            members
                .into_iter()
                .map(|member| MemberResponse {
                    user_id: member.user_id,
                    display_name: member.display_name,
                    email: member.email,
                    role: member.role,
                })
                .collect::<Vec<_>>(),
        )
        .into_response(),
        Err(error) => app_error(error),
    }
}

async fn add_member(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(organization_id): Path<String>,
    Json(request): Json<AddMemberRequest>,
) -> Response {
    if let Err(error) = require_admin_access(&state, &auth, &organization_id).await {
        return app_error(error);
    }
    let role = request.role.trim().to_lowercase();
    if !VALID_ROLES.contains(&role.as_str()) {
        return app_error(AppError::Validation("invalid role".to_owned()));
    }
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let repository = MySqlOrganizationRepository::new(db);
    if role == "owner" {
        if let Err(error) = require_owner_access(&state, &auth, &organization_id).await {
            return app_error(error);
        }
    }
    let user_id = match repository.find_user_id_by_email(request.email.trim()).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            return app_error(AppError::NotFound(
                "no user with that email — they must register first".to_owned(),
            ));
        }
        Err(error) => return app_error(error),
    };
    match repository
        .set_member_role(&organization_id, &user_id, &role)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn update_member_role(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((organization_id, user_id)): Path<(String, String)>,
    Json(request): Json<UpdateMemberRoleRequest>,
) -> Response {
    if let Err(error) = require_admin_access(&state, &auth, &organization_id).await {
        return app_error(error);
    }
    let role = request.role.trim().to_lowercase();
    if !VALID_ROLES.contains(&role.as_str()) {
        return app_error(AppError::Validation("invalid role".to_owned()));
    }
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let repository = MySqlOrganizationRepository::new(db);
    let current = match repository.member_role(&organization_id, &user_id).await {
        Ok(Some(role)) => role,
        Ok(None) => return app_error(AppError::NotFound("member was not found".to_owned())),
        Err(error) => return app_error(error),
    };

    if role == "owner" || current == "owner" {
        if let Err(error) = require_owner_access(&state, &auth, &organization_id).await {
            return app_error(error);
        }
    }
    if current == "owner" && role != "owner" {
        if let Err(error) = ensure_not_last_owner(&repository, &organization_id).await {
            return app_error(error);
        }
    }
    match repository
        .set_member_role(&organization_id, &user_id, &role)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn remove_member(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path((organization_id, user_id)): Path<(String, String)>,
) -> Response {
    if let Err(error) = require_admin_access(&state, &auth, &organization_id).await {
        return app_error(error);
    }
    let db = match state.db() {
        Ok(db) => db,
        Err(error) => return app_error(error),
    };
    let repository = MySqlOrganizationRepository::new(db);
    let current = match repository.member_role(&organization_id, &user_id).await {
        Ok(Some(role)) => role,
        Ok(None) => return app_error(AppError::NotFound("member was not found".to_owned())),
        Err(error) => return app_error(error),
    };
    if current == "owner" {
        if let Err(error) = require_owner_access(&state, &auth, &organization_id).await {
            return app_error(error);
        }
        if let Err(error) = ensure_not_last_owner(&repository, &organization_id).await {
            return app_error(error);
        }
    }
    match repository.remove_member(&organization_id, &user_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => app_error(error),
    }
}

async fn ensure_not_last_owner(
    repository: &MySqlOrganizationRepository,
    organization_id: &str,
) -> Result<(), AppError> {
    let owners = repository.count_owners(organization_id).await?;
    if owners <= 1 {
        return Err(AppError::Conflict(
            "the organization must keep at least one owner".to_owned(),
        ));
    }
    Ok(())
}

async fn require_admin_access(
    state: &AppState,
    auth: &AuthenticatedUser,
    organization_id: &str,
) -> Result<(), crate::errors::AppError> {
    require_organization_role(
        state,
        auth,
        organization_id,
        &[UserRoleKind::Owner, UserRoleKind::Admin],
    )
    .await
}

async fn require_owner_access(
    state: &AppState,
    auth: &AuthenticatedUser,
    organization_id: &str,
) -> Result<(), crate::errors::AppError> {
    require_organization_role(state, auth, organization_id, &[UserRoleKind::Owner]).await
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
