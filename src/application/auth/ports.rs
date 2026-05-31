use async_trait::async_trait;

use crate::domain::users::{User, UserRole, UserRoleKind};
use crate::errors::AppError;

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: &str) -> Result<Option<User>, AppError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError>;
    async fn list_roles_by_user_id(&self, user_id: &str) -> Result<Vec<UserRole>, AppError>;
    async fn has_role_in_organization(
        &self,
        user_id: &str,
        organization_id: &str,
        allowed_roles: &[UserRoleKind],
    ) -> Result<bool, AppError>;
}

pub trait PasswordHasher: Send + Sync {
    fn hash_password(&self, password: &str) -> Result<String, AppError>;
    fn verify_password(&self, password: &str, password_hash: &str) -> Result<bool, AppError>;
}

pub trait TokenIssuer: Send + Sync {
    fn issue_access_token(&self, user_id: &str) -> Result<String, AppError>;
}

pub trait TokenVerifier: Send + Sync {
    fn verify_access_token(&self, token: &str) -> Result<String, AppError>;
}
