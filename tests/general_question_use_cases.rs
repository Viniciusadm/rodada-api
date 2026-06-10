use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use holyplay_backend::application::questions::{
    CreateGeneralQuestionCommand, DeleteGeneralQuestionCommand, GeneralQuestionUseCases,
    ListGeneralQuestionsQuery, QuestionRepository, UpdateGeneralQuestionCommand,
};
use holyplay_backend::domain::games::{McOption, QuestionType};
use holyplay_backend::domain::questions::Question;
use holyplay_backend::domain::{DomainId, NonEmptyString, Points, UtcDateTime};
use holyplay_backend::errors::AppError;

#[derive(Clone, Default)]
struct FakeQuestionRepository {
    store: Arc<Mutex<Vec<(String, Question)>>>,
}

#[async_trait]
impl QuestionRepository for FakeQuestionRepository {
    async fn find_or_create_default_bank(&self, owner_user_id: &str) -> Result<String, AppError> {
        Ok(format!("bank-{owner_user_id}"))
    }

    async fn create_question(
        &self,
        id: &str,
        question_bank_id: &str,
        command: &CreateGeneralQuestionCommand,
    ) -> Result<(), AppError> {
        let question = build_question(
            id,
            question_bank_id,
            command.question_type,
            &command.prompt,
            &command.expected_answer,
            command.options.clone(),
            command.default_points,
        );
        self.store
            .lock()
            .unwrap()
            .push((command.owner_user_id.clone(), question));
        Ok(())
    }

    async fn update_question(
        &self,
        command: &UpdateGeneralQuestionCommand,
    ) -> Result<bool, AppError> {
        let mut store = self.store.lock().unwrap();
        for (owner, question) in store.iter_mut() {
            if owner == &command.owner_user_id && question.id.as_str() == command.id {
                *question = build_question(
                    &command.id,
                    question.question_bank_id.as_str(),
                    command.question_type,
                    &command.prompt,
                    &command.expected_answer,
                    command.options.clone(),
                    command.default_points,
                );
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn delete_question(
        &self,
        command: &DeleteGeneralQuestionCommand,
    ) -> Result<bool, AppError> {
        let mut store = self.store.lock().unwrap();
        let before = store.len();
        store.retain(|(owner, question)| {
            !(owner == &command.owner_user_id && question.id.as_str() == command.id)
        });
        Ok(store.len() != before)
    }

    async fn get_question(
        &self,
        owner_user_id: &str,
        id: &str,
    ) -> Result<Option<Question>, AppError> {
        Ok(self
            .store
            .lock()
            .unwrap()
            .iter()
            .find(|(owner, question)| owner == owner_user_id && question.id.as_str() == id)
            .map(|(_, question)| question.clone()))
    }

    async fn list_questions(
        &self,
        query: &ListGeneralQuestionsQuery,
    ) -> Result<(Vec<Question>, u64), AppError> {
        let store = self.store.lock().unwrap();
        let search = query
            .search
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_lowercase);
        let mut matched: Vec<Question> = store
            .iter()
            .filter(|(owner, _)| owner == &query.owner_user_id)
            .filter(|(_, question)| match &search {
                Some(term) => question.text.as_str().to_lowercase().contains(term),
                None => true,
            })
            .map(|(_, question)| question.clone())
            .collect();
        let total = matched.len() as u64;
        let offset = ((query.page.max(1) - 1) * query.page_size) as usize;
        let page_size = query.page_size as usize;
        let page: Vec<Question> = matched.drain(..).skip(offset).take(page_size).collect();
        Ok((page, total))
    }
}

fn build_question(
    id: &str,
    bank_id: &str,
    question_type: QuestionType,
    prompt: &str,
    answer: &str,
    options: Vec<McOption>,
    default_points: i32,
) -> Question {
    Question {
        id: DomainId::new(id).unwrap(),
        question_bank_id: DomainId::new(bank_id).unwrap(),
        question_type,
        text: NonEmptyString::new(prompt, "prompt").unwrap(),
        answer: NonEmptyString::new(answer, "answer").unwrap(),
        options,
        default_points: Points::new(default_points),
        is_active: true,
        created_at: UtcDateTime::new_iso8601("2026-06-10T00:00:00Z").unwrap(),
        updated_at: UtcDateTime::new_iso8601("2026-06-10T00:00:00Z").unwrap(),
    }
}

fn create_command(owner: &str, prompt: &str) -> CreateGeneralQuestionCommand {
    CreateGeneralQuestionCommand {
        owner_user_id: owner.to_owned(),
        question_type: QuestionType::FreeText,
        prompt: prompt.to_owned(),
        expected_answer: "resposta".to_owned(),
        options: Vec::new(),
        default_points: 100,
    }
}

#[tokio::test]
async fn create_persists_a_general_question() {
    let use_cases = GeneralQuestionUseCases::new(FakeQuestionRepository::default());

    let question = use_cases
        .create(create_command("owner-1", "Qual a capital do Brasil?"))
        .await
        .unwrap();

    assert_eq!(question.text.as_str(), "Qual a capital do Brasil?");
    assert_eq!(question.question_type, QuestionType::FreeText);
}

#[tokio::test]
async fn create_rejects_empty_prompt() {
    let use_cases = GeneralQuestionUseCases::new(FakeQuestionRepository::default());

    let mut command = create_command("owner-1", "   ");
    command.expected_answer = "resposta".to_owned();

    let error = use_cases.create(command).await.unwrap_err();
    assert!(matches!(error, AppError::Validation(_)));
}

#[tokio::test]
async fn create_rejects_multiple_choice_with_wrong_option_count() {
    let use_cases = GeneralQuestionUseCases::new(FakeQuestionRepository::default());

    let command = CreateGeneralQuestionCommand {
        owner_user_id: "owner-1".to_owned(),
        question_type: QuestionType::MultipleChoice,
        prompt: "Escolha".to_owned(),
        expected_answer: String::new(),
        options: vec![McOption {
            text: "Única".to_owned(),
            correct: true,
        }],
        default_points: 100,
    };

    let error = use_cases.create(command).await.unwrap_err();
    assert!(matches!(error, AppError::Validation(_)));
}

#[tokio::test]
async fn create_rejects_multiple_choice_without_single_correct() {
    let use_cases = GeneralQuestionUseCases::new(FakeQuestionRepository::default());

    let command = CreateGeneralQuestionCommand {
        owner_user_id: "owner-1".to_owned(),
        question_type: QuestionType::MultipleChoice,
        prompt: "Escolha".to_owned(),
        expected_answer: "A".to_owned(),
        options: vec![
            McOption {
                text: "A".to_owned(),
                correct: true,
            },
            McOption {
                text: "B".to_owned(),
                correct: true,
            },
        ],
        default_points: 100,
    };

    let error = use_cases.create(command).await.unwrap_err();
    assert!(matches!(error, AppError::Validation(_)));
}

#[tokio::test]
async fn update_changes_owned_question() {
    let repository = FakeQuestionRepository::default();
    let use_cases = GeneralQuestionUseCases::new(repository.clone());
    let created = use_cases
        .create(create_command("owner-1", "Antigo"))
        .await
        .unwrap();

    let updated = use_cases
        .update(UpdateGeneralQuestionCommand {
            id: created.id.as_str().to_owned(),
            owner_user_id: "owner-1".to_owned(),
            question_type: QuestionType::FreeText,
            prompt: "Novo".to_owned(),
            expected_answer: "resposta".to_owned(),
            options: Vec::new(),
            default_points: 200,
        })
        .await
        .unwrap();

    assert_eq!(updated.text.as_str(), "Novo");
    assert_eq!(updated.default_points.value(), 200);
}

#[tokio::test]
async fn update_rejects_question_owned_by_another_user() {
    let use_cases = GeneralQuestionUseCases::new(FakeQuestionRepository::default());
    let created = use_cases
        .create(create_command("owner-1", "Pergunta"))
        .await
        .unwrap();

    let error = use_cases
        .update(UpdateGeneralQuestionCommand {
            id: created.id.as_str().to_owned(),
            owner_user_id: "intruder".to_owned(),
            question_type: QuestionType::FreeText,
            prompt: "Hack".to_owned(),
            expected_answer: "resposta".to_owned(),
            options: Vec::new(),
            default_points: 100,
        })
        .await
        .unwrap_err();

    assert!(matches!(error, AppError::NotFound(_)));
}

#[tokio::test]
async fn delete_removes_owned_question() {
    let use_cases = GeneralQuestionUseCases::new(FakeQuestionRepository::default());
    let created = use_cases
        .create(create_command("owner-1", "Pergunta"))
        .await
        .unwrap();

    use_cases
        .delete(DeleteGeneralQuestionCommand {
            id: created.id.as_str().to_owned(),
            owner_user_id: "owner-1".to_owned(),
        })
        .await
        .unwrap();

    let error = use_cases
        .delete(DeleteGeneralQuestionCommand {
            id: created.id.as_str().to_owned(),
            owner_user_id: "owner-1".to_owned(),
        })
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::NotFound(_)));
}

#[tokio::test]
async fn list_filters_by_search_and_paginates() {
    let use_cases = GeneralQuestionUseCases::new(FakeQuestionRepository::default());
    for prompt in ["Geografia do Brasil", "Geografia da Europa", "História antiga"] {
        use_cases
            .create(create_command("owner-1", prompt))
            .await
            .unwrap();
    }
    use_cases
        .create(create_command("owner-2", "Geografia mundial"))
        .await
        .unwrap();

    let (items, total) = use_cases
        .list(ListGeneralQuestionsQuery {
            owner_user_id: "owner-1".to_owned(),
            search: Some("geografia".to_owned()),
            page: 1,
            page_size: 1,
        })
        .await
        .unwrap();

    assert_eq!(total, 2);
    assert_eq!(items.len(), 1);
}
