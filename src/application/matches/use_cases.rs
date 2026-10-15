use uuid::Uuid;

use crate::application::csv_questions::{parse_csv, ImportQuestionsReport};
use crate::application::matches::{
    AddMatchQuestionCommand, AddParticipantCommand, AdjustScoreCommand, BuzzCommand, BuzzOutcome,
    CreateMatchCommand, DeleteMatchQuestionCommand, DrawRoundCommand, EndMatchCommand,
    ImportMatchQuestionsCommand, JoinByPinCommand, JoinMatchByIdCommand, JoinMatchResult,
    MatchRealtimeEventKind, MatchRepository, MatchRoundDetailState, MatchState,
    NoopRealtimeMatchPublisher, PlayerAnswerCommand, PlayerAnswerOutcome, RealtimeMatchPublisher,
    RegisterAnswerCommand, ResetMatchCommand, RevealAnswerCommand, StartMatchCommand,
    UpdateMatchQuestionCommand,
};
use crate::application::questions::validate_question_payload;
use crate::domain::games::QuestionType;
use crate::domain::matches::Match;
use crate::errors::AppError;

pub struct MatchUseCases<R, P = NoopRealtimeMatchPublisher> {
    repository: R,
    realtime: P,
}

impl<R> MatchUseCases<R>
where
    R: MatchRepository,
{
    pub fn new(repository: R) -> Self {
        Self {
            repository,
            realtime: NoopRealtimeMatchPublisher,
        }
    }
}

impl<R, P> MatchUseCases<R, P>
where
    R: MatchRepository,
    P: RealtimeMatchPublisher,
{
    pub fn new_with_realtime(repository: R, realtime: P) -> Self {
        Self {
            repository,
            realtime,
        }
    }

    pub async fn create_match(&self, command: CreateMatchCommand) -> Result<Match, AppError> {
        let id = new_id();
        self.repository.create_from_template(&id, &command).await?;
        let created = self
            .repository
            .find_by_id(&id)
            .await?
            .ok_or_else(|| AppError::NotFound("created match was not found".to_owned()))?;
        self.realtime
            .publish_match_event(created.id.as_str(), MatchRealtimeEventKind::Created)
            .await?;
        self.realtime
            .publish_user_event(
                created.host_user_id.as_str(),
                MatchRealtimeEventKind::RoomUpdated,
            )
            .await?;
        Ok(created)
    }

    pub async fn start_match(&self, command: StartMatchCommand) -> Result<(), AppError> {
        self.repository.start(&command).await?;
        self.publish_updated(&command.match_id).await?;
        self.publish_room_updated(&command.match_id).await
    }

    pub async fn draw_question(&self, command: DrawRoundCommand) -> Result<(), AppError> {
        self.repository.draw_round(&new_id(), &command).await?;
        self.publish_updated(&command.match_id).await
    }

    pub async fn join_by_pin(
        &self,
        command: JoinByPinCommand,
    ) -> Result<JoinMatchResult, AppError> {
        let token = new_id();
        let participant = self
            .repository
            .join_by_pin(&new_id(), &token, &command)
            .await?;
        self.publish_state(
            participant.match_id.as_str(),
            MatchRealtimeEventKind::RoomUpdated,
        )
        .await?;
        self.publish_room_updated(participant.match_id.as_str())
            .await?;
        Ok(JoinMatchResult {
            participant,
            participant_token: token,
        })
    }

    pub async fn resume_by_pin(&self, pin: &str, token: &str) -> Result<JoinMatchResult, AppError> {
        let participant = self.repository.resume_by_pin(pin, token).await?;
        Ok(JoinMatchResult {
            participant,
            participant_token: token.to_owned(),
        })
    }

    pub async fn join_match_by_id(
        &self,
        command: JoinMatchByIdCommand,
    ) -> Result<JoinMatchResult, AppError> {
        let token = new_id();
        let participant = self
            .repository
            .join_match_by_id(&new_id(), &token, &command)
            .await?;
        self.publish_state(
            participant.match_id.as_str(),
            MatchRealtimeEventKind::RoomUpdated,
        )
        .await?;
        self.publish_room_updated(participant.match_id.as_str())
            .await?;
        Ok(JoinMatchResult {
            participant,
            participant_token: token,
        })
    }

    pub async fn add_participant(&self, command: AddParticipantCommand) -> Result<(), AppError> {
        self.repository.add_participant(&new_id(), &command).await?;
        self.publish_state(&command.match_id, MatchRealtimeEventKind::RoomUpdated)
            .await?;
        self.publish_room_updated(&command.match_id).await
    }

    pub async fn add_match_question(
        &self,
        command: AddMatchQuestionCommand,
    ) -> Result<(), AppError> {
        validate_question_payload(
            command.question_type.unwrap_or(QuestionType::FreeText),
            &command.prompt,
            &command.expected_answer,
            &command.options,
        )?;
        self.repository
            .add_match_question(&new_id(), &command)
            .await?;
        self.publish_updated(&command.match_id).await
    }

    pub async fn update_match_question(
        &self,
        command: UpdateMatchQuestionCommand,
    ) -> Result<(), AppError> {
        validate_question_payload(
            command.question_type.unwrap_or(QuestionType::FreeText),
            &command.prompt,
            &command.expected_answer,
            &command.options,
        )?;
        self.repository.update_match_question(&command).await?;
        self.publish_updated(&command.match_id).await
    }

    pub async fn delete_match_question(
        &self,
        command: DeleteMatchQuestionCommand,
    ) -> Result<(), AppError> {
        self.repository.delete_match_question(&command).await?;
        self.publish_updated(&command.match_id).await
    }

    pub async fn import_match_questions(
        &self,
        command: ImportMatchQuestionsCommand,
    ) -> Result<ImportQuestionsReport, AppError> {
        let (rows, invalid_rows) = parse_csv(&command.csv);
        if rows.is_empty() && invalid_rows.is_empty() {
            return Err(AppError::Validation("CSV has no question rows".to_owned()));
        }
        self.repository
            .import_match_questions(&command, &rows)
            .await?;
        self.publish_updated(&command.match_id).await?;
        Ok(ImportQuestionsReport {
            created: rows.len(),
            invalid_rows,
        })
    }

    pub async fn buzz(&self, command: BuzzCommand) -> Result<bool, AppError> {
        let outcome = self.repository.buzz(&new_id(), &command).await?;
        self.publish_state(&command.match_id, MatchRealtimeEventKind::BuzzerUpdated)
            .await?;
        Ok(matches!(outcome, BuzzOutcome::Accepted))
    }

    pub async fn player_answer(
        &self,
        command: PlayerAnswerCommand,
    ) -> Result<PlayerAnswerOutcome, AppError> {
        let outcome = self.repository.player_answer(&command).await?;
        self.publish_state(&command.match_id, MatchRealtimeEventKind::BuzzerUpdated)
            .await?;
        Ok(outcome)
    }

    pub async fn reveal_answer(&self, command: RevealAnswerCommand) -> Result<(), AppError> {
        self.repository.reveal_answer(&command).await?;
        self.publish_updated(&command.match_id).await
    }

    pub async fn register_answer(&self, command: RegisterAnswerCommand) -> Result<(), AppError> {
        self.repository.register_answer(&new_id(), &command).await?;
        self.publish_updated(&command.match_id).await
    }

    pub async fn adjust_score(&self, command: AdjustScoreCommand) -> Result<(), AppError> {
        self.repository
            .record_score_adjustment(&new_id(), &command)
            .await?;
        self.publish_updated(&command.match_id).await
    }

    pub async fn end_match(&self, command: EndMatchCommand) -> Result<(), AppError> {
        self.repository.end(&command).await?;
        self.publish_updated(&command.match_id).await?;
        self.publish_room_updated(&command.match_id).await
    }

    pub async fn reset_match(&self, command: ResetMatchCommand) -> Result<(), AppError> {
        self.repository.reset(&command).await?;
        self.publish_updated(&command.match_id).await
    }

    pub async fn get_state(&self, match_id: &str) -> Result<Option<MatchState>, AppError> {
        self.repository.get_state(match_id).await
    }

    pub async fn list_round_details(
        &self,
        match_id: &str,
    ) -> Result<Vec<MatchRoundDetailState>, AppError> {
        self.repository.list_round_details(match_id).await
    }

    async fn publish_updated(&self, match_id: &str) -> Result<(), AppError> {
        self.publish_state(match_id, MatchRealtimeEventKind::Updated)
            .await
    }

    async fn publish_state(
        &self,
        match_id: &str,
        kind: MatchRealtimeEventKind,
    ) -> Result<(), AppError> {
        match self.repository.get_state(match_id).await? {
            Some(state) => {
                self.realtime
                    .publish_match_state(match_id, kind, state)
                    .await
            }
            None => self.realtime.publish_match_event(match_id, kind).await,
        }
    }

    async fn publish_room_updated(&self, match_id: &str) -> Result<(), AppError> {
        let Some(match_entity) = self.repository.find_by_id(match_id).await? else {
            return Ok(());
        };
        self.realtime
            .publish_user_event(
                match_entity.host_user_id.as_str(),
                MatchRealtimeEventKind::RoomUpdated,
            )
            .await
    }
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}
