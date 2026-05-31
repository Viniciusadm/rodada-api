use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::auth::{RegisterUserRecord, UserRepository};
use crate::domain::users::{User, UserRole, UserRoleKind};
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub struct MySqlUserRepository {
    pool: MySqlPool,
}

impl MySqlUserRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    async fn find_by_column(&self, column: &str, value: &str) -> Result<Option<User>, AppError> {
        let sql = format!(
            "SELECT id, email, password_hash, display_name, is_active, last_login_at, created_at, updated_at FROM users WHERE {column} = ?"
        );
        let row = sqlx::query(&sql)
            .bind(value)
            .fetch_optional(&self.pool)
            .await?;

        row.map(|row| {
            Ok(User {
                id: mapper::id(row.try_get("id")?)?,
                email: mapper::email(row.try_get("email")?)?,
                password_hash: mapper::non_empty(
                    row.try_get("password_hash")?,
                    "user.password_hash",
                )?,
                display_name: mapper::non_empty(row.try_get("display_name")?, "user.display_name")?,
                is_active: row.try_get("is_active")?,
                last_login_at: mapper::opt_datetime(row.try_get("last_login_at")?)?,
                created_at: mapper::datetime(row.try_get("created_at")?)?,
                updated_at: mapper::datetime(row.try_get("updated_at")?)?,
            })
        })
        .transpose()
    }

    fn map_write_error(error: sqlx::Error) -> AppError {
        if let sqlx::Error::Database(database_error) = &error {
            if database_error.is_unique_violation() {
                return AppError::Conflict("user or organization already exists".to_owned());
            }
        }
        AppError::from(error)
    }
}

#[async_trait]
impl UserRepository for MySqlUserRepository {
    async fn find_by_id(&self, id: &str) -> Result<Option<User>, AppError> {
        self.find_by_column("id", id).await
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        self.find_by_column("email", email).await
    }

    async fn register_user(&self, record: RegisterUserRecord) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "INSERT INTO users (id, email, password_hash, display_name, is_active) VALUES (?, ?, ?, ?, TRUE)",
        )
        .bind(&record.user_id)
        .bind(&record.email)
        .bind(&record.password_hash)
        .bind(&record.display_name)
        .execute(&mut *tx)
        .await
        .map_err(Self::map_write_error)?;

        sqlx::query(
            "INSERT INTO organizations (id, name, slug, contact_email, owner_user_id) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&record.organization_id)
        .bind(&record.organization_name)
        .bind(&record.organization_slug)
        .bind(&record.email)
        .bind(&record.user_id)
        .execute(&mut *tx)
        .await
        .map_err(Self::map_write_error)?;

        sqlx::query(
            "INSERT INTO user_roles (id, organization_id, user_id, role) VALUES (?, ?, ?, 'owner')",
        )
        .bind(&record.role_id)
        .bind(&record.organization_id)
        .bind(&record.user_id)
        .execute(&mut *tx)
        .await
        .map_err(Self::map_write_error)?;

        tx.commit().await?;
        Ok(())
    }

    async fn list_roles_by_user_id(&self, user_id: &str) -> Result<Vec<UserRole>, AppError> {
        let rows = sqlx::query(
            "SELECT id, organization_id, user_id, role, created_at, updated_at FROM user_roles WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(user_role_from_row).collect()
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

fn user_role_from_row(row: sqlx::mysql::MySqlRow) -> Result<UserRole, AppError> {
    Ok(UserRole {
        id: mapper::id(row.try_get("id")?)?,
        organization_id: mapper::id(row.try_get("organization_id")?)?,
        user_id: mapper::id(row.try_get("user_id")?)?,
        role: mapper::user_role_kind(row.try_get("role")?),
        created_at: mapper::datetime(row.try_get("created_at")?)?,
        updated_at: mapper::datetime(row.try_get("updated_at")?)?,
    })
}
