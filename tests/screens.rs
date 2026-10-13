use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use holyplay_backend::api;
use holyplay_backend::api::state::AppState;
use holyplay_backend::application::auth::TokenIssuer;
use holyplay_backend::screens;
use tower::ServiceExt;

#[test]
fn present_screen_compiles_without_errors() {
    let compilation = screens::present();
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    assert_eq!(compilation.deck.slides.len(), 1);
    for name in ["situacao", "pergunta", "rodada", "revelada", "placar"] {
        assert!(
            compilation.deck.externs.iter().any(|known| known == name),
            "missing @extern {name}"
        );
    }
}

#[tokio::test]
async fn present_screen_route_serves_the_deck_without_database() {
    let app = api::router(AppState::new(None, "test-secret"));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/screens/present")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let deck: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(deck["slides"].as_array().unwrap().len(), 1);
    assert!(deck["externs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|name| name == "situacao"));
    assert_eq!(deck["state"]["situacao"], "pergunta");
}

#[test]
fn custom_screens_may_declare_any_subset_of_the_contract() {
    let check = screens::check_present(
        "@extern pergunta = \"Exemplo\"\n@data exemplo\n| nome | pontos |\n| --- | --- |\n| Ana | 1 |\n@end\n@extern placar = $exemplo\n\n# $pergunta\n::: for pessoa in $placar\n$pessoa.nome\n:::",
    );
    assert!(check.is_valid(), "{:#?}", check.diagnostics);
}

#[test]
fn custom_screens_report_compile_errors() {
    let check = screens::check_present("# Título {colr=red}");
    assert!(!check.is_valid());
    assert_eq!(check.diagnostics[0].code, "F209");
}

#[test]
fn custom_screens_reject_externs_the_hub_does_not_fill() {
    let check = screens::check_present("\n@extern pontuacao = 0\n\n# $pontuacao");
    assert!(!check.is_valid());
    assert_eq!(check.diagnostics[0].code, "HUB1");
    assert_eq!(check.diagnostics[0].line, 2);
    assert!(check.diagnostics[0].message.contains("$pontuacao"));
}

#[test]
fn custom_screens_reject_externs_with_another_type() {
    let check = screens::check_present("@extern rodada = \"três\"\n@data exemplo\n| nome | pontos |\n| --- | --- |\n| Ana | muitos |\n@end\n@extern placar = $exemplo\n\n# $rodada");
    let codes: Vec<&str> = check.diagnostics.iter().map(|d| d.code.as_str()).collect();
    assert_eq!(codes, vec!["HUB2", "HUB2"]);
    assert!(check.diagnostics[0].message.contains("número"));
    assert!(check.diagnostics[1].message.contains("pontos"));
}

#[test]
fn the_default_screen_follows_the_contract() {
    assert!(screens::check_present(screens::DEFAULT_PRESENT).is_valid());
}

#[tokio::test]
async fn screen_check_route_requires_a_session_and_reports_problems() {
    let secret = "test-secret";
    let app = api::router(AppState::new(None, secret));
    let anonymous = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/screens/present/check")
                .header("content-type", "application/json")
                .body(Body::from(r##"{"source":"# Oi"}"##))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);

    let token = holyplay_backend::infrastructure::auth::JwtTokenService::new(secret)
        .issue_access_token("user-1")
        .unwrap();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/screens/present/check")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {token}"))
                .body(Body::from(
                    r##"{"source":"@extern pontuacao = 0\n\n# $pontuacao"}"##,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let report: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(report["valid"], false);
    assert_eq!(report["diagnostics"][0]["code"], "HUB1");
    assert_eq!(report["deck"]["slides"].as_array().unwrap().len(), 1);
}

#[test]
fn template_validation_rejects_screens_outside_the_contract() {
    use holyplay_backend::application::match_templates::validate_present_screen;
    use holyplay_backend::errors::AppError;
    assert!(validate_present_screen(None).is_ok());
    assert!(validate_present_screen(Some(screens::DEFAULT_PRESENT)).is_ok());
    match validate_present_screen(Some("@extern pontuacao = 0\n\n# A")) {
        Err(AppError::Validation(message)) => {
            assert!(message.contains("linha 1"), "{message}");
            assert!(message.contains("$pontuacao"), "{message}");
        }
        other => panic!("unexpected result: {other:?}"),
    }
}

#[tokio::test]
async fn default_source_is_public_and_match_screens_need_the_database() {
    let app = api::router(AppState::new(None, "test-secret"));
    let source = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/screens/present/source")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(source.status(), StatusCode::OK);
    let body = to_bytes(source.into_body(), usize::MAX).await.unwrap();
    assert_eq!(
        std::str::from_utf8(&body).unwrap(),
        screens::DEFAULT_PRESENT
    );

    let screen = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/matches/match-1/screen")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(screen.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[test]
fn every_contract_suggestion_declares_a_valid_extern() {
    let suggestions = screens::present_suggestions();
    assert_eq!(suggestions.len(), 17);
    for suggestion in &suggestions {
        let check = screens::check_present(&format!("{}\n\n# A", suggestion.declaration));
        assert!(
            check.is_valid(),
            "{}: {:#?}",
            suggestion.name,
            check.diagnostics
        );
        assert_eq!(check.deck.externs, vec![suggestion.name.to_owned()]);
        assert!(!suggestion.detail.is_empty());
    }
}

#[tokio::test]
async fn contract_route_lists_the_suggestions() {
    let app = api::router(AppState::new(None, "test-secret"));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/screens/present/contract")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let list: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let placar = list
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["name"] == "placar")
        .unwrap();
    assert!(placar["declaration"]
        .as_str()
        .unwrap()
        .starts_with("@data exemplo_placar"));
}
