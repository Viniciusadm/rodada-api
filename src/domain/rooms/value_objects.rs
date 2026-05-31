#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomStatus {
    Waiting,
    Live,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceRole {
    Host,
    Presenter,
    Player,
    Viewer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuzzerDecision {
    Accepted,
    Rejected,
    Early,
    Duplicate,
}
