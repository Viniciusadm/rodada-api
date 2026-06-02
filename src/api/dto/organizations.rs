use serde::{Deserialize, Serialize};

use crate::application::organizations::CreateOrganizationCommand;

#[derive(Debug, Deserialize)]
pub struct CreateOrganizationRequest {
    pub name: String,
    pub slug: String,
    pub contact_email: Option<String>,
    pub owner_user_id: Option<String>,
}

impl From<CreateOrganizationRequest> for CreateOrganizationCommand {
    fn from(value: CreateOrganizationRequest) -> Self {
        Self {
            name: value.name,
            slug: value.slug,
            contact_email: value.contact_email,
            owner_user_id: value.owner_user_id,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct OrganizationResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub contact_email: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MinistryResponse {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrganizationRequest {
    pub name: String,
    pub contact_email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMinistryRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMinistryRequest {
    pub name: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct MemberResponse {
    pub user_id: String,
    pub display_name: String,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub email: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemberRoleRequest {
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct SearchQueryParams {
    pub q: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub games: Vec<SearchGameResult>,
    pub players: Vec<SearchPlayerResult>,
    pub rooms: Vec<SearchRoomResult>,
}

#[derive(Debug, Serialize)]
pub struct SearchGameResult {
    pub id: String,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Serialize)]
pub struct SearchPlayerResult {
    pub id: String,
    pub display_name: String,
    pub ministry_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchRoomResult {
    pub match_id: String,
    pub name: String,
    pub pin: String,
}
