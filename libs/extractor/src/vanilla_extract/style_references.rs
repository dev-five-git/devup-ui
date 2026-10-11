use oxc_ast::ast::{Declaration, Expression, Program, Statement};
use rustc_hash::FxHashMap;

/// Final producer values and their declaration-free selector identities.
#[derive(Debug, Default, Clone)]
pub(crate) struct StyleReferences(FxHashMap<String, String>);

impl StyleReferences {
    pub(crate) fn register(&mut self, value: String, reference: String) {
        self.0.insert(value, reference);
    }

    pub(crate) fn merge(&mut self, other: Self) {
        self.0.extend(other.0);
    }

    fn ordered(&self) -> Vec<(&str, &str)> {
        let mut entries: Vec<_> = self
            .0
            .iter()
            .map(|(value, reference)| (value.as_str(), reference.as_str()))
            .collect();
        entries.sort_by(|left, right| {
            right
                .0
                .len()
                .cmp(&left.0.len())
                .then_with(|| left.0.cmp(right.0))
        });
        entries
    }

    pub(crate) fn contains_class_list(&self, value: &str) -> bool {
        self.ordered().iter().any(|(classes, _)| {
            value.match_indices(classes).any(|(start, _)| {
                token_boundary(value, start) && token_boundary(value, start + classes.len())
            })
        })
    }

    pub(crate) fn tokens(&self, value: &str) -> Option<Vec<(String, bool)>> {
        let entries = self.ordered();
        let mut ranges = Vec::new();
        for (classes, _) in entries {
            for (start, _) in value.match_indices(classes) {
                let end = start + classes.len();
                if token_boundary(value, start) && token_boundary(value, end) {
                    ranges.push(start..end);
                }
            }
        }
        if ranges.is_empty() {
            return None;
        }
        let mut tokens = Vec::new();
        let mut offset = 0;
        for token in value.split_whitespace() {
            let start = value[offset..].find(token).map_or(offset, |at| offset + at);
            let end = start + token.len();
            tokens.push((
                token.to_string(),
                ranges
                    .iter()
                    .any(|range| range.start <= start && range.end >= end),
            ));
            offset = end;
        }
        Some(tokens)
    }

    pub(crate) fn selector(&self, selector: &str) -> String {
        let mut entries = self.ordered();
        let mut identities: Vec<_> = self.0.values().map(String::as_str).collect();
        identities.sort_unstable();
        identities.dedup();
        for identity in identities {
            if !entries.iter().any(|(value, _)| *value == identity) {
                entries.push((identity, identity));
            }
        }
        entries.sort_by(|left, right| {
            right
                .0
                .len()
                .cmp(&left.0.len())
                .then_with(|| left.0.cmp(right.0))
        });
        let mut result = String::with_capacity(selector.len());
        let mut position = 0;
        let mut quote = None;
        let mut escaped = false;
        while position < selector.len() {
            let Some(character) = selector[position..].chars().next() else {
                break;
            };
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if let Some(delimiter) = quote {
                if character == delimiter {
                    quote = None;
                }
            } else if matches!(character, '\'' | '"') {
                quote = Some(character);
            } else if let Some((value, reference)) = entries
                .iter()
                .find(|(value, _)| selector[position..].starts_with(value))
            {
                if !selector[..position].ends_with('.') {
                    result.push('.');
                }
                result.push_str(reference);
                position += value.len();
                continue;
            }
            result.push(character);
            position += character.len_utf8();
        }
        result
    }
}

fn token_boundary(value: &str, at: usize) -> bool {
    at == 0
        || at == value.len()
        || value[..at]
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace)
        || value[at..].chars().next().is_some_and(char::is_whitespace)
}

pub(crate) fn record_bindings(
    program: &Program<'_>,
    bindings: &FxHashMap<String, String>,
    references: &mut StyleReferences,
) {
    for statement in &program.body {
        let declarations = match statement {
            Statement::VariableDeclaration(declaration) => &declaration.declarations,
            Statement::ExportDeclaration(export) => {
                let Declaration::VariableDeclaration(declaration) = &export.declaration else {
                    continue;
                };
                &declaration.declarations
            }
            _ => continue,
        };
        for declaration in declarations {
            if let Some(identifier) = declaration.id.get_binding_identifier()
                && let Some(reference) = bindings.get(identifier.name.as_str())
                && let Some(Expression::StringLiteral(value)) = &declaration.init
            {
                references.register(value.value.to_string(), reference.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests;
