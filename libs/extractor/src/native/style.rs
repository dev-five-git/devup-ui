//! What the style props of one element declare, in the order they are
//! written, and the `style` value that applies them

use super::props::Declaration;
use super::sheet::Sheet;

/// The styles one prop declares
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Choice {
    /// Nothing, as `null` or `undefined` declare
    Empty,
    /// Styles the build knows, hoisted into the stylesheet
    Static(Vec<Declaration>),
    /// Styles whose values the element reads at runtime: each key with the
    /// code of its value
    Dynamic(Vec<(String, String)>),
    /// One choice or the other, by the code of a condition
    Condition {
        test: String,
        consequent: Box<Self>,
        alternate: Box<Self>,
    },
}

impl Choice {
    /// Whether the choice declares the React Native style `key`
    fn declares(&self, key: &str) -> bool {
        match self {
            Self::Empty => false,
            Self::Static(declarations) => declarations.iter().any(|(name, _)| name == key),
            Self::Dynamic(values) => values.iter().any(|(name, _)| name == key),
            Self::Condition {
                consequent,
                alternate,
                ..
            } => consequent.declares(key) || alternate.declares(key),
        }
    }

    /// The code of the choice, its static styles named in `sheet`
    fn code(&self, sheet: &mut Sheet) -> String {
        match self {
            Self::Empty => "null".to_string(),
            Self::Static(declarations) => sheet.reference(declarations),
            Self::Dynamic(values) => object_code(values),
            Self::Condition {
                test,
                consequent,
                alternate,
            } => {
                let test = operand(test);
                match alternate.as_ref() {
                    Self::Empty => format!("{test} && {}", consequent.operand_code(sheet)),
                    alternate => format!(
                        "{test} ? {} : {}",
                        consequent.operand_code(sheet),
                        alternate.operand_code(sheet)
                    ),
                }
            }
        }
    }

    /// The code of the choice as an operand of a condition
    fn operand_code(&self, sheet: &mut Sheet) -> String {
        let code = self.code(sheet);
        if matches!(self, Self::Condition { .. }) {
            format!("({code})")
        } else {
            code
        }
    }
}

/// `code` as an operand: a name or a member read as it is, else parenthesized
fn operand(code: &str) -> String {
    if code
        .chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '_' | '$' | '.'))
    {
        code.to_string()
    } else {
        format!("({code})")
    }
}

/// An object literal of `values`, each key with the code of its value
fn object_code(values: &[(String, String)]) -> String {
    let properties: Vec<String> = values
        .iter()
        .map(|(key, value)| format!("{key}: {value}"))
        .collect();
    format!("{{ {} }}", properties.join(", "))
}

/// `declarations` with `later` declared after them, a later key winning
fn merge(declarations: &mut Vec<Declaration>, later: Vec<Declaration>) {
    for (key, value) in later {
        match declarations.iter_mut().find(|(name, _)| *name == key) {
            Some(declaration) => declaration.1 = value,
            None => declarations.push((key, value)),
        }
    }
}

/// The choices of an element in the order they apply, with as many static
/// styles as can be moved first merged into one
fn ordered(choices: Vec<Choice>) -> Vec<Choice> {
    let mut first = Vec::new();
    let mut rest: Vec<Choice> = Vec::new();
    for choice in choices {
        match choice {
            Choice::Empty => {}
            Choice::Static(declarations) => {
                if declarations
                    .iter()
                    .all(|(key, _)| rest.iter().all(|choice| !choice.declares(key)))
                {
                    merge(&mut first, declarations);
                } else if let Some(Choice::Static(last)) = rest.last_mut() {
                    merge(last, declarations);
                } else {
                    rest.push(Choice::Static(declarations));
                }
            }
            choice => rest.push(choice),
        }
    }
    if !first.is_empty() {
        rest.insert(0, Choice::Static(first));
    }
    rest
}

/// The code of the `style` an element takes: its choices, then the code of
/// the styles it is given whole (a spread's `style`, its own `style` prop);
/// `None` when its props declare no style, and it keeps what it is given
pub(super) fn style_code(
    choices: Vec<Choice>,
    given: Vec<String>,
    sheet: &mut Sheet,
) -> Option<String> {
    let mut entries: Vec<String> = ordered(choices)
        .iter()
        .map(|choice| choice.code(sheet))
        .collect();
    if entries.is_empty() {
        return None;
    }
    entries.extend(given);
    Some(if entries.len() == 1 {
        entries.concat()
    } else {
        format!("[{}]", entries.join(", "))
    })
}
