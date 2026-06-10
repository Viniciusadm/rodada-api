use serde::{Deserialize, Serialize};

use crate::application::questions::{
    CreateGeneralQuestionCommand, ImportGeneralQuestionsCommand, ListGeneralQuestionsQuery,
    UpdateGeneralQuestionCommand,
};
use crate::domain::games::{McOption, QuestionType};
use crate::domain::questions::Question;
use crate::errors::AppError;

#[derive(Debug, Deserialize)]
pub struct McOptionRequest {
    pub text: String,
    pub correct: bool,
}

impl From<McOptionRequest> for McOption {
    fn from(value: McOptionRequest) -> Self {
        Self {
            text: value.text,
            correct: value.correct,
        }
    }
}

fn parse_question_type(code: &str) -> Result<QuestionType, AppError> {
    QuestionType::from_code(code)
        .ok_or_else(|| AppError::Validation(format!("unknown question type: {code}")))
}

fn map_options(options: Option<Vec<McOptionRequest>>) -> Vec<McOption> {
    options
        .unwrap_or_default()
        .into_iter()
        .map(Into::into)
        .collect()
}

fn resolve_answer(
    question_type: QuestionType,
    expected_answer: Option<String>,
    options: &[McOption],
) -> String {
    let answer = expected_answer.unwrap_or_default();
    if question_type == QuestionType::MultipleChoice && answer.trim().is_empty() {
        return options
            .iter()
            .find(|option| option.correct)
            .map(|option| option.text.clone())
            .unwrap_or(answer);
    }
    answer
}

#[derive(Debug, Deserialize)]
pub struct GeneralQuestionRequest {
    pub question_type: Option<String>,
    pub prompt: String,
    pub expected_answer: Option<String>,
    pub options: Option<Vec<McOptionRequest>>,
    pub default_points: Option<i32>,
}

impl GeneralQuestionRequest {
    pub fn into_create_command(
        self,
        owner_user_id: String,
    ) -> Result<CreateGeneralQuestionCommand, AppError> {
        let question_type =
            parse_question_type(self.question_type.as_deref().unwrap_or("free_text"))?;
        let options = map_options(self.options);
        Ok(CreateGeneralQuestionCommand {
            owner_user_id,
            question_type,
            prompt: self.prompt,
            expected_answer: resolve_answer(question_type, self.expected_answer, &options),
            options,
            default_points: self.default_points.unwrap_or(100),
        })
    }

    pub fn into_update_command(
        self,
        id: String,
        owner_user_id: String,
    ) -> Result<UpdateGeneralQuestionCommand, AppError> {
        let question_type =
            parse_question_type(self.question_type.as_deref().unwrap_or("free_text"))?;
        let options = map_options(self.options);
        Ok(UpdateGeneralQuestionCommand {
            id,
            owner_user_id,
            question_type,
            prompt: self.prompt,
            expected_answer: resolve_answer(question_type, self.expected_answer, &options),
            options,
            default_points: self.default_points.unwrap_or(100),
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct ListGeneralQuestionsParams {
    pub search: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

impl ListGeneralQuestionsParams {
    pub fn into_query(self, owner_user_id: String) -> ListGeneralQuestionsQuery {
        ListGeneralQuestionsQuery {
            owner_user_id,
            search: self.search,
            page: self.page.unwrap_or(1).max(1),
            page_size: self.page_size.unwrap_or(20).clamp(1, 100),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct McOptionResponse {
    pub text: String,
    pub correct: bool,
}

#[derive(Debug, Serialize)]
pub struct GeneralQuestionResponse {
    pub id: String,
    pub question_type: String,
    pub prompt: String,
    pub expected_answer: String,
    pub options: Vec<McOptionResponse>,
    pub default_points: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Question> for GeneralQuestionResponse {
    fn from(question: Question) -> Self {
        Self {
            id: question.id.as_str().to_owned(),
            question_type: question.question_type.as_code().to_owned(),
            prompt: question.text.as_str().to_owned(),
            expected_answer: question.answer.as_str().to_owned(),
            options: question
                .options
                .into_iter()
                .map(|option| McOptionResponse {
                    text: option.text,
                    correct: option.correct,
                })
                .collect(),
            default_points: question.default_points.value(),
            created_at: question.created_at.as_str().to_owned(),
            updated_at: question.updated_at.as_str().to_owned(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct GeneralQuestionListResponse {
    pub items: Vec<GeneralQuestionResponse>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Deserialize)]
pub struct ImportGeneralQuestionsRequest {
    pub csv: String,
}

impl ImportGeneralQuestionsRequest {
    pub fn into_command(self, owner_user_id: String) -> ImportGeneralQuestionsCommand {
        ImportGeneralQuestionsCommand {
            owner_user_id,
            csv: self.csv,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CsvInvalidRowResponse {
    pub line: usize,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct ImportGeneralQuestionsResponse {
    pub created: usize,
    pub invalid_rows: Vec<CsvInvalidRowResponse>,
}
