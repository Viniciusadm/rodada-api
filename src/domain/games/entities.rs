use crate::domain::games::{GameModeKind, GameStatus};
use crate::domain::{DomainId, NonEmptyString, Slug, UtcDateTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game {
    pub id: DomainId,
    pub organization_id: Option<DomainId>,
    pub slug: Slug,
    pub name: NonEmptyString,
    pub description: NonEmptyString,
    pub status: GameStatus,
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameMode {
    pub id: DomainId,
    pub game_id: DomainId,
    pub kind: GameModeKind,
    pub name: NonEmptyString,
    pub is_active: bool,
    pub created_at: UtcDateTime,
    pub updated_at: UtcDateTime,
}
