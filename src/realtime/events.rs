use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RealtimeEventKind {
    Ack,
    Error,
    MatchCreated,
    MatchUpdated,
    Pong,
    RoomUpdated,
    RankingUpdated,
    BuzzerUpdated,
    PresenceUpdated,
}
