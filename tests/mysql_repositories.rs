use holyplay_backend::application::games::{CreateGameCommand, GameRepository};
use holyplay_backend::application::organizations::{
    CreateOrganizationCommand, OrganizationRepository,
};
use holyplay_backend::application::players::{CreatePlayerCommand, PlayerRepository};
use holyplay_backend::infrastructure::repositories::mysql_games::MySqlGameRepository;
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
            name: "Igreja Teste".to_owned(),
            slug: "igreja-teste".to_owned(),
            contact_email: Some("contato@example.com".to_owned()),
            owner_user_id: None,
        },
    )
    .await
    .unwrap();

    let organization = repo.find_by_id(id).await.unwrap().unwrap();
    assert_eq!(organization.id.as_str(), id);
    assert_eq!(organization.name.as_str(), "Igreja Teste");
}

#[tokio::test]
#[ignore = "requires a disposable MySQL database in DATABASE_URL"]
async fn game_repository_lists_active_games() {
    let pool = migrated_pool().await;
    let repo = MySqlGameRepository::new(pool);
    let id = "20000000-0000-0000-0000-000000000001";

    repo.create_game(
        id,
        &CreateGameCommand {
            organization_id: None,
            slug: "quiz-teste".to_owned(),
            name: "Quiz Teste".to_owned(),
            description: "Jogo de teste".to_owned(),
            status: "active".to_owned(),
            icon_key: Some("book".to_owned()),
            sort_order: 0,
        },
    )
    .await
    .unwrap();

    let games = repo.list_active_games(None).await.unwrap();
    assert!(games.iter().any(|game| game.id.as_str() == id));
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
                name: "Igreja Jogadores".to_owned(),
                slug: "igreja-jogadores".to_owned(),
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
