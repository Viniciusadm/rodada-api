use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub host: IpAddr,
    pub port: u16,
    pub database_url: Option<String>,
    pub jwt_secret: String,
    pub demo_seeds_enabled: bool,
    pub demo_admin_email: String,
    pub demo_admin_password: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let host = env::var("HTTP_HOST")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
        let port = env::var("HTTP_PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(3001);
        let database_url = env::var("DATABASE_URL").ok();
        let jwt_secret =
            env::var("JWT_SECRET").unwrap_or_else(|_| "development-only-secret".to_owned());
        let demo_seeds_enabled = env::var("DEMO_SEEDS")
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
            .unwrap_or(false);
        let demo_admin_email =
            env::var("DEMO_ADMIN_EMAIL").unwrap_or_else(|_| "admin@holyplay.local".to_owned());
        let demo_admin_password =
            env::var("DEMO_ADMIN_PASSWORD").unwrap_or_else(|_| "holyplay-demo-admin".to_owned());

        Self {
            host,
            port,
            database_url,
            jwt_secret,
            demo_seeds_enabled,
            demo_admin_email,
            demo_admin_password,
        }
    }

    pub fn http_addr(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port)
    }
}
