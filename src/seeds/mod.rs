use sqlx::MySqlPool;

use crate::application::auth::PasswordHasher;
use crate::errors::AppError;
use crate::infrastructure::auth::Argon2PasswordHasher;

const DEMO_ORG_ID: &str = "10000000-0000-0000-0000-000000000001";
const DEMO_ADMIN_USER_ID: &str = "10000000-0000-0000-0000-000000000002";
const DEMO_ADMIN_ROLE_ID: &str = "10000000-0000-0000-0000-000000000003";
const DEMO_MINISTRY_YOUTH_ID: &str = "10000000-0000-0000-0000-000000000004";
const DEMO_MINISTRY_EBD_ID: &str = "10000000-0000-0000-0000-000000000005";
const GAME_BIBLE_QUIZ_ID: &str = "20000000-0000-0000-0000-000000000001";
const GAME_VERSE_COMPLETE_ID: &str = "20000000-0000-0000-0000-000000000002";
const GAME_TRUE_FALSE_ID: &str = "20000000-0000-0000-0000-000000000003";
const MODE_BIBLE_QUIZ_CLASSIC_ID: &str = "21000000-0000-0000-0000-000000000001";
const MODE_BIBLE_QUIZ_POINTS_ID: &str = "21000000-0000-0000-0000-000000000002";
const MODE_VERSE_CLASSIC_ID: &str = "21000000-0000-0000-0000-000000000003";
const MODE_TRUE_FALSE_CLASSIC_ID: &str = "21000000-0000-0000-0000-000000000004";
const PLAYER_ANA_ID: &str = "30000000-0000-0000-0000-000000000001";
const PLAYER_LUCAS_ID: &str = "30000000-0000-0000-0000-000000000002";
const PLAYER_MARIA_ID: &str = "30000000-0000-0000-0000-000000000003";
const QUESTION_BANK_ID: &str = "40000000-0000-0000-0000-000000000001";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemoSeedConfig {
    pub admin_email: String,
    pub admin_password: String,
}

pub async fn run_demo_seeds(pool: &MySqlPool, config: DemoSeedConfig) -> Result<(), AppError> {
    seed_identity(pool, &config).await?;
    seed_games(pool).await?;
    seed_players(pool).await?;
    seed_questions(pool).await?;
    Ok(())
}

async fn seed_identity(pool: &MySqlPool, config: &DemoSeedConfig) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO organizations (id, name, slug, contact_email) VALUES (?, ?, ?, ?)
         ON DUPLICATE KEY UPDATE name = VALUES(name), contact_email = VALUES(contact_email)",
    )
    .bind(DEMO_ORG_ID)
    .bind("Igreja Demo HolyPlay")
    .bind("igreja-demo")
    .bind("contato@holyplay.local")
    .execute(pool)
    .await?;

    let password_hash = Argon2PasswordHasher.hash_password(&config.admin_password)?;
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, display_name, is_active) VALUES (?, ?, ?, ?, TRUE)
         ON DUPLICATE KEY UPDATE password_hash = VALUES(password_hash), display_name = VALUES(display_name), is_active = TRUE",
    )
    .bind(DEMO_ADMIN_USER_ID)
    .bind(&config.admin_email)
    .bind(password_hash)
    .bind("Admin Demo")
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO user_roles (id, organization_id, user_id, role) VALUES (?, ?, ?, 'owner')
         ON DUPLICATE KEY UPDATE role = VALUES(role)",
    )
    .bind(DEMO_ADMIN_ROLE_ID)
    .bind(DEMO_ORG_ID)
    .bind(DEMO_ADMIN_USER_ID)
    .execute(pool)
    .await?;

    sqlx::query("UPDATE organizations SET owner_user_id = ? WHERE id = ?")
        .bind(DEMO_ADMIN_USER_ID)
        .bind(DEMO_ORG_ID)
        .execute(pool)
        .await?;

    sqlx::query(
        "INSERT INTO ministries (id, organization_id, name, slug, is_active) VALUES (?, ?, ?, ?, TRUE)
         ON DUPLICATE KEY UPDATE name = VALUES(name), is_active = TRUE",
    )
    .bind(DEMO_MINISTRY_YOUTH_ID)
    .bind(DEMO_ORG_ID)
    .bind("Jovens")
    .bind("jovens")
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO ministries (id, organization_id, name, slug, is_active) VALUES (?, ?, ?, ?, TRUE)
         ON DUPLICATE KEY UPDATE name = VALUES(name), is_active = TRUE",
    )
    .bind(DEMO_MINISTRY_EBD_ID)
    .bind(DEMO_ORG_ID)
    .bind("EBD")
    .bind("ebd")
    .execute(pool)
    .await?;

    Ok(())
}

async fn seed_games(pool: &MySqlPool) -> Result<(), AppError> {
    upsert_game(
        pool,
        GAME_BIBLE_QUIZ_ID,
        "quiz-biblico",
        "Quiz Biblico",
        "Perguntas e respostas para grupos, celulas e escolas biblicas.",
        "active",
        "book-open",
        10,
    )
    .await?;
    upsert_game(
        pool,
        GAME_VERSE_COMPLETE_ID,
        "versiculo-completo",
        "Versiculo Completo",
        "Complete passagens biblicas com agilidade e precisao.",
        "active",
        "quote",
        20,
    )
    .await?;
    upsert_game(
        pool,
        GAME_TRUE_FALSE_ID,
        "verdadeiro-falso",
        "Verdadeiro ou Falso",
        "Rodadas rapidas para revisar conteudo biblico.",
        "beta",
        "check-circle",
        30,
    )
    .await?;

    upsert_mode(
        pool,
        MODE_BIBLE_QUIZ_CLASSIC_ID,
        GAME_BIBLE_QUIZ_ID,
        "classic",
        "Classico",
        "Pontuacao simples por resposta correta.",
        Some(0),
        Some(30),
    )
    .await?;
    upsert_mode(
        pool,
        MODE_BIBLE_QUIZ_POINTS_ID,
        GAME_BIBLE_QUIZ_ID,
        "initial_points",
        "Pontos Iniciais",
        "Todos comecam com pontos e podem ganhar ou perder ao responder.",
        Some(100),
        Some(30),
    )
    .await?;
    upsert_mode(
        pool,
        MODE_VERSE_CLASSIC_ID,
        GAME_VERSE_COMPLETE_ID,
        "classic",
        "Classico",
        "Complete o versiculo sorteado.",
        Some(0),
        Some(45),
    )
    .await?;
    upsert_mode(
        pool,
        MODE_TRUE_FALSE_CLASSIC_ID,
        GAME_TRUE_FALSE_ID,
        "classic",
        "Classico",
        "Responda verdadeiro ou falso.",
        Some(0),
        Some(20),
    )
    .await?;

    Ok(())
}

async fn seed_players(pool: &MySqlPool) -> Result<(), AppError> {
    upsert_player(pool, PLAYER_ANA_ID, DEMO_MINISTRY_YOUTH_ID, "Ana Souza").await?;
    upsert_player(pool, PLAYER_LUCAS_ID, DEMO_MINISTRY_YOUTH_ID, "Lucas Lima").await?;
    upsert_player(pool, PLAYER_MARIA_ID, DEMO_MINISTRY_EBD_ID, "Maria Santos").await?;
    Ok(())
}

async fn seed_questions(pool: &MySqlPool) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO question_banks (id, organization_id, game_id, name, description, created_by_user_id)
         VALUES (?, ?, ?, ?, ?, ?)
         ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description)",
    )
    .bind(QUESTION_BANK_ID)
    .bind(DEMO_ORG_ID)
    .bind(GAME_BIBLE_QUIZ_ID)
    .bind("Perguntas Demo")
    .bind("Banco inicial para navegar no ambiente local.")
    .bind(DEMO_ADMIN_USER_ID)
    .execute(pool)
    .await?;

    upsert_question(
        pool,
        "40000000-0000-0000-0000-000000000101",
        "Quem construiu a arca?",
        "Noe",
        100,
        "easy",
    )
    .await?;
    upsert_question(
        pool,
        "40000000-0000-0000-0000-000000000102",
        "Quantos discipulos Jesus escolheu?",
        "Doze",
        100,
        "easy",
    )
    .await?;
    upsert_question(
        pool,
        "40000000-0000-0000-0000-000000000103",
        "Qual jovem derrotou Golias?",
        "Davi",
        100,
        "easy",
    )
    .await?;
    upsert_question(
        pool,
        "40000000-0000-0000-0000-000000000104",
        "Em qual cidade Jesus nasceu?",
        "Belem",
        100,
        "easy",
    )
    .await?;
    upsert_question(
        pool,
        "40000000-0000-0000-0000-000000000105",
        "Qual apostolo escreveu a carta aos Romanos?",
        "Paulo",
        150,
        "medium",
    )
    .await?;

    Ok(())
}

async fn upsert_game(
    pool: &MySqlPool,
    id: &str,
    slug: &str,
    name: &str,
    description: &str,
    status: &str,
    icon_key: &str,
    sort_order: i32,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO games (id, organization_id, slug, name, description, status, icon_key, sort_order)
         VALUES (?, NULL, ?, ?, ?, ?, ?, ?)
         ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), status = VALUES(status), icon_key = VALUES(icon_key), sort_order = VALUES(sort_order)",
    )
    .bind(id)
    .bind(slug)
    .bind(name)
    .bind(description)
    .bind(status)
    .bind(icon_key)
    .bind(sort_order)
    .execute(pool)
    .await?;
    Ok(())
}

async fn upsert_mode(
    pool: &MySqlPool,
    id: &str,
    game_id: &str,
    code: &str,
    name: &str,
    description: &str,
    initial_points: Option<i32>,
    time_seconds: Option<i32>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO game_modes (id, game_id, code, name, description, default_initial_points, default_time_per_question_seconds, is_active)
         VALUES (?, ?, ?, ?, ?, ?, ?, TRUE)
         ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), default_initial_points = VALUES(default_initial_points), default_time_per_question_seconds = VALUES(default_time_per_question_seconds), is_active = TRUE",
    )
    .bind(id)
    .bind(game_id)
    .bind(code)
    .bind(name)
    .bind(description)
    .bind(initial_points)
    .bind(time_seconds)
    .execute(pool)
    .await?;
    Ok(())
}

async fn upsert_player(
    pool: &MySqlPool,
    id: &str,
    ministry_id: &str,
    display_name: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO players (id, organization_id, user_id, ministry_id, display_name, normalized_name, is_active)
         VALUES (?, ?, NULL, ?, ?, ?, TRUE)
         ON DUPLICATE KEY UPDATE ministry_id = VALUES(ministry_id), display_name = VALUES(display_name), normalized_name = VALUES(normalized_name), is_active = TRUE",
    )
    .bind(id)
    .bind(DEMO_ORG_ID)
    .bind(ministry_id)
    .bind(display_name)
    .bind(display_name.trim().to_ascii_lowercase())
    .execute(pool)
    .await?;
    Ok(())
}

async fn upsert_question(
    pool: &MySqlPool,
    id: &str,
    prompt: &str,
    expected_answer: &str,
    points: i32,
    difficulty: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO questions (id, question_bank_id, prompt, expected_answer, default_points, difficulty, is_active)
         VALUES (?, ?, ?, ?, ?, ?, TRUE)
         ON DUPLICATE KEY UPDATE prompt = VALUES(prompt), expected_answer = VALUES(expected_answer), default_points = VALUES(default_points), difficulty = VALUES(difficulty), is_active = TRUE",
    )
    .bind(id)
    .bind(QUESTION_BANK_ID)
    .bind(prompt)
    .bind(expected_answer)
    .bind(points)
    .bind(difficulty)
    .execute(pool)
    .await?;
    Ok(())
}
