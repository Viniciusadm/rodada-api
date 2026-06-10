use uuid::Uuid;

use crate::application::matches::MatchQuestionDraftCommand;
use crate::application::match_templates::{
    CreateMatchTemplateCommand, DeleteMatchTemplateCommand, DuplicateMatchTemplateCommand,
    MatchTemplateRepository, MatchTemplateState, MatchTemplateSummaryState,
    UpdateMatchTemplateCommand,
};
use crate::application::questions::validate_question_payload;
use crate::domain::games::{MatchMode, QuestionType, ScoringMode};
use crate::errors::AppError;

pub struct MatchTemplateUseCases<R> {
    repository: R,
}

impl<R> MatchTemplateUseCases<R>
where
    R: MatchTemplateRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn create(
        &self,
        command: CreateMatchTemplateCommand,
    ) -> Result<MatchTemplateState, AppError> {
        validate_template(
            command.mode,
            command.scoring_mode,
            &command.name,
            &command.questions,
        )?;
        let id = new_id();
        self.repository.create(&id, &command).await?;
        self.fetch(&command.host_user_id, &id).await
    }

    pub async fn update(
        &self,
        command: UpdateMatchTemplateCommand,
    ) -> Result<MatchTemplateState, AppError> {
        validate_template(
            command.mode,
            command.scoring_mode,
            &command.name,
            &command.questions,
        )?;
        let updated = self.repository.update(&command).await?;
        if !updated {
            return Err(AppError::NotFound("match template was not found".to_owned()));
        }
        self.fetch(&command.host_user_id, &command.id).await
    }

    pub async fn delete(&self, command: DeleteMatchTemplateCommand) -> Result<(), AppError> {
        let deleted = self.repository.delete(&command).await?;
        if !deleted {
            return Err(AppError::NotFound("match template was not found".to_owned()));
        }
        Ok(())
    }

    pub async fn duplicate(
        &self,
        command: DuplicateMatchTemplateCommand,
    ) -> Result<MatchTemplateState, AppError> {
        let source = self.fetch(&command.host_user_id, &command.id).await?;
        let create = into_create_command(&command.host_user_id, source)?;
        self.create(create).await
    }

    pub async fn list(
        &self,
        owner_user_id: &str,
        search: Option<&str>,
    ) -> Result<Vec<MatchTemplateSummaryState>, AppError> {
        self.repository
            .list_summaries_by_owner(owner_user_id, search)
            .await
    }

    pub async fn get(
        &self,
        owner_user_id: &str,
        id: &str,
    ) -> Result<MatchTemplateState, AppError> {
        self.fetch(owner_user_id, id).await
    }

    async fn fetch(
        &self,
        owner_user_id: &str,
        id: &str,
    ) -> Result<MatchTemplateState, AppError> {
        self.repository
            .get_with_questions(owner_user_id, id)
            .await?
            .ok_or_else(|| AppError::NotFound("match template was not found".to_owned()))
    }
}

fn new_id() -> String {
    Uuid::new_v4().to_string()
}

pub fn validate_template(
    mode: MatchMode,
    scoring_mode: ScoringMode,
    name: &str,
    questions: &[MatchQuestionDraftCommand],
) -> Result<(), AppError> {
    if name.trim().is_empty() {
        return Err(AppError::Validation("template name is required".to_owned()));
    }
    if questions.is_empty() {
        return Err(AppError::Validation(
            "template needs at least one question".to_owned(),
        ));
    }
    if mode == MatchMode::Quiz && scoring_mode != ScoringMode::Classic {
        return Err(AppError::Validation(
            "quiz templates only support classic scoring".to_owned(),
        ));
    }
    for question in questions {
        match mode {
            MatchMode::Buzzer => {
                if question.question_type != QuestionType::Buzzer {
                    return Err(AppError::Validation(
                        "buzzer templates only support buzzer questions".to_owned(),
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

fn into_create_command(
    host_user_id: &str,
    source: MatchTemplateState,
) -> Result<CreateMatchTemplateCommand, AppError> {
    let mode = MatchMode::from_code(&source.mode)
        .ok_or_else(|| AppError::Validation("unknown match mode".to_owned()))?;
    let scoring_mode = ScoringMode::from_code(&source.scoring_mode)
        .ok_or_else(|| AppError::Validation("unknown scoring mode".to_owned()))?;
    let ranking_display =
        crate::domain::games::RankingDisplay::from_code(&source.ranking_display)
            .ok_or_else(|| AppError::Validation("unknown ranking display".to_owned()))?;
    let questions = source
        .questions
        .into_iter()
        .map(|question| {
            let question_type = QuestionType::from_code(&question.question_type)
                .unwrap_or(QuestionType::FreeText);
            MatchQuestionDraftCommand {
                source_question_id: question.source_question_id,
                question_type,
                prompt: question.prompt,
                expected_answer: question.expected_answer,
                options: question.options,
                points: question.points,
            }
        })
        .collect();
    Ok(CreateMatchTemplateCommand {
        host_user_id: host_user_id.to_owned(),
        name: format!("{} (cópia)", source.name),
        description: source.description,
        mode,
        scoring_mode,
        max_rounds: source.max_rounds,
        time_per_round_seconds: source.time_per_round_seconds,
        initial_points: source.initial_points,
        remote_enabled: source.remote_enabled,
        capacity: source.capacity,
        buzzer_arm_delay_seconds: source.buzzer_arm_delay_seconds,
        ranking_display,
        questions,
    })
}
