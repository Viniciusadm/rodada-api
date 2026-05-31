use holyplay_backend::{api, config::AppConfig, database, seeds};
use tokio::net::TcpListener;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = AppConfig::from_env();
    let db = if let Some(database_url) = &config.database_url {
        let pool = database::pool::connect(database_url).await?;
        if config.demo_seeds_enabled {
            seeds::run_demo_seeds(
                &pool,
                seeds::DemoSeedConfig {
                    admin_email: config.demo_admin_email.clone(),
                    admin_password: config.demo_admin_password.clone(),
                },
            )
            .await?;
        }
        Some(pool)
    } else {
        warn!("DATABASE_URL not configured; API endpoints that require persistence will return configuration errors");
        None
    };
    let app = api::router(api::state::AppState::new(db, config.jwt_secret.clone()));
    let listener = TcpListener::bind(config.http_addr()).await?;

    info!("holyplay backend listening on {}", config.http_addr());
    axum::serve(listener, app).await?;

    Ok(())
}
