use crate::api::extractors::AuthenticatedUser;
use crate::api::state::AppState;
use crate::application::auth::AuthorizationUseCases;
use crate::domain::users::UserRoleKind;
use crate::errors::AppError;
use crate::infrastructure::repositories::mysql_users::MySqlUserRepository;

pub async fn require_organization_role(
    state: &AppState,
    user: &AuthenticatedUser,
    organization_id: &str,
    allowed_roles: &[UserRoleKind],
) -> Result<(), AppError> {
    let db = state.db()?;
    AuthorizationUseCases::new(MySqlUserRepository::new(db))
        .ensure_organization_role(&user.user_id, organization_id, allowed_roles)
        .await
}
