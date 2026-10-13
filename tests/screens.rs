use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use holyplay_backend::api;
use holyplay_backend::api::state::AppState;
use holyplay_backend::application::auth::TokenIssuer;
use holyplay_backend::infrastructure::auth::JwtTokenService;
use holyplay_backend::screens;
use tower::ServiceExt;

const SECRET: &str = "test-secret";

async fn send(request: Request<Body>) -> (StatusCode, Vec<u8>) {
    let response = api::router(AppState::new(None, SECRET))
        .oneshot(request)
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, body.to_vec())
}

fn get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

fn check_request(source: &str, token: Option<&str>) -> Request<Body> {
    let builder = Request::builder()
        .method("POST")
        .uri("/api/v1/screens/present/check")
        .header("content-type", "application/json");
    let builder = match token {
        Some(token) => builder.header("authorization", format!("Bearer {token}")),
        None => builder,
    };
    builder
        .body(Body::from(
            serde_json::json!({ "source": source }).to_string(),
        ))
        .unwrap()
}

fn json(body: &[u8]) -> serde_json::Value {
    serde_json::from_slice(body).unwrap()
}

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
    let (status, body) = send(get("/api/v1/screens/present")).await;
    assert_eq!(status, StatusCode::OK);
    let deck = json(&body);
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
    let (status, _) = send(check_request("# Oi", None)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let token = JwtTokenService::new(SECRET)
        .issue_access_token("user-1")
        .unwrap();
    let (status, body) = send(check_request(
        "@extern pontuacao = 0\n\n# $pontuacao",
        Some(&token),
    ))
    .await;
    assert_eq!(status, StatusCode::OK);
    let report = json(&body);
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
    let (status, body) = send(get("/api/v1/screens/present/source")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        std::str::from_utf8(&body).unwrap(),
        screens::DEFAULT_PRESENT
    );

    let (status, _) = send(get("/api/v1/matches/match-1/screen")).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
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
    let (status, body) = send(get("/api/v1/screens/present/contract")).await;
    assert_eq!(status, StatusCode::OK);
    let list = json(&body);
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
