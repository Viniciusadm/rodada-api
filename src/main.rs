use holyplay_backend::{api, config::AppConfig, database, seeds};
use tokio::net::TcpListener;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let dotenv_result = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    match dotenv_result {
        Ok(path) => info!("loaded environment from {}", path.display()),
        Err(err) if err.not_found() => {}
        Err(err) => warn!("failed to load .env file: {err}"),
    }

    let config = AppConfig::from_env();
    let db = if let Some(database_url) = &config.database_url {
        let pool = database::pool::connect(database_url).await?;
        if config.run_migrations {
            info!("running database migrations");
            sqlx::migrate!("./migrations").run(&pool).await?;
        }
        // Required catalog (games + modes) runs on every startup; it is idempotent
        // and the product cannot create matches without it.
        info!("seeding required game catalog");
        seeds::run_catalog_seeds(&pool).await?;
        // Demo/filler data only when explicitly requested.
        if config.demo_seeds_enabled {
            info!("seeding demo data (DEMO_SEEDS enabled)");
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

    info!("games hub backend listening on {}", config.http_addr());
    axum::serve(listener, app).await?;

    Ok(())
}
