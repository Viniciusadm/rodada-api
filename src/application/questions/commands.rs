use crate::domain::games::{McOption, QuestionType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateGeneralQuestionCommand {
    pub owner_user_id: String,
    pub question_type: QuestionType,
    pub prompt: String,
    pub expected_answer: String,
    pub options: Vec<McOption>,
    pub default_points: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateGeneralQuestionCommand {
    pub id: String,
    pub owner_user_id: String,
    pub question_type: QuestionType,
    pub prompt: String,
    pub expected_answer: String,
    pub options: Vec<McOption>,
    pub default_points: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteGeneralQuestionCommand {
    pub id: String,
    pub owner_user_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListGeneralQuestionsQuery {
    pub owner_user_id: String,
    pub search: Option<String>,
    pub page: u32,
    pub page_size: u32,
}
