use async_trait::async_trait;
use holyplay_backend::application::matches::{
    AddMatchQuestionCommand, AdjustScoreCommand, BuzzCommand, CreateMatchCommand,
    DeleteMatchQuestionCommand, DrawRoundCommand, EndMatchCommand, ImportMatchQuestionsCommand,
    JoinByPinCommand, JoinMatchByIdCommand, LiveRoomState, MatchQuestionDraftCommand,
    MatchRepository, MatchState, MatchUseCases, ParsedCsvQuestion, PlayerAnswerCommand,
    RegisterAnswerCommand, ResetMatchCommand, RevealAnswerCommand, StartMatchCommand,
    UpdateMatchQuestionCommand,
};
use holyplay_backend::domain::matches::{
    JoinSource, Match, MatchParticipant, MatchStatus, ScoreEvent,
};
use holyplay_backend::domain::{DomainId, NonEmptyString, Points, UtcDateTime};
use holyplay_backend::errors::AppError;
use std::sync::{Arc, Mutex};

#[derive(Default, Clone)]
struct FakeMatchRepository {
    state: Arc<Mutex<FakeState>>,
}

#[derive(Default)]
struct FakeState {
    match_id: Option<String>,
    participant_id: Option<String>,
    started: bool,
    drawn: bool,
    buzzed: bool,
    revealed: bool,
    answered: bool,
    adjusted: bool,
    ended: bool,
    reset: bool,
}

#[async_trait]
impl MatchRepository for FakeMatchRepository {
    async fn create(&self, id: &str, _command: &CreateMatchCommand) -> Result<(), AppError> {
        self.state.lock().unwrap().match_id = Some(id.to_owned());
        Ok(())
    }

    async fn start(&self, _command: &StartMatchCommand) -> Result<(), AppError> {
        self.state.lock().unwrap().started = true;
        Ok(())
    }

    async fn draw_round(
        &self,
        _round_id: &str,
        _command: &DrawRoundCommand,
    ) -> Result<(), AppError> {
        self.state.lock().unwrap().drawn = true;
        Ok(())
    }

    async fn join_by_pin(
        &self,
        participant_id: &str,
        _token: &str,
        _command: &JoinByPinCommand,
    ) -> Result<MatchParticipant, AppError> {
        self.state.lock().unwrap().participant_id = Some(participant_id.to_owned());
        Ok(fake_participant(participant_id))
    }

    async fn add_participant(
        &self,
        id: &str,
        _command: &holyplay_backend::application::matches::AddParticipantCommand,
    ) -> Result<(), AppError> {
        self.state.lock().unwrap().participant_id = Some(id.to_owned());
        Ok(())
    }

    async fn join_match_by_id(
        &self,
        participant_id: &str,
        _token: &str,
        _command: &JoinMatchByIdCommand,
    ) -> Result<MatchParticipant, AppError> {
        self.state.lock().unwrap().participant_id = Some(participant_id.to_owned());
        Ok(fake_participant(participant_id))
    }

    async fn add_match_question(
        &self,
        _id: &str,
        _command: &AddMatchQuestionCommand,
    ) -> Result<(), AppError> {
        Ok(())
    }

    async fn update_match_question(
        &self,
        _command: &UpdateMatchQuestionCommand,
    ) -> Result<(), AppError> {
        Ok(())
    }

    async fn delete_match_question(
        &self,
        _command: &DeleteMatchQuestionCommand,
    ) -> Result<(), AppError> {
        Ok(())
    }

    async fn import_match_questions(
        &self,
        _command: &ImportMatchQuestionsCommand,
        _rows: &[ParsedCsvQuestion],
    ) -> Result<(), AppError> {
        Ok(())
    }

    async fn player_answer(&self, _command: &PlayerAnswerCommand) -> Result<bool, AppError> {
        self.state.lock().unwrap().buzzed = true;
        Ok(true)
    }

    async fn buzz(&self, _buzzer_event_id: &str, _command: &BuzzCommand) -> Result<bool, AppError> {
        self.state.lock().unwrap().buzzed = true;
        Ok(true)
    }

    async fn reveal_answer(&self, _command: &RevealAnswerCommand) -> Result<(), AppError> {
        self.state.lock().unwrap().revealed = true;
        Ok(())
    }

    async fn register_answer(
        &self,
        _answer_id: &str,
        _command: &RegisterAnswerCommand,
    ) -> Result<(), AppError> {
        self.state.lock().unwrap().answered = true;
        Ok(())
    }

    async fn record_score_adjustment(
        &self,
        _event_id: &str,
        _command: &AdjustScoreCommand,
    ) -> Result<(), AppError> {
        self.state.lock().unwrap().adjusted = true;
        Ok(())
    }

    async fn end(&self, _command: &EndMatchCommand) -> Result<(), AppError> {
        self.state.lock().unwrap().ended = true;
        Ok(())
    }

    async fn reset(&self, _command: &ResetMatchCommand) -> Result<(), AppError> {
        self.state.lock().unwrap().reset = true;
        Ok(())
    }

    async fn find_by_id(&self, id: &str) -> Result<Option<Match>, AppError> {
        Ok(Some(fake_match(id)))
    }

    async fn list_by_organization(&self, _organization_id: &str) -> Result<Vec<Match>, AppError> {
        Ok(vec![fake_match("match-1")])
    }

    async fn list_participants(&self, _match_id: &str) -> Result<Vec<MatchParticipant>, AppError> {
        Ok(vec![fake_participant("participant-1")])
    }

    async fn list_score_events(&self, _match_id: &str) -> Result<Vec<ScoreEvent>, AppError> {
        Ok(Vec::new())
    }

    async fn get_state(&self, _match_id: &str) -> Result<Option<MatchState>, AppError> {
        Ok(None)
    }

    async fn list_live_rooms(
        &self,
        _organization_id: &str,
    ) -> Result<Vec<LiveRoomState>, AppError> {
        Ok(Vec::new())
    }
}

#[tokio::test]
async fn match_use_cases_cover_full_host_and_player_flow_without_frontend() {
    let repository = FakeMatchRepository::default();
    let state = repository.state.clone();
    let use_cases = MatchUseCases::new(repository);

    let match_entity = use_cases
        .create_match(CreateMatchCommand {
            organization_id: "org-1".to_owned(),
            game_id: "game-1".to_owned(),
            game_mode_id: "mode-1".to_owned(),
            host_user_id: "host-1".to_owned(),
            name: "Quiz Teste".to_owned(),
            max_rounds: Some(10),
            time_per_round_seconds: Some(20),
            initial_points: 0,
            remote_enabled: true,
            capacity: Some(50),
            questions: vec![MatchQuestionDraftCommand {
                source_question_id: None,
                prompt: "Pergunta?".to_owned(),
                expected_answer: "Resposta".to_owned(),
                points: 100,
            }],
        })
        .await
        .unwrap();

    use_cases
        .join_by_pin(JoinByPinCommand {
            pin: "123456".to_owned(),
            player_id: None,
            display_name: "Ana".to_owned(),
        })
        .await
        .unwrap();
    use_cases
        .start_match(StartMatchCommand {
            match_id: match_entity.id.as_str().to_owned(),
            host_user_id: "host-1".to_owned(),
        })
        .await
        .unwrap();
    use_cases
        .draw_question(DrawRoundCommand {
            match_id: match_entity.id.as_str().to_owned(),
            host_user_id: "host-1".to_owned(),
        })
        .await
        .unwrap();
    assert!(use_cases
        .buzz(BuzzCommand {
            match_id: match_entity.id.as_str().to_owned(),
            round_id: "round-1".to_owned(),
            participant_id: "participant-1".to_owned(),
            presence_session_id: None,
        })
        .await
        .unwrap());
    use_cases
        .reveal_answer(RevealAnswerCommand {
            match_id: match_entity.id.as_str().to_owned(),
            round_id: "round-1".to_owned(),
            host_user_id: "host-1".to_owned(),
        })
        .await
        .unwrap();
    use_cases
        .register_answer(RegisterAnswerCommand {
            match_id: match_entity.id.as_str().to_owned(),
            round_id: "round-1".to_owned(),
            participant_id: Some("participant-1".to_owned()),
            result: "correct".to_owned(),
            actor_user_id: "host-1".to_owned(),
        })
        .await
        .unwrap();
    use_cases
        .adjust_score(AdjustScoreCommand {
            match_id: match_entity.id.as_str().to_owned(),
            participant_id: "participant-1".to_owned(),
            actor_user_id: "host-1".to_owned(),
            delta: 50,
            reason: Some("bonus".to_owned()),
        })
        .await
        .unwrap();
    use_cases
        .end_match(EndMatchCommand {
            match_id: match_entity.id.as_str().to_owned(),
            host_user_id: "host-1".to_owned(),
        })
        .await
        .unwrap();
    use_cases
        .reset_match(ResetMatchCommand {
            match_id: match_entity.id.as_str().to_owned(),
            host_user_id: "host-1".to_owned(),
        })
        .await
        .unwrap();

    let state = state.lock().unwrap();
    assert!(state.started);
    assert!(state.drawn);
    assert!(state.buzzed);
    assert!(state.revealed);
    assert!(state.answered);
    assert!(state.adjusted);
    assert!(state.ended);
    assert!(state.reset);
}

fn fake_match(id: &str) -> Match {
    Match {
        id: domain_id(id),
        organization_id: domain_id("org-1"),
        game_id: domain_id("game-1"),
        game_mode_id: domain_id("mode-1"),
        host_user_id: domain_id("host-1"),
        name: non_empty("Quiz Teste"),
        status: MatchStatus::Waiting,
        current_round_id: None,
        winner_participant_id: None,
        created_at: now(),
        started_at: None,
        ended_at: None,
        updated_at: now(),
    }
}

fn fake_participant(id: &str) -> MatchParticipant {
    MatchParticipant {
        id: domain_id(id),
        match_id: domain_id("match-1"),
        player_id: None,
        display_name_snapshot: non_empty("Ana"),
        join_source: JoinSource::Remote,
        current_score: Points::new(0),
        joined_at: now(),
        left_at: None,
    }
}

fn domain_id(value: &str) -> DomainId {
    DomainId::new(value).unwrap()
}

fn non_empty(value: &str) -> NonEmptyString {
    NonEmptyString::new(value, "test").unwrap()
}

fn now() -> UtcDateTime {
    UtcDateTime::new_iso8601("2026-05-30T00:00:00Z").unwrap()
}
