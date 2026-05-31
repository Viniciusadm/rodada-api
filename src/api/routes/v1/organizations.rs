use axum::extract::{Path, Query, State};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};

use crate::api::dto::organizations::{
    MinistryResponse, OrganizationResponse, SearchGameResult, SearchPlayerResult,
    SearchQueryParams, SearchResponse, SearchRoomResult,
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
use crate::infrastructure::repositories::mysql_games::MySqlGameRepository;
use crate::infrastructure::repositories::mysql_matches::MySqlMatchRepository;
use crate::infrastructure::repositories::mysql_organizations::MySqlOrganizationRepository;
use crate::infrastructure::repositories::mysql_players::MySqlPlayerRepository;

const SEARCH_LIMIT: usize = 5;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/organizations/:organization_id", get(get_organization))
        .route(
            "/organizations/:organization_id/ministries",
            get(list_ministries),
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

    let games_repo = MySqlGameRepository::new(db.clone());
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
