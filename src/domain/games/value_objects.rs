#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus {
    Active,
    Beta,
    Maintenance,
    ComingSoon,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameModeKind {
    Classic,
    InitialPoints,
    Elimination,
}
