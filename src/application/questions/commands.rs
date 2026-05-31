#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateQuestionBankCommand {
    pub organization_id: String,
    pub game_id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_by_user_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionDraftCommand {
    pub prompt: String,
    pub expected_answer: String,
    pub default_points: i32,
    pub difficulty: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateQuestionCommand {
    pub question_bank_id: String,
    pub question: QuestionDraftCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportQuestionsCommand {
    pub organization_id: String,
    pub question_bank_id: String,
    pub imported_by_user_id: String,
    pub questions: Vec<QuestionDraftCommand>,
}
