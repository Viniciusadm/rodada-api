use std::sync::OnceLock;

use ribalta::ir::{Field, Scalar};
use ribalta::{Compilation, Deck, Severity};
use serde::Serialize;

pub const DEFAULT_PRESENT: &str = include_str!("present.ribalta");

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
        name: "partida",
        contract: Contract::Value(Kind::Text),
        description: "nome da partida",
        example: "\"Quiz da juventude\"",
    },
    Entry {
        name: "situacao",
        contract: Contract::Value(Kind::Text),
        description: "pergunta, resultado ou aguardando (também quando encerrada)",
        example: "pergunta",
    },
    Entry {
        name: "rodada",
        contract: Contract::Value(Kind::Number),
        description: "número da pergunta atual",
        example: "3",
    },
    Entry {
        name: "cronometro",
        contract: Contract::Value(Kind::Bool),
        description: "se a pergunta tem tempo correndo",
        example: "true",
    },
    Entry {
        name: "tempo",
        contract: Contract::Value(Kind::Number),
        description: "segundos restantes",
        example: "18",
    },
    Entry {
        name: "pergunta",
        contract: Contract::Value(Kind::Text),
        description: "texto da pergunta atual",
        example: "\"Qual é o maior oceano do planeta?\"",
    },
    Entry {
        name: "revelada",
        contract: Contract::Value(Kind::Bool),
        description: "se a resposta já foi revelada",
        example: "false",
    },
    Entry {
        name: "resposta",
        contract: Contract::Value(Kind::Text),
        description: "resposta esperada (perguntas sem opções)",
        example: "\"Oceano Pacífico\"",
    },
    Entry {
        name: "aviso",
        contract: Contract::Value(Kind::Text),
        description: "linha de status, como quem respondeu primeiro",
        example: "\"Os jogadores respondem pelo celular\"",
    },
    Entry {
        name: "acertou",
        contract: Contract::Value(Kind::Bool),
        description: "se a última resposta estava certa",
        example: "true",
    },
    Entry {
        name: "jogador",
        contract: Contract::Value(Kind::Text),
        description: "quem respondeu a última pergunta",
        example: "\"Ana\"",
    },
    Entry {
        name: "ganhos",
        contract: Contract::Value(Kind::Number),
        description: "pontos ganhos na última pergunta",
        example: "100",
    },
    Entry {
        name: "titulo",
        contract: Contract::Value(Kind::Text),
        description:
            "título quando não há pergunta: aguardando início, próxima pergunta ou encerrada",
        example: "\"Aguardando início\"",
    },
    Entry {
        name: "vencedor",
        contract: Contract::Value(Kind::Text),
        description: "nome de quem venceu, vazio enquanto a partida corre",
        example: "\"\"",
    },
    Entry {
        name: "mostrar_placar",
        contract: Contract::Value(Kind::Bool),
        description: "se o placar deve aparecer agora, conforme as regras do modelo",
        example: "true",
    },
    Entry {
        name: "opcoes",
        contract: Contract::Records(&[("texto", Kind::Text), ("correta", Kind::Bool)]),
        description: "opções da pergunta de múltipla escolha",
        example: "Pacífico | true",
    },
    Entry {
        name: "placar",
        contract: Contract::Records(&[
            ("nome", Kind::Text),
            ("pontos", Kind::Number),
            ("fora", Kind::Bool),
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
    pub deck: Deck,
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
pub struct ExternSuggestion {
    pub name: &'static str,
    pub detail: String,
    pub declaration: String,
}

pub fn present_suggestions() -> Vec<ExternSuggestion> {
    PRESENT_CONTRACT
        .iter()
        .map(|entry| ExternSuggestion {
            name: entry.name,
            detail: format!("{} · {}", entry.contract.label(), entry.description),
            declaration: declaration(entry),
        })
        .collect()
}

fn declaration(entry: &Entry) -> String {
    match entry.contract {
        Contract::Value(_) => format!("@extern {} = {}", entry.name, entry.example),
        Contract::Records(fields) => {
            let names: Vec<&str> = fields.iter().map(|(field, _)| *field).collect();
            let rule: Vec<&str> = fields.iter().map(|_| "---").collect();
            format!(
                "@data exemplo_{name}\n| {} |\n| {} |\n| {} |\n@end\n@extern {name} = $exemplo_{name}",
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
    PRESENT.get_or_init(|| ribalta::compile(DEFAULT_PRESENT))
}

pub fn present_deck(source: Option<&str>) -> Deck {
    match source {
        Some(source) => ribalta::compile(source).deck,
        None => present().deck.clone(),
    }
}

pub fn check_present(source: &str) -> ScreenCheck {
    let compilation = ribalta::compile(source);
    let mut diagnostics: Vec<ScreenDiagnostic> =
        compilation.diagnostics.iter().map(Into::into).collect();
    for name in &compilation.deck.externs {
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
        if let Some(problem) = mismatch(compilation.deck.state.get(name), contract) {
            diagnostics.push(error(
                "HUB2",
                format!("`${name}` {problem}"),
                format!("O hub envia `${name}` como {}", contract.label()),
            ));
        }
    }
    ScreenCheck {
        deck: compilation.deck,
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
