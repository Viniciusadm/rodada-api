use crate::application::auth::{
    AuthenticatedSession, LoginCommand, PasswordHasher, TokenIssuer, UserRepository,
};
use crate::domain::users::UserRoleKind;
use crate::errors::AppError;

pub struct AuthUseCases<R, H, T> {
    users: R,
    password_hasher: H,
    token_issuer: T,
}

pub struct AuthorizationUseCases<R> {
    users: R,
}

impl<R> AuthorizationUseCases<R>
where
    R: UserRepository,
{
    pub fn new(users: R) -> Self {
        Self { users }
    }

    pub async fn ensure_organization_role(
        &self,
        user_id: &str,
        organization_id: &str,
        allowed_roles: &[UserRoleKind],
    ) -> Result<(), AppError> {
        let allowed = self
            .users
            .has_role_in_organization(user_id, organization_id, allowed_roles)
            .await?;
        if allowed {
            Ok(())
        } else {
            Err(AppError::Forbidden(
                "user has no role in this organization".to_owned(),
            ))
        }
    }
}

impl<R, H, T> AuthUseCases<R, H, T>
where
    R: UserRepository,
    H: PasswordHasher,
    T: TokenIssuer,
{
    pub fn new(users: R, password_hasher: H, token_issuer: T) -> Self {
        Self {
            users,
            password_hasher,
            token_issuer,
        }
    }

    pub async fn login(&self, command: LoginCommand) -> Result<AuthenticatedSession, AppError> {
        let user = self
            .users
            .find_by_email(&command.email)
            .await?
            .ok_or_else(|| AppError::Unauthorized("invalid credentials".to_owned()))?;

        if !user.is_active {
            return Err(AppError::Unauthorized("inactive user".to_owned()));
        }

        let valid_password = self
            .password_hasher
            .verify_password(&command.password, user.password_hash.as_str())?;
        if !valid_password {
            return Err(AppError::Unauthorized("invalid credentials".to_owned()));
        }

        Ok(AuthenticatedSession {
            access_token: self.token_issuer.issue_access_token(user.id.as_str())?,
            user_id: user.id.as_str().to_owned(),
            display_name: user.display_name.as_str().to_owned(),
        })
    }
}
