use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use holyplay_backend::application::auth::{
    AuthUseCases, AuthorizationUseCases, LoginCommand, PasswordHasher, RegisterCommand,
    RegisterUserRecord, TokenIssuer, UserRepository,
};
use holyplay_backend::domain::users::{User, UserRole, UserRoleKind};
use holyplay_backend::domain::{DomainId, Email, NonEmptyString, UtcDateTime};
use holyplay_backend::errors::AppError;

#[tokio::test]
async fn login_issues_token_for_valid_active_user() {
    let use_cases = AuthUseCases::new(FakeUsers::default(), FakePasswords, FakeTokens);

    let session = use_cases
        .login(LoginCommand {
            email: "host@example.com".to_owned(),
            password: "correct".to_owned(),
        })
        .await
        .unwrap();

    assert_eq!(session.user_id, "user-1");
    assert_eq!(session.access_token, "token-for-user-1");
}

#[tokio::test]
async fn register_creates_user_organization_owner_role_and_issues_token() {
    let users = FakeUsers::default();
    let use_cases = AuthUseCases::new(users.clone(), FakePasswords, FakeTokens);

    let session = use_cases
        .register(RegisterCommand {
            display_name: "Nova Pessoa".to_owned(),
            email: "NOVA@EXAMPLE.COM".to_owned(),
            password: "correct-password".to_owned(),
        })
        .await
        .unwrap();

    assert_eq!(session.display_name, "Nova Pessoa");
    assert_eq!(
        session.access_token,
        format!("token-for-{}", session.user_id)
    );

    let records = users.registered.lock().unwrap();
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record.user_id, session.user_id);
    assert_eq!(record.email, "nova@example.com");
    assert_eq!(record.password_hash, "hash-correct-password");
    assert_eq!(record.display_name, "Nova Pessoa");
    assert_eq!(record.organization_name, "Comunidade de Nova Pessoa");
    assert!(record.organization_slug.starts_with("nova-pessoa-"));
}

#[tokio::test]
async fn register_rejects_duplicate_email() {
    let use_cases = AuthUseCases::new(
        FakeUsers::with_existing_email("taken@example.com"),
        FakePasswords,
        FakeTokens,
    );

    let result = use_cases
        .register(RegisterCommand {
            display_name: "Nova Pessoa".to_owned(),
            email: "taken@example.com".to_owned(),
            password: "correct-password".to_owned(),
        })
        .await;

    assert!(matches!(result, Err(AppError::Conflict(_))));
}

#[tokio::test]
async fn register_rejects_short_password() {
    let use_cases = AuthUseCases::new(FakeUsers::default(), FakePasswords, FakeTokens);

    let result = use_cases
        .register(RegisterCommand {
            display_name: "Nova Pessoa".to_owned(),
            email: "nova@example.com".to_owned(),
            password: "short".to_owned(),
        })
        .await;

    assert!(matches!(result, Err(AppError::Validation(_))));
}

#[tokio::test]
async fn user_without_role_is_forbidden_from_other_organization() {
    let use_cases = AuthorizationUseCases::new(FakeUsers::default());

    let result = use_cases
        .ensure_organization_role("user-1", "org-2", &[UserRoleKind::Host])
        .await;

    assert!(matches!(result, Err(AppError::Forbidden(_))));
}

#[tokio::test]
async fn user_with_allowed_role_can_access_organization() {
    let use_cases = AuthorizationUseCases::new(FakeUsers::default());

    let result = use_cases
        .ensure_organization_role(
            "user-1",
            "org-1",
            &[UserRoleKind::Owner, UserRoleKind::Host],
        )
        .await;

    assert!(result.is_ok());
}

#[derive(Clone, Default)]
struct FakeUsers {
    existing_email: Option<String>,
    registered: Arc<Mutex<Vec<RegisterUserRecord>>>,
}

impl FakeUsers {
    fn with_existing_email(email: &str) -> Self {
        Self {
            existing_email: Some(email.to_owned()),
            registered: Arc::default(),
        }
    }
}

#[async_trait]
impl UserRepository for FakeUsers {
    async fn find_by_id(&self, id: &str) -> Result<Option<User>, AppError> {
        Ok((id == "user-1").then(fake_user))
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        Ok(
            (email == "host@example.com" || self.existing_email.as_deref() == Some(email))
                .then(fake_user),
        )
    }

    async fn register_user(&self, record: RegisterUserRecord) -> Result<(), AppError> {
        self.registered.lock().unwrap().push(record);
        Ok(())
    }

    async fn list_roles_by_user_id(&self, user_id: &str) -> Result<Vec<UserRole>, AppError> {
        if user_id != "user-1" {
            return Ok(Vec::new());
        }

        Ok(vec![UserRole {
            id: id("role-1"),
            organization_id: id("org-1"),
            user_id: id("user-1"),
            role: UserRoleKind::Host,
            created_at: now(),
            updated_at: now(),
        }])
    }

    async fn has_role_in_organization(
        &self,
        user_id: &str,
        organization_id: &str,
        allowed_roles: &[UserRoleKind],
    ) -> Result<bool, AppError> {
        let roles = self.list_roles_by_user_id(user_id).await?;
        Ok(roles.into_iter().any(|role| {
            role.organization_id.as_str() == organization_id && allowed_roles.contains(&role.role)
        }))
    }
}

struct FakePasswords;

impl PasswordHasher for FakePasswords {
    fn hash_password(&self, password: &str) -> Result<String, AppError> {
        Ok(format!("hash-{password}"))
    }

    fn verify_password(&self, password: &str, password_hash: &str) -> Result<bool, AppError> {
        Ok(password == "correct" && password_hash == "hash-correct")
    }
}

struct FakeTokens;

impl TokenIssuer for FakeTokens {
    fn issue_access_token(&self, user_id: &str) -> Result<String, AppError> {
        Ok(format!("token-for-{user_id}"))
    }
}

fn fake_user() -> User {
    User {
        id: id("user-1"),
        email: Email::new("host@example.com".to_owned()).unwrap(),
        password_hash: NonEmptyString::new("hash-correct".to_owned(), "password_hash").unwrap(),
        display_name: NonEmptyString::new("Host User".to_owned(), "display_name").unwrap(),
        is_active: true,
        last_login_at: None,
        created_at: now(),
        updated_at: now(),
    }
}

fn id(value: &str) -> DomainId {
    DomainId::new(value.to_owned()).unwrap()
}

fn now() -> UtcDateTime {
    UtcDateTime::new_iso8601(Utc::now().naive_utc().to_string()).unwrap()
}
