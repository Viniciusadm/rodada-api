use sqlx::MySqlPool;

use crate::realtime::RealtimeHub;

#[derive(Debug, Clone)]
pub struct AppState {
    pub db: Option<MySqlPool>,
    pub jwt_secret: String,
    pub realtime: RealtimeHub,
}

impl AppState {
    pub fn new(db: Option<MySqlPool>, jwt_secret: impl Into<String>) -> Self {
        Self {
            db,
            jwt_secret: jwt_secret.into(),
            realtime: RealtimeHub::default(),
        }
    }

    pub fn db(&self) -> Result<MySqlPool, crate::errors::AppError> {
        self.db.clone().ok_or_else(|| {
            crate::errors::AppError::Configuration("DATABASE_URL is not configured".to_owned())
        })
    }
}
