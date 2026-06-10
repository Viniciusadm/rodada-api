use holyplay_backend::domain::games::{McOption, QuestionType};

#[test]
fn grade_judges_each_question_type() {
    assert!(QuestionType::TrueFalse.grade("true", None, "verdadeiro"));
    assert!(!QuestionType::TrueFalse.grade("true", None, "falso"));

    assert!(QuestionType::FreeText.grade("Poção|pocao", None, "POÇÃO"));
    assert!(!QuestionType::FreeText.grade("Poção", None, "espada"));

    let options = [
        McOption {
            text: "Paris".to_owned(),
            correct: true,
        },
        McOption {
            text: "Londres".to_owned(),
            correct: false,
        },
    ];
    assert!(QuestionType::MultipleChoice.grade("Paris", Some(&options), "0"));
    assert!(!QuestionType::MultipleChoice.grade("Paris", Some(&options), "1"));
    assert!(!QuestionType::MultipleChoice.grade("Paris", Some(&options), "5"));
    assert!(!QuestionType::MultipleChoice.grade("Paris", None, "0"));

    assert!(!QuestionType::Buzzer.grade("x", None, "x"));
}
