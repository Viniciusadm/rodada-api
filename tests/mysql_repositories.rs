use holyplay_backend::application::organizations::{
    CreateOrganizationCommand, OrganizationRepository,
};
use holyplay_backend::application::players::{CreatePlayerCommand, PlayerRepository};
use holyplay_backend::domain::games::{catalog, GameKind};
use holyplay_backend::infrastructure::repositories::mysql_organizations::MySqlOrganizationRepository;
use holyplay_backend::infrastructure::repositories::mysql_players::MySqlPlayerRepository;
use sqlx::MySqlPool;

async fn migrated_pool() -> MySqlPool {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must point to a disposable MySQL database");
    let pool = MySqlPool::connect(&database_url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

#[tokio::test]
#[ignore = "requires a disposable MySQL database in DATABASE_URL"]
async fn organization_repository_creates_and_reads() {
    let pool = migrated_pool().await;
    let repo = MySqlOrganizationRepository::new(pool);
    let id = "10000000-0000-0000-0000-000000000001";

    repo.create(
        id,
        &CreateOrganizationCommand {
            name: "Comunidade Teste".to_owned(),
            slug: "comunidade-teste".to_owned(),
            contact_email: Some("contato@example.com".to_owned()),
            owner_user_id: None,
        },
    )
    .await
    .unwrap();

    let organization = repo.find_by_id(id).await.unwrap().unwrap();
    assert_eq!(organization.id.as_str(), id);
    assert_eq!(organization.name.as_str(), "Comunidade Teste");
}

#[test]
fn catalog_defines_the_three_games_with_kinds() {
    // The catalog is static product data in code (no DB), so this needs no pool.
    let games: Vec<_> = catalog::active_games().collect();
    assert_eq!(games.len(), 3);
    assert_eq!(
        catalog::game_kind(catalog::GAME_GAMER_QUIZ_ID),
        Some(GameKind::Buzzer)
    );
    assert_eq!(
        catalog::game_kind(catalog::GAME_TRUE_FALSE_ID),
        Some(GameKind::TrueFalse)
    );
    assert_eq!(
        catalog::game_kind(catalog::GAME_FREE_TEXT_ID),
        Some(GameKind::FreeText)
    );
    // Quiz Gamer offers all three scoring modes; the others only classic.
    assert_eq!(
        catalog::mode_code(catalog::MODE_GAMER_QUIZ_CLASSIC_ID),
        Some("classic")
    );
}

#[test]
fn grade_judges_true_false_and_free_text() {
    assert!(GameKind::TrueFalse.grade("true", "verdadeiro"));
    assert!(!GameKind::TrueFalse.grade("true", "falso"));
    // Accent/case-insensitive, accepts pipe-separated variants.
    assert!(GameKind::FreeText.grade("Poção|pocao", "POÇÃO"));
    assert!(!GameKind::FreeText.grade("Poção", "espada"));
    // Buzzer is host-judged, never auto-correct.
    assert!(!GameKind::Buzzer.grade("x", "x"));
}

#[tokio::test]
#[ignore = "requires a disposable MySQL database in DATABASE_URL"]
async fn player_repository_creates_and_lists_by_organization() {
    let pool = migrated_pool().await;
    let org_repo = MySqlOrganizationRepository::new(pool.clone());
    let player_repo = MySqlPlayerRepository::new(pool);
    let organization_id = "30000000-0000-0000-0000-000000000001";
    let player_id = "30000000-0000-0000-0000-000000000002";

    org_repo
        .create(
            organization_id,
            &CreateOrganizationCommand {
                name: "Arena Jogadores".to_owned(),
                slug: "arena-jogadores".to_owned(),
                contact_email: None,
                owner_user_id: None,
            },
        )
        .await
        .unwrap();

    player_repo
        .create(
            player_id,
            &CreatePlayerCommand {
                organization_id: organization_id.to_owned(),
                user_id: None,
                ministry_id: None,
                display_name: "Ana Teste".to_owned(),
            },
        )
        .await
        .unwrap();

    let players = player_repo
        .list_by_organization(organization_id)
        .await
        .unwrap();
    assert!(players.iter().any(|player| player.id.as_str() == player_id));
}
