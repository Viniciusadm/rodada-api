//! Demo (filler) seeds: a sample organization, admin, teams, players, question
//! bank, one finished match and notifications. These exist only to populate a
//! local/demo environment and are gated behind the `DEMO_SEEDS` flag. They are
//! NOT required for the product to work — real orgs/users come from sign-up.
//!
//! Depends on the catalog seeds having run first (the demo match references
//! [`GAME_GAMER_QUIZ_ID`] / [`MODE_GAMER_QUIZ_CLASSIC_ID`]).

use sqlx::MySqlPool;

use crate::application::auth::PasswordHasher;
use crate::errors::AppError;
use crate::infrastructure::auth::Argon2PasswordHasher;

use super::catalog::{GAME_GAMER_QUIZ_ID, MODE_GAMER_QUIZ_CLASSIC_ID};

const DEMO_ORG_ID: &str = "10000000-0000-0000-0000-000000000001";
const DEMO_ADMIN_USER_ID: &str = "10000000-0000-0000-0000-000000000002";
const DEMO_ADMIN_ROLE_ID: &str = "10000000-0000-0000-0000-000000000003";
const DEMO_TEAM_ALPHA_ID: &str = "10000000-0000-0000-0000-000000000004";
const DEMO_TEAM_PIXEL_ID: &str = "10000000-0000-0000-0000-000000000005";
const PLAYER_ANA_ID: &str = "30000000-0000-0000-0000-000000000001";
const PLAYER_LUCAS_ID: &str = "30000000-0000-0000-0000-000000000002";
const PLAYER_BIA_ID: &str = "30000000-0000-0000-0000-000000000003";
const QUESTION_BANK_ID: &str = "40000000-0000-0000-0000-000000000001";
const DEMO_MATCH_ID: &str = "50000000-0000-0000-0000-000000000001";
const PART_ANA_ID: &str = "51000000-0000-0000-0000-000000000001";
const PART_LUCAS_ID: &str = "51000000-0000-0000-0000-000000000002";
const PART_BIA_ID: &str = "51000000-0000-0000-0000-000000000003";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemoSeedConfig {
    pub admin_email: String,
    pub admin_password: String,
}

/// Seeds demonstration data. Requires the catalog seeds to have run first.
pub async fn run_demo_seeds(pool: &MySqlPool, config: DemoSeedConfig) -> Result<(), AppError> {
    seed_identity(pool, &config).await?;
    seed_players(pool).await?;
    seed_questions(pool).await?;
    seed_demo_match(pool).await?;
    seed_notifications(pool).await?;
    Ok(())
}

async fn seed_identity(pool: &MySqlPool, config: &DemoSeedConfig) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO organizations (id, name, slug, contact_email) VALUES (?, ?, ?, ?)
         ON DUPLICATE KEY UPDATE name = VALUES(name), contact_email = VALUES(contact_email)",
    )
    .bind(DEMO_ORG_ID)
    .bind("Games Hub Demo")
    .bind("games-hub-demo")
    .bind("contato@gameshub.local")
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
    .bind(DEMO_TEAM_ALPHA_ID)
    .bind(DEMO_ORG_ID)
    .bind("Time Alpha")
    .bind("time-alpha")
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO ministries (id, organization_id, name, slug, is_active) VALUES (?, ?, ?, ?, TRUE)
         ON DUPLICATE KEY UPDATE name = VALUES(name), is_active = TRUE",
    )
    .bind(DEMO_TEAM_PIXEL_ID)
    .bind(DEMO_ORG_ID)
    .bind("Time Pixel")
    .bind("time-pixel")
    .execute(pool)
    .await?;

    Ok(())
}

async fn seed_players(pool: &MySqlPool) -> Result<(), AppError> {
    upsert_player(pool, PLAYER_ANA_ID, DEMO_TEAM_ALPHA_ID, "Ana Souza").await?;
    upsert_player(pool, PLAYER_LUCAS_ID, DEMO_TEAM_ALPHA_ID, "Lucas Lima").await?;
    upsert_player(pool, PLAYER_BIA_ID, DEMO_TEAM_PIXEL_ID, "Bia Silva").await?;
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
    .bind(GAME_GAMER_QUIZ_ID)
    .bind("Perguntas Demo")
    .bind("Banco inicial para navegar no ambiente local.")
    .bind(DEMO_ADMIN_USER_ID)
    .execute(pool)
    .await?;

    upsert_question(
        pool,
        "40000000-0000-0000-0000-000000000101",
        "Qual console popularizou controles com sensor de movimento?",
        "Nintendo Wii",
        100,
        "easy",
    )
    .await?;
    upsert_question(
        pool,
        "40000000-0000-0000-0000-000000000102",
        "Qual genero usa fases, chefes e progressao por niveis?",
        "Plataforma",
        100,
        "easy",
    )
    .await?;
    upsert_question(
        pool,
        "40000000-0000-0000-0000-000000000103",
        "Qual item costuma recuperar vida em jogos de aventura?",
        "Pocao",
        100,
        "easy",
    )
    .await?;
    upsert_question(
        pool,
        "40000000-0000-0000-0000-000000000104",
        "Qual modo coloca equipes competindo pelo maior placar?",
        "Competitivo",
        100,
        "easy",
    )
    .await?;
    upsert_question(
        pool,
        "40000000-0000-0000-0000-000000000105",
        "Qual recurso permite jogar com pessoas online em tempo real?",
        "Multiplayer",
        150,
        "medium",
    )
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

/// Seeds one finished match with participants, rounds and score events so that the
/// dashboard, ranking, history and player stats show real data out of the box.
async fn seed_demo_match(pool: &MySqlPool) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO matches (id, organization_id, game_id, game_mode_id, host_user_id, name, status, started_at, ended_at)
         VALUES (?, ?, ?, ?, ?, ?, 'finished', DATE_SUB(NOW(6), INTERVAL 2 HOUR), DATE_SUB(NOW(6), INTERVAL 1 HOUR))
         ON DUPLICATE KEY UPDATE name = VALUES(name), status = VALUES(status), started_at = VALUES(started_at), ended_at = VALUES(ended_at)",
    )
    .bind(DEMO_MATCH_ID)
    .bind(DEMO_ORG_ID)
    .bind(GAME_GAMER_QUIZ_ID)
    .bind(MODE_GAMER_QUIZ_CLASSIC_ID)
    .bind(DEMO_ADMIN_USER_ID)
    .bind("Quiz Gamer - Noite da Comunidade")
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO match_settings (match_id, max_rounds, time_per_round_seconds, initial_points, remote_enabled, capacity)
         VALUES (?, 5, 30, 0, TRUE, 20)
         ON DUPLICATE KEY UPDATE max_rounds = VALUES(max_rounds), time_per_round_seconds = VALUES(time_per_round_seconds), capacity = VALUES(capacity)",
    )
    .bind(DEMO_MATCH_ID)
    .execute(pool)
    .await?;

    let questions = [
        (
            "52000000-0000-0000-0000-000000000001",
            1,
            "Qual console popularizou controles com sensor de movimento?",
            "Nintendo Wii",
        ),
        (
            "52000000-0000-0000-0000-000000000002",
            2,
            "Qual genero usa fases, chefes e progressao por niveis?",
            "Plataforma",
        ),
        (
            "52000000-0000-0000-0000-000000000003",
            3,
            "Qual item costuma recuperar vida em jogos de aventura?",
            "Pocao",
        ),
    ];
    for (id, position, prompt, answer) in questions {
        upsert_match_question(pool, id, position, prompt, answer).await?;
    }

    upsert_match_participant(pool, PART_ANA_ID, PLAYER_ANA_ID, "Ana Souza", 300).await?;
    upsert_match_participant(pool, PART_LUCAS_ID, PLAYER_LUCAS_ID, "Lucas Lima", 200).await?;
    upsert_match_participant(pool, PART_BIA_ID, PLAYER_BIA_ID, "Bia Silva", 100).await?;

    let rounds = [
        (
            "53000000-0000-0000-0000-000000000001",
            "52000000-0000-0000-0000-000000000001",
            1,
            PART_ANA_ID,
        ),
        (
            "53000000-0000-0000-0000-000000000002",
            "52000000-0000-0000-0000-000000000002",
            2,
            PART_LUCAS_ID,
        ),
        (
            "53000000-0000-0000-0000-000000000003",
            "52000000-0000-0000-0000-000000000003",
            3,
            PART_ANA_ID,
        ),
    ];
    for (id, question_id, number, selected) in rounds {
        upsert_match_round(pool, id, question_id, number, selected).await?;
    }

    // Ana wins with three correct answers, Lucas two, Bia one.
    let score_events = [
        (
            "54000000-0000-0000-0000-000000000001",
            PART_ANA_ID,
            "53000000-0000-0000-0000-000000000001",
            100,
            100,
        ),
        (
            "54000000-0000-0000-0000-000000000002",
            PART_LUCAS_ID,
            "53000000-0000-0000-0000-000000000001",
            100,
            100,
        ),
        (
            "54000000-0000-0000-0000-000000000003",
            PART_BIA_ID,
            "53000000-0000-0000-0000-000000000001",
            100,
            100,
        ),
        (
            "54000000-0000-0000-0000-000000000004",
            PART_ANA_ID,
            "53000000-0000-0000-0000-000000000002",
            100,
            200,
        ),
        (
            "54000000-0000-0000-0000-000000000005",
            PART_LUCAS_ID,
            "53000000-0000-0000-0000-000000000002",
            100,
            200,
        ),
        (
            "54000000-0000-0000-0000-000000000006",
            PART_ANA_ID,
            "53000000-0000-0000-0000-000000000003",
            100,
            300,
        ),
    ];
    for (id, participant_id, round_id, delta, score_after) in score_events {
        upsert_score_event(pool, id, participant_id, round_id, delta, score_after).await?;
    }

    sqlx::query("UPDATE matches SET winner_participant_id = ? WHERE id = ?")
        .bind(PART_ANA_ID)
        .bind(DEMO_MATCH_ID)
        .execute(pool)
        .await?;

    Ok(())
}

async fn upsert_match_question(
    pool: &MySqlPool,
    id: &str,
    position: i32,
    prompt: &str,
    answer: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO match_questions (id, match_id, source_question_id, position, prompt_snapshot, expected_answer_snapshot, points, used_at)
         VALUES (?, ?, NULL, ?, ?, ?, 100, NOW(6))
         ON DUPLICATE KEY UPDATE prompt_snapshot = VALUES(prompt_snapshot), expected_answer_snapshot = VALUES(expected_answer_snapshot)",
    )
    .bind(id)
    .bind(DEMO_MATCH_ID)
    .bind(position)
    .bind(prompt)
    .bind(answer)
    .execute(pool)
    .await?;
    Ok(())
}

async fn upsert_match_participant(
    pool: &MySqlPool,
    id: &str,
    player_id: &str,
    display_name: &str,
    score: i32,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO match_participants (id, match_id, player_id, display_name_snapshot, normalized_name_snapshot, join_source, current_score)
         VALUES (?, ?, ?, ?, ?, 'local', ?)
         ON DUPLICATE KEY UPDATE player_id = VALUES(player_id), display_name_snapshot = VALUES(display_name_snapshot), current_score = VALUES(current_score)",
    )
    .bind(id)
    .bind(DEMO_MATCH_ID)
    .bind(player_id)
    .bind(display_name)
    .bind(display_name.trim().to_ascii_lowercase())
    .bind(score)
    .execute(pool)
    .await?;
    Ok(())
}

async fn upsert_match_round(
    pool: &MySqlPool,
    id: &str,
    question_id: &str,
    round_number: i32,
    selected_participant_id: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO match_rounds (id, match_id, match_question_id, round_number, status, selected_participant_id, answer_revealed_at, closed_at)
         VALUES (?, ?, ?, ?, 'closed', ?, NOW(6), NOW(6))
         ON DUPLICATE KEY UPDATE status = VALUES(status), selected_participant_id = VALUES(selected_participant_id)",
    )
    .bind(id)
    .bind(DEMO_MATCH_ID)
    .bind(question_id)
    .bind(round_number)
    .bind(selected_participant_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn upsert_score_event(
    pool: &MySqlPool,
    id: &str,
    participant_id: &str,
    round_id: &str,
    delta: i32,
    score_after: i32,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO score_events (id, match_id, participant_id, match_round_id, actor_user_id, event_type, delta, score_after, reason)
         VALUES (?, ?, ?, ?, ?, 'correct_answer', ?, ?, NULL)
         ON DUPLICATE KEY UPDATE delta = VALUES(delta), score_after = VALUES(score_after)",
    )
    .bind(id)
    .bind(DEMO_MATCH_ID)
    .bind(participant_id)
    .bind(round_id)
    .bind(DEMO_ADMIN_USER_ID)
    .bind(delta)
    .bind(score_after)
    .execute(pool)
    .await?;
    Ok(())
}

async fn seed_notifications(pool: &MySqlPool) -> Result<(), AppError> {
    let preferences = [
        ("60000000-0000-0000-0000-000000000001", "new_rooms"),
        ("60000000-0000-0000-0000-000000000002", "ranking_updates"),
    ];
    for (id, key) in preferences {
        sqlx::query(
            "INSERT INTO notification_preferences (id, organization_id, user_id, preference_key, enabled)
             VALUES (?, ?, ?, ?, TRUE)
             ON DUPLICATE KEY UPDATE enabled = VALUES(enabled)",
        )
        .bind(id)
        .bind(DEMO_ORG_ID)
        .bind(DEMO_ADMIN_USER_ID)
        .bind(key)
        .execute(pool)
        .await?;
    }

    let notifications = [
        (
            "61000000-0000-0000-0000-000000000001",
            "ranking_updates",
            "Ranking atualizado",
            "Ana Souza assumiu a lideranca do ranking.",
            "/ranking",
        ),
        (
            "61000000-0000-0000-0000-000000000002",
            "new_rooms",
            "Bem-vindo ao Games Hub",
            "Explore os jogos e crie sua primeira partida.",
            "/games",
        ),
    ];
    for (id, kind, title, body, link) in notifications {
        sqlx::query(
            "INSERT INTO notifications (id, organization_id, user_id, kind, title, body, link)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON DUPLICATE KEY UPDATE title = VALUES(title), body = VALUES(body), link = VALUES(link)",
        )
        .bind(id)
        .bind(DEMO_ORG_ID)
        .bind(DEMO_ADMIN_USER_ID)
        .bind(kind)
        .bind(title)
        .bind(body)
        .bind(link)
        .execute(pool)
        .await?;
    }

    Ok(())
}
