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
    assert_eq!(compilation.document.pages.len(), 1);
    for name in ["status", "question", "round", "revealed", "scoreboard"] {
        assert!(
            compilation.document.externs.iter().any(|known| known == name),
            "missing @extern {name}"
        );
    }
}

#[tokio::test]
async fn present_screen_route_serves_the_document_without_database() {
    let (status, body) = send(get("/api/v1/screens/present")).await;
    assert_eq!(status, StatusCode::OK);
    let document = json(&body);
    assert_eq!(document["pages"].as_array().unwrap().len(), 1);
    assert!(document["externs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|name| name == "status"));
    assert_eq!(document["state"]["status"], "question");
    assert_eq!(document["meta"]["page"]["width"], 1920);
    assert_eq!(document["meta"]["page"]["height"], 1080);
}

#[test]
fn custom_screens_may_declare_any_subset_of_the_contract() {
    let check = screens::check_present(
        "@extern question = \"Exemplo\"\n@data example\n| name | points |\n| --- | --- |\n| Ana | 1 |\n@end\n@extern scoreboard = $example\n\n# $question\n::: for person in $scoreboard\n$person.name\n:::",
    );
    assert!(check.is_valid(), "{:#?}", check.diagnostics);
}

#[test]
fn custom_screens_report_compile_errors() {
    let check = screens::check_present("# Título {colr=red}");
    assert!(!check.is_valid());
    assert_eq!(check.diagnostics[0].code, "R209");
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
    let check = screens::check_present("@extern round = \"três\"\n@data example\n| name | points |\n| --- | --- |\n| Ana | muitos |\n@end\n@extern scoreboard = $example\n\n# $round");
    let codes: Vec<&str> = check.diagnostics.iter().map(|d| d.code.as_str()).collect();
    assert_eq!(codes, vec!["HUB2", "HUB2"]);
    assert!(check.diagnostics[0].message.contains("número"));
    assert!(check.diagnostics[1].message.contains("points"));
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
    assert_eq!(report["document"]["pages"].as_array().unwrap().len(), 1);
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
        assert_eq!(check.document.externs, vec![suggestion.name.to_owned()]);
        assert!(!suggestion.detail.is_empty());
    }
}

#[tokio::test]
async fn contract_route_lists_the_suggestions() {
    let (status, body) = send(get("/api/v1/screens/present/contract")).await;
    assert_eq!(status, StatusCode::OK);
    let list = json(&body);
    let scoreboard = list
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["name"] == "scoreboard")
        .unwrap();
    assert!(scoreboard["declaration"]
        .as_str()
        .unwrap()
        .starts_with("@data example_scoreboard"));
    assert_eq!(scoreboard["kind"], "lista de registros");
    assert_eq!(scoreboard["fields"][1]["name"], "points");
    let status = list
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["name"] == "status")
        .unwrap();
    assert_eq!(status["kind"], "texto");
    assert!(status.get("fields").is_none());
}

#[test]
fn every_template_compiles_and_follows_the_contract() {
    let mut ids: Vec<&str> = screens::PRESENT_TEMPLATES.iter().map(|t| t.id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), screens::PRESENT_TEMPLATES.len());
    for template in screens::PRESENT_TEMPLATES {
        let check = screens::check_present(template.source);
        assert!(
            check.diagnostics.is_empty(),
            "{}: {:#?}",
            template.id,
            check.diagnostics
        );
        assert_eq!(check.document.pages.len(), 1, "{}", template.id);
    }
}

#[tokio::test]
async fn templates_route_lists_every_template_with_its_document() {
    let (status, body) = send(get("/api/v1/screens/present/templates")).await;
    assert_eq!(status, StatusCode::OK);
    let list = json(&body);
    let list = list.as_array().unwrap();
    assert_eq!(list.len(), 5);
    assert_eq!(list[0]["id"], "classic");
    for template in list {
        assert!(!template["source"].as_str().unwrap().is_empty());
        assert_eq!(template["document"]["pages"].as_array().unwrap().len(), 1);
    }
}
