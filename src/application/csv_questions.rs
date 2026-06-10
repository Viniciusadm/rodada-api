use crate::domain::games::{McOption, QuestionType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCsvQuestion {
    pub question_type: QuestionType,
    pub prompt: String,
    pub expected_answer: String,
    pub options: Vec<McOption>,
    pub points: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportQuestionsReport {
    pub created: usize,
    pub invalid_rows: Vec<CsvInvalidRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvInvalidRow {
    pub line: usize,
    pub reason: String,
}

pub fn parse_csv(csv: &str) -> (Vec<ParsedCsvQuestion>, Vec<CsvInvalidRow>) {
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
