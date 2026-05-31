#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchStatus {
    Waiting,
    InProgress,
    Finished,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinSource {
    Local,
    Remote,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundStatus {
    Open,
    AnswerRevealed,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerResult {
    Correct,
    Incorrect,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreEventKind {
    InitialPoints,
    CorrectAnswer,
    IncorrectAnswer,
    ManualAdjustment,
    Penalty,
    Reset,
}
