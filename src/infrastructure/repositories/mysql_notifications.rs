use chrono::NaiveDateTime;
use sqlx::{MySqlPool, Row};
use uuid::Uuid;

use crate::errors::AppError;

#[derive(Clone)]
pub struct MySqlNotificationRepository {
    pool: MySqlPool,
}

pub struct NotificationRow {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub body: Option<String>,
    pub link: Option<String>,
    pub read: bool,
    pub created_at: String,
}

pub struct NotificationPreferenceRow {
    pub key: String,
    pub enabled: bool,
}

impl MySqlNotificationRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    pub async fn list_for_user(
        &self,
        user_id: &str,
        limit: u32,
    ) -> Result<Vec<NotificationRow>, AppError> {
        let rows = sqlx::query(
            "SELECT id, kind, title, body, link, read_at, created_at
             FROM notifications
             WHERE user_id = ? OR user_id IS NULL
             ORDER BY created_at DESC
             LIMIT ?",
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                let read_at: Option<NaiveDateTime> = row.try_get("read_at")?;
                let created_at: NaiveDateTime = row.try_get("created_at")?;
                Ok(NotificationRow {
                    id: row.try_get("id")?,
                    kind: row.try_get("kind")?,
                    title: row.try_get("title")?,
                    body: row.try_get("body")?,
                    link: row.try_get("link")?,
                    read: read_at.is_some(),
                    created_at: created_at.to_string(),
                })
            })
            .collect()
    }

    pub async fn mark_read(&self, notification_id: &str, user_id: &str) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE notifications SET read_at = CURRENT_TIMESTAMP(6)
             WHERE id = ? AND (user_id = ? OR user_id IS NULL) AND read_at IS NULL",
        )
        .bind(notification_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_all_read(&self, user_id: &str) -> Result<(), AppError> {
        sqlx::query(
            "UPDATE notifications SET read_at = CURRENT_TIMESTAMP(6)
             WHERE (user_id = ? OR user_id IS NULL) AND read_at IS NULL",
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn create(
        &self,
        user_id: Option<&str>,
        kind: &str,
        title: &str,
        body: Option<&str>,
        link: Option<&str>,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO notifications (id, user_id, kind, title, body, link)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(user_id)
        .bind(kind)
        .bind(title)
        .bind(body)
        .bind(link)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_preferences(
        &self,
        user_id: &str,
    ) -> Result<Vec<NotificationPreferenceRow>, AppError> {
        let rows = sqlx::query(
            "SELECT preference_key, enabled FROM notification_preferences WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| {
                Ok(NotificationPreferenceRow {
                    key: row.try_get("preference_key")?,
                    enabled: row.try_get("enabled")?,
                })
            })
            .collect()
    }

    pub async fn upsert_preference(
        &self,
        user_id: &str,
        key: &str,
        enabled: bool,
    ) -> Result<(), AppError> {
        sqlx::query(
            "INSERT INTO notification_preferences (id, user_id, preference_key, enabled)
             VALUES (?, ?, ?, ?)
             ON DUPLICATE KEY UPDATE enabled = VALUES(enabled)",
        )
        .bind(Uuid::new_v4().to_string())
        .bind(user_id)
        .bind(key)
        .bind(enabled)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn is_enabled(&self, user_id: &str, key: &str) -> Result<bool, AppError> {
        let row = sqlx::query(
            "SELECT enabled FROM notification_preferences WHERE user_id = ? AND preference_key = ?",
        )
        .bind(user_id)
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(row.try_get("enabled")?),
            None => Ok(true),
        }
    }
}
