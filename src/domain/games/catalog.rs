use crate::domain::games::{Game, GameKind, GameMode, GameModeKind};
use crate::domain::{DomainId, NonEmptyString, Slug, UtcDateTime};

pub const GAME_GAMER_QUIZ_ID: &str = "20000000-0000-0000-0000-000000000001";
pub const GAME_FREE_TEXT_ID: &str = "20000000-0000-0000-0000-000000000002";
pub const GAME_TRUE_FALSE_ID: &str = "20000000-0000-0000-0000-000000000003";
pub const MODE_GAMER_QUIZ_CLASSIC_ID: &str = "21000000-0000-0000-0000-000000000001";
const MODE_GAMER_QUIZ_POINTS_ID: &str = "21000000-0000-0000-0000-000000000002";
const MODE_WORD_CLASSIC_ID: &str = "21000000-0000-0000-0000-000000000003";
const MODE_TRUE_FALSE_CLASSIC_ID: &str = "21000000-0000-0000-0000-000000000004";
const MODE_GAMER_QUIZ_ELIMINATION_ID: &str = "21000000-0000-0000-0000-000000000005";

pub struct CatalogMode {
    pub id: &'static str,
    pub kind: GameModeKind,
    pub name: &'static str,
    pub description: &'static str,
    pub default_initial_points: Option<i32>,
    pub default_time_per_question_seconds: Option<i32>,
}

pub struct CatalogGame {
    pub id: &'static str,
    pub slug: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub kind: GameKind,
    pub icon_key: &'static str,
    pub sort_order: i32,
    pub modes: &'static [CatalogMode],
}

const GAMER_QUIZ_MODES: &[CatalogMode] = &[
    CatalogMode {
        id: MODE_GAMER_QUIZ_CLASSIC_ID,
        kind: GameModeKind::Classic,
        name: "Clássico",
        description: "Pontuação simples por resposta correta.",
        default_initial_points: Some(0),
        default_time_per_question_seconds: Some(30),
    },
    CatalogMode {
        id: MODE_GAMER_QUIZ_POINTS_ID,
        kind: GameModeKind::InitialPoints,
        name: "Pontos Iniciais",
        description: "Todos começam com pontos e podem ganhar ou perder ao responder.",
        default_initial_points: Some(100),
        default_time_per_question_seconds: Some(30),
    },
    CatalogMode {
        id: MODE_GAMER_QUIZ_ELIMINATION_ID,
        kind: GameModeKind::Elimination,
        name: "Eliminação",
        description: "Cada erro tira uma vida; o último sobrevivente vence.",
        default_initial_points: Some(3),
        default_time_per_question_seconds: Some(30),
    },
];

const FREE_TEXT_MODES: &[CatalogMode] = &[CatalogMode {
    id: MODE_WORD_CLASSIC_ID,
    kind: GameModeKind::Classic,
    name: "Clássico",
    description: "Escreva a resposta correta.",
    default_initial_points: Some(0),
    default_time_per_question_seconds: Some(45),
}];

const TRUE_FALSE_MODES: &[CatalogMode] = &[CatalogMode {
    id: MODE_TRUE_FALSE_CLASSIC_ID,
    kind: GameModeKind::Classic,
    name: "Clássico",
    description: "Responda verdadeiro ou falso.",
    default_initial_points: Some(0),
    default_time_per_question_seconds: Some(20),
}];

pub const GAMES: &[CatalogGame] = &[
    CatalogGame {
        id: GAME_GAMER_QUIZ_ID,
        slug: "quiz-gamer",
        name: "Quiz Gamer",
        description: "Perguntas e respostas para grupos e comunidades gamer.",
        kind: GameKind::Buzzer,
        icon_key: "quiz",
        sort_order: 10,
        modes: GAMER_QUIZ_MODES,
    },
    CatalogGame {
        id: GAME_FREE_TEXT_ID,
        slug: "escreva-a-resposta",
        name: "Escreva a Resposta",
        description: "Digite a resposta certa para vencer cada rodada.",
        kind: GameKind::FreeText,
        icon_key: "pencil",
        sort_order: 20,
        modes: FREE_TEXT_MODES,
    },
    CatalogGame {
        id: GAME_TRUE_FALSE_ID,
        slug: "verdadeiro-falso",
        name: "Verdadeiro ou Falso",
        description: "Rodadas rápidas para revisar curiosidades de cada jogo.",
        kind: GameKind::TrueFalse,
        icon_key: "check-circle",
        sort_order: 30,
        modes: TRUE_FALSE_MODES,
    },
];

/// All games offered by the catalog (the catalog has no inactive games).
pub fn active_games() -> impl Iterator<Item = &'static CatalogGame> {
    GAMES.iter()
}

pub fn game_by_id(id: &str) -> Option<&'static CatalogGame> {
    GAMES.iter().find(|g| g.id == id)
}

pub fn mode_by_id(id: &str) -> Option<(&'static CatalogGame, &'static CatalogMode)> {
    GAMES
        .iter()
        .find_map(|g| g.modes.iter().find(|m| m.id == id).map(|m| (g, m)))
}

pub fn game_kind(game_id: &str) -> Option<GameKind> {
    game_by_id(game_id).map(|g| g.kind)
}

pub fn game_name(game_id: &str) -> Option<&'static str> {
    game_by_id(game_id).map(|g| g.name)
}

pub fn mode_code(mode_id: &str) -> Option<&'static str> {
    mode_by_id(mode_id).map(|(_, m)| mode_code_of(m.kind))
}

pub fn mode_name(mode_id: &str) -> Option<&'static str> {
    mode_by_id(mode_id).map(|(_, m)| m.name)
}

pub fn mode_code_of(kind: GameModeKind) -> &'static str {
    match kind {
        GameModeKind::Classic => "classic",
        GameModeKind::InitialPoints => "initial_points",
        GameModeKind::Elimination => "elimination",
    }
}

const EPOCH: &str = "1970-01-01T00:00:00Z";

pub fn to_domain_game(game: &CatalogGame) -> Game {
    Game {
        id: DomainId::new(game.id).expect("catalog game id is valid"),
        organization_id: None,
        slug: Slug::new(game.slug).expect("catalog slug is valid"),
        name: NonEmptyString::new(game.name, "game.name").expect("catalog name is non-empty"),
        description: NonEmptyString::new(game.description, "game.description")
            .expect("catalog description is non-empty"),
        created_at: UtcDateTime::new_iso8601(EPOCH).expect("epoch is valid"),
        updated_at: UtcDateTime::new_iso8601(EPOCH).expect("epoch is valid"),
    }
}

pub fn to_domain_mode(game_id: &str, mode: &CatalogMode) -> GameMode {
    GameMode {
        id: DomainId::new(mode.id).expect("catalog mode id is valid"),
        game_id: DomainId::new(game_id).expect("catalog game id is valid"),
        kind: mode.kind,
        name: NonEmptyString::new(mode.name, "game_mode.name").expect("catalog mode name"),
        description: Some(mode.description.to_owned()),
        default_initial_points: mode.default_initial_points,
        default_time_per_question_seconds: mode.default_time_per_question_seconds,
        is_active: true,
        created_at: UtcDateTime::new_iso8601(EPOCH).expect("epoch is valid"),
        updated_at: UtcDateTime::new_iso8601(EPOCH).expect("epoch is valid"),
    }
}
