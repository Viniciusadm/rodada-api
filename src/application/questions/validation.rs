use crate::domain::games::{McOption, QuestionType};
use crate::errors::AppError;

pub fn validate_question_payload(
    question_type: QuestionType,
    prompt: &str,
    expected_answer: &str,
    options: &[McOption],
) -> Result<(), AppError> {
    if prompt.trim().is_empty() {
        return Err(AppError::Validation(
            "question prompt is required".to_owned(),
        ));
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
