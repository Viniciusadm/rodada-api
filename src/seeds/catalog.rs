//! Required catalog seeds: the global games and game modes the product needs to
//! function. These rows have `organization_id = NULL` and are idempotent upserts,
//! so they are safe to run on every startup (including production). Without them
//! match creation breaks (the frontend has no game to pick).

use sqlx::MySqlPool;

use crate::errors::AppError;

pub(super) const GAME_GAMER_QUIZ_ID: &str = "20000000-0000-0000-0000-000000000001";
pub(super) const GAME_WORD_COMPLETE_ID: &str = "20000000-0000-0000-0000-000000000002";
pub(super) const GAME_TRUE_FALSE_ID: &str = "20000000-0000-0000-0000-000000000003";
pub(super) const MODE_GAMER_QUIZ_CLASSIC_ID: &str = "21000000-0000-0000-0000-000000000001";
const MODE_GAMER_QUIZ_POINTS_ID: &str = "21000000-0000-0000-0000-000000000002";
const MODE_WORD_CLASSIC_ID: &str = "21000000-0000-0000-0000-000000000003";
const MODE_TRUE_FALSE_CLASSIC_ID: &str = "21000000-0000-0000-0000-000000000004";
const MODE_GAMER_QUIZ_ELIMINATION_ID: &str = "21000000-0000-0000-0000-000000000005";

/// Seeds the global games/modes catalog. Idempotent; runs on every startup.
pub async fn run_catalog_seeds(pool: &MySqlPool) -> Result<(), AppError> {
    upsert_game(
        pool,
        GameSeed {
            id: GAME_GAMER_QUIZ_ID,
            slug: "quiz-gamer",
            name: "Quiz Gamer",
            description: "Perguntas e respostas para grupos e comunidades gamer.",
            status: "active",
            icon_key: "quiz",
            sort_order: 10,
        },
    )
    .await?;
    upsert_game(
        pool,
        GameSeed {
            id: GAME_WORD_COMPLETE_ID,
            slug: "complete-a-palavra",
            name: "Complete a Palavra",
            description: "Complete termos do universo de jogos com agilidade e precisao.",
            status: "active",
            icon_key: "word",
            sort_order: 20,
        },
    )
    .await?;
    upsert_game(
        pool,
        GameSeed {
            id: GAME_TRUE_FALSE_ID,
            slug: "verdadeiro-falso",
            name: "Verdadeiro ou Falso",
            description: "Rodadas rapidas para revisar curiosidades de jogos.",
            status: "beta",
            icon_key: "check-circle",
            sort_order: 30,
        },
    )
    .await?;

    upsert_mode(
        pool,
        ModeSeed {
            id: MODE_GAMER_QUIZ_CLASSIC_ID,
            game_id: GAME_GAMER_QUIZ_ID,
            code: "classic",
            name: "Classico",
            description: "Pontuacao simples por resposta correta.",
            initial_points: Some(0),
            time_seconds: Some(30),
        },
    )
    .await?;
    upsert_mode(
        pool,
        ModeSeed {
            id: MODE_GAMER_QUIZ_POINTS_ID,
            game_id: GAME_GAMER_QUIZ_ID,
            code: "initial_points",
            name: "Pontos Iniciais",
            description: "Todos comecam com pontos e podem ganhar ou perder ao responder.",
            initial_points: Some(100),
            time_seconds: Some(30),
        },
    )
    .await?;
    upsert_mode(
        pool,
        ModeSeed {
            id: MODE_GAMER_QUIZ_ELIMINATION_ID,
            game_id: GAME_GAMER_QUIZ_ID,
            code: "elimination",
            name: "Eliminacao",
            description: "Cada erro tira uma vida; o ultimo sobrevivente vence.",
            initial_points: Some(3),
            time_seconds: Some(30),
        },
    )
    .await?;
    upsert_mode(
        pool,
        ModeSeed {
            id: MODE_WORD_CLASSIC_ID,
            game_id: GAME_WORD_COMPLETE_ID,
            code: "classic",
            name: "Classico",
            description: "Complete a palavra sorteada.",
            initial_points: Some(0),
            time_seconds: Some(45),
        },
    )
    .await?;
    upsert_mode(
        pool,
        ModeSeed {
            id: MODE_TRUE_FALSE_CLASSIC_ID,
            game_id: GAME_TRUE_FALSE_ID,
            code: "classic",
            name: "Classico",
            description: "Responda verdadeiro ou falso.",
            initial_points: Some(0),
            time_seconds: Some(20),
        },
    )
    .await?;

    Ok(())
}

struct GameSeed<'a> {
    id: &'a str,
    slug: &'a str,
    name: &'a str,
    description: &'a str,
    status: &'a str,
    icon_key: &'a str,
    sort_order: i32,
}

async fn upsert_game(pool: &MySqlPool, game: GameSeed<'_>) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO games (id, organization_id, slug, name, description, status, icon_key, sort_order)
         VALUES (?, NULL, ?, ?, ?, ?, ?, ?)
         ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), status = VALUES(status), icon_key = VALUES(icon_key), sort_order = VALUES(sort_order)",
    )
    .bind(game.id)
    .bind(game.slug)
    .bind(game.name)
    .bind(game.description)
    .bind(game.status)
    .bind(game.icon_key)
    .bind(game.sort_order)
    .execute(pool)
    .await?;
    Ok(())
}

struct ModeSeed<'a> {
    id: &'a str,
    game_id: &'a str,
    code: &'a str,
    name: &'a str,
    description: &'a str,
    initial_points: Option<i32>,
    time_seconds: Option<i32>,
}

async fn upsert_mode(pool: &MySqlPool, mode: ModeSeed<'_>) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO game_modes (id, game_id, code, name, description, default_initial_points, default_time_per_question_seconds, is_active)
         VALUES (?, ?, ?, ?, ?, ?, ?, TRUE)
         ON DUPLICATE KEY UPDATE name = VALUES(name), description = VALUES(description), default_initial_points = VALUES(default_initial_points), default_time_per_question_seconds = VALUES(default_time_per_question_seconds), is_active = TRUE",
    )
    .bind(mode.id)
    .bind(mode.game_id)
    .bind(mode.code)
    .bind(mode.name)
    .bind(mode.description)
    .bind(mode.initial_points)
    .bind(mode.time_seconds)
    .execute(pool)
    .await?;
    Ok(())
}
