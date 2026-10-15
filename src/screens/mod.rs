use std::sync::OnceLock;

use ribalta::ir::{Field, Scalar};
use ribalta::{Compilation, CompileOptions, Document, Locale, PageSize, Severity};
use serde::Serialize;

pub struct PresentTemplate {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub source: &'static str,
}

pub const PRESENT_TEMPLATES: &[PresentTemplate] = &[
    PresentTemplate {
        id: "classic",
        name: "Clássico",
        description: "Pergunta com opções coloridas e placar ao lado.",
        source: include_str!("templates/classic.ribalta"),
    },
    PresentTemplate {
        id: "spotlight",
        name: "Destaque",
        description: "Minimalista, com a pergunta enorme no centro e cronômetro grande.",
        source: include_str!("templates/spotlight.ribalta"),
    },
    PresentTemplate {
        id: "scoreboard",
        name: "Placar ao vivo",
        description: "O placar é o protagonista; a pergunta aparece numa faixa no topo.",
        source: include_str!("templates/scoreboard.ribalta"),
    },
    PresentTemplate {
        id: "game_show",
        name: "Show de auditório",
        description: "Cores fortes, opções grandes e comemoração a cada acerto.",
        source: include_str!("templates/game_show.ribalta"),
    },
    PresentTemplate {
        id: "arena",
        name: "Arena",
        description: "Alto contraste, número da rodada gigante e pódio no final.",
        source: include_str!("templates/arena.ribalta"),
    },
];

pub const DEFAULT_PRESENT: &str = PRESENT_TEMPLATES[0].source;

const SCREEN_PAGE: PageSize = PageSize {
    width: 1920,
    height: Some(1080),
};

fn compile(source: &str) -> Compilation {
    let options = CompileOptions {
        page: SCREEN_PAGE,
        ..CompileOptions::default()
    };
    ribalta::compile_with_options("", source, Locale::Pt, options)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Text,
    Number,
    Bool,
}

impl Kind {
    fn label(self) -> &'static str {
        match self {
            Kind::Text => "texto",
            Kind::Number => "número",
            Kind::Bool => "verdadeiro/falso",
        }
    }
}

enum Contract {
    Value(Kind),
    Records(&'static [(&'static str, Kind)]),
}

impl Contract {
    fn label(&self) -> String {
        match self {
            Contract::Value(kind) => kind.label().to_owned(),
            Contract::Records(fields) => format!(
                "lista de registros com {}",
                fields
                    .iter()
                    .map(|(name, kind)| format!("{name} ({})", kind.label()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

struct Entry {
    name: &'static str,
    contract: Contract,
    description: &'static str,
    example: &'static str,
}

const PRESENT_CONTRACT: &[Entry] = &[
    Entry {
        name: "match_name",
        contract: Contract::Value(Kind::Text),
        description: "nome da partida",
        example: "\"Quiz da juventude\"",
    },
    Entry {
        name: "status",
        contract: Contract::Value(Kind::Text),
        description: "momento da partida: question (pergunta aberta), result (resultado da última resposta), waiting (aguardando) ou finished (encerrada)",
        example: "question",
    },
    Entry {
        name: "round",
        contract: Contract::Value(Kind::Number),
        description: "número da pergunta atual",
        example: "3",
    },
    Entry {
        name: "has_timer",
        contract: Contract::Value(Kind::Bool),
        description: "se a pergunta tem tempo correndo",
        example: "true",
    },
    Entry {
        name: "seconds_left",
        contract: Contract::Value(Kind::Number),
        description: "segundos restantes no cronômetro",
        example: "18",
    },
    Entry {
        name: "question",
        contract: Contract::Value(Kind::Text),
        description: "texto da pergunta atual",
        example: "\"Qual é o maior oceano do planeta?\"",
    },
    Entry {
        name: "revealed",
        contract: Contract::Value(Kind::Bool),
        description: "se a resposta já foi revelada",
        example: "false",
    },
    Entry {
        name: "answer",
        contract: Contract::Value(Kind::Text),
        description: "resposta esperada (perguntas sem opções)",
        example: "\"Oceano Pacífico\"",
    },
    Entry {
        name: "notice",
        contract: Contract::Value(Kind::Text),
        description: "linha de status, como quem respondeu primeiro",
        example: "\"Os jogadores respondem pelo celular\"",
    },
    Entry {
        name: "was_correct",
        contract: Contract::Value(Kind::Bool),
        description: "se a última resposta estava certa",
        example: "true",
    },
    Entry {
        name: "player",
        contract: Contract::Value(Kind::Text),
        description: "quem respondeu a última pergunta",
        example: "\"Ana\"",
    },
    Entry {
        name: "points_earned",
        contract: Contract::Value(Kind::Number),
        description: "pontos ganhos na última pergunta",
        example: "100",
    },
    Entry {
        name: "headline",
        contract: Contract::Value(Kind::Text),
        description: "título quando não há pergunta: aguardando início, próxima pergunta ou encerrada",
        example: "\"Aguardando início\"",
    },
    Entry {
        name: "winner",
        contract: Contract::Value(Kind::Text),
        description: "nome de quem venceu, vazio enquanto a partida corre",
        example: "\"\"",
    },
    Entry {
        name: "show_scoreboard",
        contract: Contract::Value(Kind::Bool),
        description: "se o placar deve aparecer agora, conforme as regras do modelo",
        example: "true",
    },
    Entry {
        name: "options",
        contract: Contract::Records(&[("text", Kind::Text), ("correct", Kind::Bool)]),
        description: "opções da pergunta de múltipla escolha",
        example: "Pacífico | true",
    },
    Entry {
        name: "scoreboard",
        contract: Contract::Records(&[
            ("name", Kind::Text),
            ("points", Kind::Number),
            ("out", Kind::Bool),
        ]),
        description: "os cinco primeiros do placar",
        example: "Ana | 300 | false",
    },
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenDiagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub end_column: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

impl From<&ribalta::Diagnostic> for ScreenDiagnostic {
    fn from(diagnostic: &ribalta::Diagnostic) -> Self {
        Self {
            severity: diagnostic.severity,
            code: diagnostic.code.to_owned(),
            message: diagnostic.message.clone(),
            line: diagnostic.line,
            column: diagnostic.column,
            end_column: diagnostic.end_column,
            hint: diagnostic.hint.clone(),
        }
    }
}

pub struct ScreenCheck {
    pub document: Document,
    pub diagnostics: Vec<ScreenDiagnostic>,
}

impl ScreenCheck {
    pub fn is_valid(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity != Severity::Error)
    }

    pub fn first_error(&self) -> Option<&ScreenDiagnostic> {
        self.diagnostics
            .iter()
            .find(|diagnostic| diagnostic.severity == Severity::Error)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternField {
    pub name: &'static str,
    pub kind: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExternSuggestion {
    pub name: &'static str,
    pub detail: String,
    pub declaration: String,
    pub kind: String,
    pub description: &'static str,
    pub example: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<ExternField>>,
}

pub fn present_suggestions() -> Vec<ExternSuggestion> {
    PRESENT_CONTRACT
        .iter()
        .map(|entry| ExternSuggestion {
            name: entry.name,
            detail: format!("{} · {}", entry.contract.label(), entry.description),
            declaration: declaration(entry),
            kind: match entry.contract {
                Contract::Value(kind) => kind.label().to_owned(),
                Contract::Records(_) => "lista de registros".to_owned(),
            },
            description: entry.description,
            example: entry.example,
            fields: match entry.contract {
                Contract::Value(_) => None,
                Contract::Records(fields) => Some(
                    fields
                        .iter()
                        .map(|(name, kind)| ExternField {
                            name,
                            kind: kind.label(),
                        })
                        .collect(),
                ),
            },
        })
        .collect()
}

#[derive(Debug, Clone, Serialize)]
pub struct TemplateSummary {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub source: &'static str,
    pub document: Document,
}

pub fn present_templates() -> &'static [TemplateSummary] {
    static TEMPLATES: OnceLock<Vec<TemplateSummary>> = OnceLock::new();
    TEMPLATES.get_or_init(|| {
        PRESENT_TEMPLATES
            .iter()
            .map(|template| TemplateSummary {
                id: template.id,
                name: template.name,
                description: template.description,
                source: template.source,
                document: compile(template.source).document,
            })
            .collect()
    })
}

fn declaration(entry: &Entry) -> String {
    match entry.contract {
        Contract::Value(_) => format!("@extern {} = {}", entry.name, entry.example),
        Contract::Records(fields) => {
            let names: Vec<&str> = fields.iter().map(|(field, _)| *field).collect();
            let rule: Vec<&str> = fields.iter().map(|_| "---").collect();
            format!(
                "@data example_{name}\n| {} |\n| {} |\n| {} |\n@end\n@extern {name} = $example_{name}",
                names.join(" | "),
                rule.join(" | "),
                entry.example,
                name = entry.name,
            )
        }
    }
}

pub fn present() -> &'static Compilation {
    static PRESENT: OnceLock<Compilation> = OnceLock::new();
    PRESENT.get_or_init(|| compile(DEFAULT_PRESENT))
}

pub fn present_document(source: Option<&str>) -> Document {
    match source {
        Some(source) => compile(source).document,
        None => present().document.clone(),
    }
}

pub fn check_present(source: &str) -> ScreenCheck {
    let compilation = compile(source);
    let mut diagnostics: Vec<ScreenDiagnostic> =
        compilation.diagnostics.iter().map(Into::into).collect();
    for name in &compilation.document.externs {
        let (line, column, end_column) = declaration_of(source, name);
        let error = |code: &str, message: String, hint: String| ScreenDiagnostic {
            severity: Severity::Error,
            code: code.to_owned(),
            message,
            line,
            column,
            end_column,
            hint: Some(hint),
        };
        let Some(Entry { contract, .. }) = PRESENT_CONTRACT
            .iter()
            .find(|entry| entry.name == name.as_str())
        else {
            let names: Vec<&str> = PRESENT_CONTRACT.iter().map(|entry| entry.name).collect();
            diagnostics.push(error(
                "HUB1",
                format!("o hub não preenche `${name}`"),
                format!(
                    "Use um destes @extern: {}. Para um valor fixo, use @let",
                    names.join(", ")
                ),
            ));
            continue;
        };
        if let Some(problem) = mismatch(compilation.document.state.get(name), contract) {
            diagnostics.push(error(
                "HUB2",
                format!("`${name}` {problem}"),
                format!("O hub envia `${name}` como {}", contract.label()),
            ));
        }
    }
    ScreenCheck {
        document: compilation.document,
        diagnostics,
    }
}

fn kind_of(value: &Scalar) -> Option<Kind> {
    match value {
        Scalar::Text(_) => Some(Kind::Text),
        Scalar::Number(_) => Some(Kind::Number),
        Scalar::Bool(_) => Some(Kind::Bool),
        _ => None,
    }
}

fn field_kind(value: &Field) -> Kind {
    match value {
        Field::Text(_) => Kind::Text,
        Field::Number(_) => Kind::Number,
        Field::Bool(_) => Kind::Bool,
    }
}

fn value_label(value: &Scalar) -> &'static str {
    match value {
        Scalar::Text(_) => "texto",
        Scalar::Number(_) => "número",
        Scalar::Bool(_) => "verdadeiro/falso",
        Scalar::List(_) => "lista de textos",
        Scalar::Records(_) => "lista de registros",
    }
}

fn mismatch(value: Option<&Scalar>, contract: &Contract) -> Option<String> {
    let value = value?;
    match (contract, value) {
        (Contract::Value(expected), _) if kind_of(value) == Some(*expected) => None,
        (Contract::Value(expected), _) => Some(format!(
            "precisa ser {}, mas o exemplo é {}",
            expected.label(),
            value_label(value)
        )),
        (Contract::Records(fields), Scalar::Records(rows)) => {
            let row = rows.first()?;
            row.iter().find_map(|(field, example)| {
                match fields.iter().find(|(known, _)| known == field) {
                    None => Some(format!("tem o campo `{field}`, que o hub não envia")),
                    Some((_, expected)) if *expected != field_kind(example) => Some(format!(
                        "tem o campo `{field}` como {}, mas o hub envia {}",
                        field_kind(example).label(),
                        expected.label()
                    )),
                    Some(_) => None,
                }
            })
        }
        (Contract::Records(_), _) => Some(format!(
            "precisa ser uma lista de registros, mas o exemplo é {}",
            value_label(value)
        )),
    }
}

fn declaration_of(source: &str, name: &str) -> (usize, usize, usize) {
    for (index, text) in source.lines().enumerate() {
        let trimmed = text.trim_start();
        let Some(rest) = trimmed.strip_prefix("@extern") else {
            continue;
        };
        let rest = rest.trim_start();
        if rest
            .strip_prefix(name)
            .is_some_and(|after| after.starts_with([' ', '=', '\t']))
        {
            let column = text.len() - trimmed.len() + 1;
            return (index + 1, column, text.chars().count() + 1);
        }
    }
    (1, 1, 1)
}
