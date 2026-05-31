use async_trait::async_trait;

use crate::application::organizations::CreateOrganizationCommand;
use crate::domain::organizations::{Ministry, Organization};
use crate::errors::AppError;

#[async_trait]
pub trait OrganizationRepository: Send + Sync {
    async fn create(&self, id: &str, command: &CreateOrganizationCommand) -> Result<(), AppError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Organization>, AppError>;
    async fn list_ministries(&self, organization_id: &str) -> Result<Vec<Ministry>, AppError>;
}
