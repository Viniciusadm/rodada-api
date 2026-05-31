use crate::application::auth::{
    AuthenticatedSession, LoginCommand, PasswordHasher, RegisterCommand, RegisterUserRecord,
    TokenIssuer, UserRepository,
};
use crate::domain::users::UserRoleKind;
use crate::domain::{Email, NonEmptyString};
use crate::errors::AppError;
use uuid::Uuid;

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

    pub async fn register(
        &self,
        command: RegisterCommand,
    ) -> Result<AuthenticatedSession, AppError> {
        let display_name = NonEmptyString::new(command.display_name, "display_name")?
            .as_str()
            .to_owned();
        let email = Email::new(command.email)?.as_str().to_owned();
        if command.password.len() < 8 || command.password.len() > 128 {
            return Err(AppError::Validation(
                "password must be between 8 and 128 characters".to_owned(),
            ));
        }
        if self.users.find_by_email(&email).await?.is_some() {
            return Err(AppError::Conflict("email is already registered".to_owned()));
        }

        let user_id = new_id();
        let organization_id = new_id();
        let organization_name = format!("Comunidade de {display_name}");
        let organization_slug = organization_slug(&display_name);
        self.users
            .register_user(RegisterUserRecord {
                user_id: user_id.clone(),
                organization_id,
                role_id: new_id(),
                email,
                password_hash: self.password_hasher.hash_password(&command.password)?,
                display_name: display_name.clone(),
                organization_name,
                organization_slug,
            })
            .await?;

        Ok(AuthenticatedSession {
            access_token: self.token_issuer.issue_access_token(&user_id)?,
            user_id,
            display_name,
        })
    }
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

fn organization_slug(display_name: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;
    for char in display_name.chars().flat_map(char::to_lowercase) {
        if char.is_ascii_alphanumeric() {
            slug.push(char);
            last_was_dash = false;
        } else if !last_was_dash && !slug.is_empty() {
            slug.push('-');
            last_was_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        slug.push_str("comunidade");
    }
    slug.truncate(80);
    format!("{slug}-{}", new_id().chars().take(8).collect::<String>())
}
