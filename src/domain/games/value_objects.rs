#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStatus {
    Active,
    Beta,
    Maintenance,
    ComingSoon,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameModeKind {
    Classic,
    InitialPoints,
    Elimination,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameKind {
    Buzzer,
    TrueFalse,
    WordComplete,
}

impl GameKind {
    pub fn as_code(self) -> &'static str {
        match self {
            GameKind::Buzzer => "buzzer",
            GameKind::TrueFalse => "true_false",
            GameKind::WordComplete => "word_complete",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "buzzer" => Some(GameKind::Buzzer),
            "true_false" => Some(GameKind::TrueFalse),
            "word_complete" => Some(GameKind::WordComplete),
            _ => None,
        }
    }

    pub fn is_auto_scored(self) -> bool {
        matches!(self, GameKind::TrueFalse | GameKind::WordComplete)
    }

    pub fn grade(self, expected: &str, submitted: &str) -> bool {
        match self {
            GameKind::Buzzer => false,
            GameKind::TrueFalse => parse_bool(expected) == parse_bool(submitted),
            GameKind::WordComplete => {
                let submitted = normalize_answer(submitted);
                if submitted.is_empty() {
                    return false;
                }

                expected
                    .split('|')
                    .map(normalize_answer)
                    .any(|variant| !variant.is_empty() && variant == submitted)
            }
        }
    }
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
