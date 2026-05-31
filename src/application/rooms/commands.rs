#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenRoomCommand {
    pub organization_id: String,
    pub match_id: String,
    pub capacity: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinRoomCommand {
    pub pin: String,
    pub user_id: Option<String>,
    pub participant_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuzzerCommand {
    pub match_id: String,
    pub round_id: String,
    pub participant_id: String,
    pub presence_session_id: Option<String>,
}
