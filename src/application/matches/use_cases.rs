use uuid::Uuid;

use crate::application::matches::{
    AddMatchQuestionCommand, AddParticipantCommand, AdjustScoreCommand, BuzzCommand, BuzzOutcome,
    CreateMatchCommand, CsvInvalidRow, DeleteMatchQuestionCommand, DrawRoundCommand,
    EndMatchCommand, ImportMatchQuestionsCommand, ImportQuestionsReport, JoinByPinCommand,
    JoinMatchByIdCommand, JoinMatchResult, MatchRealtimeEventKind, MatchRepository,
    MatchRoundDetailState, MatchState, NoopRealtimeMatchPublisher, ParsedCsvQuestion,
    PlayerAnswerCommand, PlayerAnswerOutcome, RealtimeMatchPublisher, RegisterAnswerCommand,
    ResetMatchCommand, RevealAnswerCommand, StartMatchCommand, UpdateMatchQuestionCommand,
};
use crate::domain::games::{MatchMode, McOption, QuestionType, ScoringMode};
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
        validate_create_match(&command)?;
        let id = new_id();
        self.repository.create(&id, &command).await?;
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
        self.realtime
            .publish_match_event(
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

    pub async fn join_match_by_id(
        &self,
        command: JoinMatchByIdCommand,
    ) -> Result<JoinMatchResult, AppError> {
        let token = new_id();
        let participant = self
            .repository
            .join_match_by_id(&new_id(), &token, &command)
            .await?;
        self.realtime
            .publish_match_event(
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
        self.realtime
            .publish_match_event(&command.match_id, MatchRealtimeEventKind::RoomUpdated)
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
        self.realtime
            .publish_match_event(&command.match_id, MatchRealtimeEventKind::BuzzerUpdated)
            .await?;
        Ok(matches!(outcome, BuzzOutcome::Accepted))
    }

    pub async fn player_answer(
        &self,
        command: PlayerAnswerCommand,
    ) -> Result<PlayerAnswerOutcome, AppError> {
        let outcome = self.repository.player_answer(&command).await?;
        self.realtime
            .publish_match_event(&command.match_id, MatchRealtimeEventKind::BuzzerUpdated)
            .await?;

        if outcome.correct.is_some() {
            self.realtime
                .publish_match_event(&command.match_id, MatchRealtimeEventKind::RankingUpdated)
                .await?;
            self.publish_ranking_updated(&command.match_id).await?;
        }
        Ok(outcome)
    }

    pub async fn reveal_answer(&self, command: RevealAnswerCommand) -> Result<(), AppError> {
        self.repository.reveal_answer(&command).await?;
        self.publish_updated(&command.match_id).await
    }

    pub async fn register_answer(&self, command: RegisterAnswerCommand) -> Result<(), AppError> {
        self.repository.register_answer(&new_id(), &command).await?;
        self.publish_updated(&command.match_id).await?;
        self.realtime
            .publish_match_event(&command.match_id, MatchRealtimeEventKind::RankingUpdated)
            .await?;
        self.publish_ranking_updated(&command.match_id).await
    }

    pub async fn adjust_score(&self, command: AdjustScoreCommand) -> Result<(), AppError> {
        self.repository
            .record_score_adjustment(&new_id(), &command)
            .await?;
        self.publish_updated(&command.match_id).await?;
        self.realtime
            .publish_match_event(&command.match_id, MatchRealtimeEventKind::RankingUpdated)
            .await?;
        self.publish_ranking_updated(&command.match_id).await
    }

    pub async fn end_match(&self, command: EndMatchCommand) -> Result<(), AppError> {
        self.repository.end(&command).await?;
        self.publish_updated(&command.match_id).await?;
        self.realtime
            .publish_match_event(&command.match_id, MatchRealtimeEventKind::RankingUpdated)
            .await?;
        self.publish_ranking_updated(&command.match_id).await?;
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
        self.realtime
            .publish_match_event(match_id, MatchRealtimeEventKind::Updated)
            .await
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

    async fn publish_ranking_updated(&self, match_id: &str) -> Result<(), AppError> {
        let Some(match_entity) = self.repository.find_by_id(match_id).await? else {
            return Ok(());
        };
        self.realtime
            .publish_user_event(
                match_entity.host_user_id.as_str(),
                MatchRealtimeEventKind::RankingUpdated,
            )
            .await
    }
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

fn validate_create_match(command: &CreateMatchCommand) -> Result<(), AppError> {
    if command.questions.is_empty() {
        return Err(AppError::Validation(
            "match needs at least one question".to_owned(),
        ));
    }

    if command.mode == MatchMode::Quiz && command.scoring_mode != ScoringMode::Classic {
        return Err(AppError::Validation(
            "quiz matches only support classic scoring".to_owned(),
        ));
    }

    for question in &command.questions {
        match command.mode {
            MatchMode::Buzzer => {
                if question.question_type != QuestionType::Buzzer {
                    return Err(AppError::Validation(
                        "buzzer matches only support buzzer questions".to_owned(),
                    ));
                }
            }
            MatchMode::Quiz => {
                if question.question_type == QuestionType::Buzzer {
                    return Err(AppError::Validation(
                        "quiz questions must have a type".to_owned(),
                    ));
                }
            }
        }
        validate_question_payload(
            question.question_type,
            &question.prompt,
            &question.expected_answer,
            &question.options,
        )?;
    }

    Ok(())
}

fn validate_question_payload(
    question_type: QuestionType,
    prompt: &str,
    expected_answer: &str,
    options: &[McOption],
) -> Result<(), AppError> {
    if prompt.trim().is_empty() {
        return Err(AppError::Validation("question prompt is required".to_owned()));
    }
    match question_type {
        QuestionType::MultipleChoice => {
            if options.len() < 2 || options.len() > 4 {
                return Err(AppError::Validation(
                    "multiple choice needs 2 to 4 options".to_owned(),
                ));
            }
            if options.iter().any(|option| option.text.trim().is_empty()) {
                return Err(AppError::Validation(
                    "multiple choice options cannot be empty".to_owned(),
                ));
            }
            if options.iter().filter(|option| option.correct).count() != 1 {
                return Err(AppError::Validation(
                    "multiple choice needs exactly one correct option".to_owned(),
                ));
            }
        }
        _ => {
            if expected_answer.trim().is_empty() {
                return Err(AppError::Validation(
                    "question expected answer is required".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn parse_csv(csv: &str) -> (Vec<ParsedCsvQuestion>, Vec<CsvInvalidRow>) {
    let mut rows = Vec::new();
    let mut invalid_rows = Vec::new();

    for (index, line) in csv.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lowered = trimmed.to_ascii_lowercase();
        if index == 0 && (lowered.contains("prompt") || lowered.starts_with("tipo,")) {
            continue;
        }

        let columns = parse_csv_line(trimmed);
        match parse_csv_row(&columns) {
            Ok(question) => rows.push(question),
            Err(reason) => invalid_rows.push(CsvInvalidRow {
                line: line_number,
                reason,
            }),
        }
    }

    (rows, invalid_rows)
}

fn parse_csv_row(columns: &[String]) -> Result<ParsedCsvQuestion, String> {
    let type_keyword = columns
        .first()
        .and_then(|value| parse_csv_type(value.trim()));

    let (question_type, prompt, answer, points_raw, options_raw) = match type_keyword {
        Some(question_type) => (
            question_type,
            columns.get(1).map(String::as_str).unwrap_or("").trim(),
            columns.get(2).map(String::as_str).unwrap_or("").trim(),
            columns.get(3).map(String::as_str).unwrap_or("").trim(),
            columns.get(4).map(String::as_str).unwrap_or("").trim(),
        ),
        None => (
            QuestionType::FreeText,
            columns.first().map(String::as_str).unwrap_or("").trim(),
            columns.get(1).map(String::as_str).unwrap_or("").trim(),
            columns.get(2).map(String::as_str).unwrap_or("").trim(),
            "",
        ),
    };

    if prompt.is_empty() {
        return Err("prompt is required".to_owned());
    }
    let points = if points_raw.is_empty() {
        100
    } else {
        points_raw
            .parse::<i32>()
            .map_err(|_| "points must be an integer".to_owned())?
    };
    if points < 0 {
        return Err("points must be zero or greater".to_owned());
    }

    match question_type {
        QuestionType::MultipleChoice => {
            let options = parse_csv_options(options_raw);
            if options.len() < 2 || options.len() > 4 {
                return Err("multiple choice needs 2 to 4 options".to_owned());
            }
            let correct = options.iter().filter(|option| option.correct).count();
            if correct != 1 {
                return Err("multiple choice needs exactly one correct option".to_owned());
            }
            let expected_answer = options
                .iter()
                .find(|option| option.correct)
                .map(|option| option.text.clone())
                .unwrap_or_default();
            Ok(ParsedCsvQuestion {
                question_type,
                prompt: prompt.to_owned(),
                expected_answer,
                options,
                points,
            })
        }
        QuestionType::TrueFalse => {
            if answer.is_empty() {
                return Err("expected answer is required".to_owned());
            }
            let normalized = if matches!(
                answer.to_ascii_lowercase().as_str(),
                "true" | "verdadeiro" | "v" | "1" | "sim"
            ) {
                "true"
            } else {
                "false"
            };
            Ok(ParsedCsvQuestion {
                question_type,
                prompt: prompt.to_owned(),
                expected_answer: normalized.to_owned(),
                options: Vec::new(),
                points,
            })
        }
        _ => {
            if answer.is_empty() {
                return Err("expected answer is required".to_owned());
            }
            Ok(ParsedCsvQuestion {
                question_type: QuestionType::FreeText,
                prompt: prompt.to_owned(),
                expected_answer: answer.to_owned(),
                options: Vec::new(),
                points,
            })
        }
    }
}

fn parse_csv_type(value: &str) -> Option<QuestionType> {
    match value.to_ascii_lowercase().as_str() {
        "texto" | "free_text" => Some(QuestionType::FreeText),
        "vf" | "v/f" | "true_false" => Some(QuestionType::TrueFalse),
        "multipla" | "múltipla" | "multiple_choice" | "mc" => Some(QuestionType::MultipleChoice),
        _ => None,
    }
}

fn parse_csv_options(raw: &str) -> Vec<McOption> {
    raw.split('|')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            if let Some(text) = value.strip_prefix('*') {
                McOption {
                    text: text.trim().to_owned(),
                    correct: true,
                }
            } else {
                McOption {
                    text: value.to_owned(),
                    correct: false,
                }
            }
        })
        .collect()
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut columns = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(char) = chars.next() {
        match char {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                current.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                columns.push(current.trim().to_owned());
                current.clear();
            }
            _ => current.push(char),
        }
    }
    columns.push(current.trim().to_owned());
    columns
}
