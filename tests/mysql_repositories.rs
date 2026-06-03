use holyplay_backend::application::players::{CreatePlayerCommand, PlayerRepository};
use holyplay_backend::domain::games::{McOption, QuestionType};
use holyplay_backend::infrastructure::repositories::mysql_players::MySqlPlayerRepository;
use sqlx::MySqlPool;

async fn migrated_pool() -> MySqlPool {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must point to a disposable MySQL database");
    let pool = MySqlPool::connect(&database_url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

#[test]
fn grade_judges_each_question_type() {
    assert!(QuestionType::TrueFalse.grade("true", None, "verdadeiro"));
    assert!(!QuestionType::TrueFalse.grade("true", None, "falso"));

    assert!(QuestionType::FreeText.grade("Poção|pocao", None, "POÇÃO"));
    assert!(!QuestionType::FreeText.grade("Poção", None, "espada"));

    let options = [
        McOption {
            text: "Paris".to_owned(),
            correct: true,
        },
        McOption {
            text: "Londres".to_owned(),
            correct: false,
        },
    ];
    assert!(QuestionType::MultipleChoice.grade("Paris", Some(&options), "0"));
    assert!(!QuestionType::MultipleChoice.grade("Paris", Some(&options), "1"));
    assert!(!QuestionType::MultipleChoice.grade("Paris", Some(&options), "5"));
    assert!(!QuestionType::MultipleChoice.grade("Paris", None, "0"));

    assert!(!QuestionType::Buzzer.grade("x", None, "x"));
}

#[tokio::test]
#[ignore = "requires a disposable MySQL database in DATABASE_URL"]
async fn player_repository_creates_and_lists_by_owner() {
    let pool = migrated_pool().await;
    let player_repo = MySqlPlayerRepository::new(pool.clone());
    let owner_user_id = "30000000-0000-0000-0000-000000000001";
    let player_id = "30000000-0000-0000-0000-000000000002";

    // The roster owner must exist (players.owner_user_id -> users.id).
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, display_name, is_active) VALUES (?, ?, ?, ?, TRUE)",
    )
    .bind(owner_user_id)
    .bind("owner@example.com")
    .bind("hash")
    .bind("Owner Teste")
    .execute(&pool)
    .await
    .unwrap();

    player_repo
        .create(
            player_id,
            &CreatePlayerCommand {
                owner_user_id: owner_user_id.to_owned(),
                user_id: None,
                display_name: "Ana Teste".to_owned(),
            },
        )
        .await
        .unwrap();

    let players = player_repo.list_by_owner(owner_user_id).await.unwrap();
    assert!(players.iter().any(|player| player.id.as_str() == player_id));
}
