use serde::{Deserialize, Serialize};

use crate::application::rooms::{BuzzerCommand, JoinRoomCommand, OpenRoomCommand};

#[derive(Debug, Deserialize)]
pub struct OpenRoomRequest {
    pub organization_id: String,
    pub match_id: String,
    pub capacity: Option<u32>,
}

impl From<OpenRoomRequest> for OpenRoomCommand {
    fn from(value: OpenRoomRequest) -> Self {
        Self {
            organization_id: value.organization_id,
            match_id: value.match_id,
            capacity: value.capacity,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct JoinRoomRequest {
    pub pin: String,
    pub user_id: Option<String>,
    pub participant_name: Option<String>,
}

impl From<JoinRoomRequest> for JoinRoomCommand {
    fn from(value: JoinRoomRequest) -> Self {
        Self {
            pin: value.pin,
            user_id: value.user_id,
            participant_name: value.participant_name,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ResumeRoomRequest {
    pub pin: String,
    pub participant_token: String,
}

#[derive(Debug, Deserialize)]
pub struct BuzzerRequest {
    pub match_id: String,
    pub round_id: String,
    pub participant_id: String,
    pub presence_session_id: Option<String>,
}

impl From<BuzzerRequest> for BuzzerCommand {
    fn from(value: BuzzerRequest) -> Self {
        Self {
            match_id: value.match_id,
            round_id: value.round_id,
            participant_id: value.participant_id,
            presence_session_id: value.presence_session_id,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RoomSessionResponse {
    pub id: String,
    pub match_id: String,
    pub pin: String,
    pub status: String,
    pub capacity: Option<u32>,
}
