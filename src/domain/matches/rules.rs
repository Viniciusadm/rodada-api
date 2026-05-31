use crate::domain::matches::{Match, MatchStatus};

pub fn can_start_match(
    match_entity: &Match,
    participant_count: usize,
    question_count: usize,
) -> bool {
    match_entity.status == MatchStatus::Waiting && participant_count > 0 && question_count > 0
}

pub fn can_draw_next_round(match_entity: &Match, open_round_exists: bool) -> bool {
    match_entity.status == MatchStatus::InProgress && !open_round_exists
}

pub fn can_register_answer(match_entity: &Match) -> bool {
    match_entity.status == MatchStatus::InProgress && match_entity.current_round_id.is_some()
}
