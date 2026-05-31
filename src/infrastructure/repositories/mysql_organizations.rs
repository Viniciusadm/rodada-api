use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::organizations::{CreateOrganizationCommand, OrganizationRepository};
use crate::domain::organizations::Organization;
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub struct MySqlOrganizationRepository {
    pool: MySqlPool,
}

impl MySqlOrganizationRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OrganizationRepository for MySqlOrganizationRepository {
    async fn create(&self, id: &str, command: &CreateOrganizationCommand) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO organizations (id, name, slug, contact_email, owner_user_id) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(&command.name)
        .bind(&command.slug)
        .bind(&command.contact_email)
        .bind(&command.owner_user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Organization>, AppError> {
        let row = sqlx::query(
            "SELECT id, name, slug, contact_email, owner_user_id, created_at, updated_at FROM organizations WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| {
            Ok(Organization {
                id: mapper::id(row.try_get("id")?)?,
                name: mapper::non_empty(row.try_get("name")?, "organization.name")?,
                slug: mapper::slug(row.try_get("slug")?)?,
                contact_email: row
                    .try_get::<Option<String>, _>("contact_email")?
                    .map(mapper::email)
                    .transpose()?,
                owner_user_id: mapper::opt_id(row.try_get("owner_user_id")?)?,
                created_at: mapper::datetime(row.try_get("created_at")?)?,
                updated_at: mapper::datetime(row.try_get("updated_at")?)?,
            })
        })
        .transpose()
    }
}
