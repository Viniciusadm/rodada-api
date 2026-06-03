use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchMode {
    Buzzer,
    Quiz,
}

impl MatchMode {
    pub fn as_code(self) -> &'static str {
        match self {
            MatchMode::Buzzer => "buzzer",
            MatchMode::Quiz => "quiz",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "buzzer" => Some(MatchMode::Buzzer),
            "quiz" => Some(MatchMode::Quiz),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoringMode {
    Classic,
    InitialPoints,
    Elimination,
}

impl ScoringMode {
    pub fn as_code(self) -> &'static str {
        match self {
            ScoringMode::Classic => "classic",
            ScoringMode::InitialPoints => "initial_points",
            ScoringMode::Elimination => "elimination",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "classic" => Some(ScoringMode::Classic),
            "initial_points" => Some(ScoringMode::InitialPoints),
            "elimination" => Some(ScoringMode::Elimination),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionType {
    Buzzer,
    TrueFalse,
    FreeText,
    MultipleChoice,
}

impl QuestionType {
    pub fn as_code(self) -> &'static str {
        match self {
            QuestionType::Buzzer => "buzzer",
            QuestionType::TrueFalse => "true_false",
            QuestionType::FreeText => "free_text",
            QuestionType::MultipleChoice => "multiple_choice",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "buzzer" => Some(QuestionType::Buzzer),
            "true_false" => Some(QuestionType::TrueFalse),
            "free_text" => Some(QuestionType::FreeText),
            "multiple_choice" => Some(QuestionType::MultipleChoice),
            _ => None,
        }
    }

    pub fn is_auto_scored(self) -> bool {
        !matches!(self, QuestionType::Buzzer)
    }

    pub fn grade(self, expected: &str, options: Option<&[McOption]>, submitted: &str) -> bool {
        match self {
            QuestionType::Buzzer => false,
            QuestionType::TrueFalse => parse_bool(expected) == parse_bool(submitted),
            QuestionType::FreeText => {
                let submitted = normalize_answer(submitted);
                if submitted.is_empty() {
                    return false;
                }

                expected
                    .split('|')
                    .map(normalize_answer)
                    .any(|variant| !variant.is_empty() && variant == submitted)
            }
            QuestionType::MultipleChoice => {
                let Some(options) = options else {
                    return false;
                };
                let Ok(index) = submitted.trim().parse::<usize>() else {
                    return false;
                };
                options.get(index).is_some_and(|option| option.correct)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McOption {
    pub text: String,
    pub correct: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McOptions {
    pub options: Vec<McOption>,
}

fn parse_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "verdadeiro" | "v" | "1" | "sim"
    )
}

fn normalize_answer(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .map(strip_accent)
        .filter(|c| c.is_alphanumeric())
        .collect()
}

fn strip_accent(c: char) -> char {
    match c {
        'á' | 'à' | 'ã' | 'â' | 'ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'õ' | 'ô' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        other => other,
    }
}
