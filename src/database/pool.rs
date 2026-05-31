use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;

pub async fn connect(database_url: &str) -> Result<MySqlPool, sqlx::Error> {
    MySqlPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
}
