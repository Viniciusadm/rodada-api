use async_trait::async_trait;

use crate::application::organizations::CreateOrganizationCommand;
use crate::domain::organizations::Organization;
use crate::errors::AppError;

#[async_trait]
pub trait OrganizationRepository: Send + Sync {
    async fn create(&self, id: &str, command: &CreateOrganizationCommand) -> Result<(), AppError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Organization>, AppError>;
}
