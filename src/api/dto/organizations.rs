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
