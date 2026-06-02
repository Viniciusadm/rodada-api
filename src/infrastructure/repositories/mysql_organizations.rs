use async_trait::async_trait;
use sqlx::{MySqlPool, Row};

use crate::application::organizations::{CreateOrganizationCommand, OrganizationRepository};
use crate::domain::organizations::{Ministry, Organization};
use crate::errors::AppError;
use crate::infrastructure::repositories::mapper;

#[derive(Clone)]
pub struct MySqlOrganizationRepository {
    pool: MySqlPool,
}

/// A member of an organization with their effective role.
#[derive(Debug, Clone)]
pub struct MemberRecord {
    pub user_id: String,
    pub display_name: String,
    pub email: String,
    pub role: String,
}

fn slugify(value: &str) -> String {
    let slug: String = value
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect();
    let trimmed = slug.trim_matches('-').to_owned();
    // Collapse repeated separators.
    let mut result = String::with_capacity(trimmed.len());
    let mut last_dash = false;
    for ch in trimmed.chars() {
        if ch == '-' {
            if !last_dash {
                result.push(ch);
            }
            last_dash = true;
        } else {
            result.push(ch);
            last_dash = false;
        }
    }
    if result.is_empty() {
        "time".to_owned()
    } else {
        result
    }
}

/// Ranks roles so the strongest one wins when a user somehow has several rows.
fn role_rank(role: &str) -> u8 {
    match role {
        "owner" => 4,
        "admin" => 3,
        "host" => 2,
        _ => 1,
    }
}

impl MySqlOrganizationRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn update_organization(
        &self,
        id: &str,
        name: &str,
        contact_email: Option<&str>,
    ) -> Result<(), AppError> {
        let result =
            sqlx::query("UPDATE organizations SET name = ?, contact_email = ? WHERE id = ?")
                .bind(name)
                .bind(contact_email)
                .bind(id)
                .execute(&self.pool)
                .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("organization was not found".to_owned()));
        }
        Ok(())
    }

    pub async fn create_ministry(
        &self,
        id: &str,
        organization_id: &str,
        name: &str,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO ministries (id, organization_id, name, slug) VALUES (?, ?, ?, ?)",
        )
        .bind(id)
        .bind(organization_id)
        .bind(name)
        .bind(slugify(name))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_ministry(
        &self,
        id: &str,
        organization_id: &str,
        name: &str,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE ministries SET name = ? WHERE id = ? AND organization_id = ?",
        )
        .bind(name)
        .bind(id)
        .bind(organization_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("ministry was not found".to_owned()));
        }
        Ok(())
    }

    pub async fn set_ministry_active(
        &self,
        id: &str,
        organization_id: &str,
        is_active: bool,
    ) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE ministries SET is_active = ? WHERE id = ? AND organization_id = ?",
        )
        .bind(is_active)
        .bind(id)
        .bind(organization_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("ministry was not found".to_owned()));
        }
        Ok(())
    }

    pub async fn list_members(&self, organization_id: &str) -> Result<Vec<MemberRecord>, AppError> {
        let rows = sqlx::query(
            "SELECT u.id AS user_id, u.display_name, u.email, ur.role
             FROM user_roles ur JOIN users u ON u.id = ur.user_id
             WHERE ur.organization_id = ?
             ORDER BY u.display_name",
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await?;

        // Collapse to a single effective (strongest) role per user.
        let mut members: Vec<MemberRecord> = Vec::new();
        for row in rows {
            let user_id: String = row.try_get("user_id")?;
            let role: String = row.try_get("role")?;
            if let Some(existing) = members.iter_mut().find(|m| m.user_id == user_id) {
                if role_rank(&role) > role_rank(&existing.role) {
                    existing.role = role;
                }
            } else {
                members.push(MemberRecord {
                    user_id,
                    display_name: row.try_get("display_name")?,
                    email: row.try_get("email")?,
                    role,
                });
            }
        }
        Ok(members)
    }

    pub async fn find_user_id_by_email(&self, email: &str) -> Result<Option<String>, AppError> {
        let id: Option<String> =
            sqlx::query_scalar("SELECT id FROM users WHERE email = ? LIMIT 1")
                .bind(email)
                .fetch_optional(&self.pool)
                .await?;
        Ok(id)
    }

    pub async fn count_owners(&self, organization_id: &str) -> Result<i64, AppError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT user_id) FROM user_roles WHERE organization_id = ? AND role = 'owner'",
        )
        .bind(organization_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    pub async fn member_role(
        &self,
        organization_id: &str,
        user_id: &str,
    ) -> Result<Option<String>, AppError> {
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT role FROM user_roles WHERE organization_id = ? AND user_id = ?",
        )
        .bind(organization_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().max_by_key(|role| role_rank(role)))
    }

    /// Sets a member's single effective role, replacing any existing role rows.
    pub async fn set_member_role(
        &self,
        organization_id: &str,
        user_id: &str,
        role: &str,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM user_roles WHERE organization_id = ? AND user_id = ?")
            .bind(organization_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO user_roles (id, organization_id, user_id, role) VALUES (UUID(), ?, ?, ?)",
        )
        .bind(organization_id)
        .bind(user_id)
        .bind(role)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn remove_member(
        &self,
        organization_id: &str,
        user_id: &str,
    ) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM user_roles WHERE organization_id = ? AND user_id = ?")
            .bind(organization_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("member was not found".to_owned()));
        }
        Ok(())
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

    async fn list_ministries(&self, organization_id: &str) -> Result<Vec<Ministry>, AppError> {
        let rows = sqlx::query(
            "SELECT id, organization_id, name, slug, is_active, created_at, updated_at FROM ministries WHERE organization_id = ? ORDER BY name",
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(Ministry {
                    id: mapper::id(row.try_get("id")?)?,
                    organization_id: mapper::id(row.try_get("organization_id")?)?,
                    name: mapper::non_empty(row.try_get("name")?, "ministry.name")?,
                    slug: mapper::slug(row.try_get("slug")?)?,
                    is_active: row.try_get("is_active")?,
                    created_at: mapper::datetime(row.try_get("created_at")?)?,
                    updated_at: mapper::datetime(row.try_get("updated_at")?)?,
                })
            })
            .collect()
    }
}
