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
    /// Styles the build knows for each theme, in the order of the theme
    /// names; the element picks one by the theme the app shows
    Themed(Vec<Vec<Declaration>>),
    /// A choice that applies only while the theme at `index` shows
    InTheme { index: usize, choice: Box<Self> },
}

/// The choice of `variants`, one a theme: static when every theme declares
/// the same styles
pub(super) fn themed(mut variants: Vec<Vec<Declaration>>) -> Choice {
    if variants.windows(2).all(|pair| pair[0] == pair[1]) {
        variants
            .pop()
            .filter(|declarations| !declarations.is_empty())
            .map_or(Choice::Empty, Choice::Static)
    } else {
        Choice::Themed(variants)
    }
}

/// `choice` applied only while the theme at `index` of `count` shows
pub(super) fn in_theme(index: usize, count: usize, choice: Choice) -> Choice {
    match choice {
        Choice::Empty => Choice::Empty,
        choice if count <= 1 => choice,
        Choice::Static(declarations) => Choice::Themed(
            (0..count)
                .map(|at| {
                    if at == index {
                        declarations.clone()
                    } else {
                        Vec::new()
                    }
                })
                .collect(),
        ),
        Choice::Themed(mut variants) => {
            for (at, variant) in variants.iter_mut().enumerate() {
                if at != index {
                    variant.clear();
                }
            }
            Choice::Themed(variants)
        }
        choice => Choice::InTheme {
            index,
            choice: Box::new(choice),
        },
    }
}

impl Choice {
    /// Whether the choice declares the React Native style `key`
    fn declares(&self, key: &str) -> bool {
        match self {
            Self::Empty => false,
            Self::Static(declarations) => declares(declarations, key),
            Self::Dynamic(values) => values.iter().any(|(name, _)| name == key),
            Self::Condition {
                consequent,
                alternate,
                ..
            } => consequent.declares(key) || alternate.declares(key),
            Self::Themed(variants) => variants
                .iter()
                .any(|declarations| declares(declarations, key)),
            Self::InTheme { choice, .. } => choice.declares(key),
        }
    }

    /// Whether the choice reads the theme the app shows
    pub(super) fn reads_theme(&self) -> bool {
        match self {
            Self::Empty | Self::Static(_) | Self::Dynamic(_) => false,
            Self::Condition {
                consequent,
                alternate,
                ..
            } => consequent.reads_theme() || alternate.reads_theme(),
            Self::Themed(_) | Self::InTheme { .. } => true,
        }
    }

    /// The code of the choice, its static styles named in `sheet` and the
    /// index of the theme read from `theme`
    fn code(&self, sheet: &mut Sheet, theme: &str) -> String {
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
                    Self::Empty => {
                        format!("{test} && {}", consequent.operand_code(sheet, theme))
                    }
                    alternate => format!(
                        "{test} ? {} : {}",
                        consequent.operand_code(sheet, theme),
                        alternate.operand_code(sheet, theme)
                    ),
                }
            }
            Self::Themed(variants) => {
                let references: Vec<String> = variants
                    .iter()
                    .map(|declarations| {
                        if declarations.is_empty() {
                            "null".to_string()
                        } else {
                            sheet.reference(declarations)
                        }
                    })
                    .collect();
                format!("[{}][{theme}]", references.join(", "))
            }
            Self::InTheme { index, choice } => {
                format!(
                    "{theme} === {index} && {}",
                    choice.operand_code(sheet, theme)
                )
            }
        }
    }

    /// The code of the choice as an operand of a condition
    fn operand_code(&self, sheet: &mut Sheet, theme: &str) -> String {
        let code = self.code(sheet, theme);
        if matches!(self, Self::Condition { .. } | Self::InTheme { .. }) {
            format!("({code})")
        } else {
            code
        }
    }
}

fn declares(declarations: &[Declaration], key: &str) -> bool {
    declarations.iter().any(|(name, _)| name == key)
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

/// `later` merged into the last of `rest` when both are static or both
/// differ by theme, else `later` itself to apply after it
fn merge_last(rest: &mut [Choice], later: Choice) -> Option<Choice> {
    match (rest.last_mut(), later) {
        (Some(Choice::Static(last)), Choice::Static(declarations)) => {
            merge(last, declarations);
            None
        }
        (Some(Choice::Themed(last)), Choice::Static(declarations)) => {
            for variant in last {
                merge(variant, declarations.clone());
            }
            None
        }
        (Some(Choice::Themed(last)), Choice::Themed(variants)) => {
            for (variant, later) in last.iter_mut().zip(variants) {
                merge(variant, later);
            }
            None
        }
        (_, later) => Some(later),
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
            Choice::Static(declarations)
                if declarations
                    .iter()
                    .all(|(key, _)| rest.iter().all(|choice| !choice.declares(key))) =>
            {
                merge(&mut first, declarations);
            }
            choice => {
                let unmerged = merge_last(&mut rest, choice);
                rest.extend(unmerged);
            }
        }
    }
    if !first.is_empty() {
        rest.insert(0, Choice::Static(first));
    }
    rest
}

/// The `style` an element takes, and whether it reads the theme the app shows
pub(super) struct StyleCode {
    pub code: String,
    pub reads_theme: bool,
}

/// The code of the `style` an element takes: its choices, then the code of
/// the styles it is given whole (a spread's `style`, its own `style` prop);
/// `None` when its props declare no style, and it keeps what it is given.
/// The index of the theme the app shows is read from `theme`
pub(super) fn style_code(
    choices: Vec<Choice>,
    given: Vec<String>,
    sheet: &mut Sheet,
    theme: &str,
) -> Option<StyleCode> {
    let choices = ordered(choices);
    let reads_theme = choices.iter().any(Choice::reads_theme);
    let mut entries: Vec<String> = choices
        .iter()
        .map(|choice| choice.code(sheet, theme))
        .collect();
    if entries.is_empty() {
        return None;
    }
    entries.extend(given);
    let code = if entries.len() == 1 {
        entries.concat()
    } else {
        format!("[{}]", entries.join(", "))
    };
    Some(StyleCode { code, reads_theme })
}
