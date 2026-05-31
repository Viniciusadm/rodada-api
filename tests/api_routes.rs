use axum::body::Body;
use axum::http::{Request, StatusCode};
use holyplay_backend::api;
use holyplay_backend::api::state::AppState;
use holyplay_backend::application::auth::TokenIssuer;
use holyplay_backend::infrastructure::auth::JwtTokenService;
use tower::ServiceExt;

const JWT_SECRET: &str = "test-secret";

#[tokio::test]
async fn health_endpoint_responds_without_database() {
    let app = api::router(AppState::new(None, JWT_SECRET));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn api_v1_routes_are_mounted_and_report_missing_database() {
    let app = api::router(AppState::new(None, JWT_SECRET));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/games")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn register_route_is_mounted_and_reports_missing_database() {
    let app = api::router(AppState::new(None, JWT_SECRET));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"display_name":"Nova Pessoa","email":"nova@example.com","password":"correct-password"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn mutating_match_endpoint_requires_authenticated_user() {
    let app = api::router(AppState::new(None, JWT_SECRET));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/matches/match-1/start")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn authenticated_match_endpoint_reaches_handler() {
    let app = api::router(AppState::new(None, JWT_SECRET));
    let token = JwtTokenService::new(JWT_SECRET)
        .issue_access_token("user-1")
        .unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/matches/match-1/start")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
