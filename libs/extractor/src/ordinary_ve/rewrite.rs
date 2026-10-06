use oxc_ast::ast::{Declaration, Program, Statement, VariableDeclaration};
use oxc_span::{GetSpan, Span};

use super::edits::Replacement;
use super::execution::{Executed, SelectedModule};
use super::selection::plan::{Selection, UnitKind};

pub(super) fn replacements(
    module: SelectedModule<'_>,
    program: &Program<'_>,
    result: &Executed,
) -> Vec<Replacement> {
    let selection = module.selection;
    let mut edits = Vec::new();
    for root in &selection.roots {
        let values: Vec<_> = result
            .captures
            .iter()
            .filter(|capture| capture.root == root.span)
            .collect();
        let declarator = selection
            .units
            .iter()
            .find(|unit| unit.node == root.owner)
            .is_some_and(|unit| matches!(unit.kind, UnitKind::Declarator { .. }));
        let default = program.body.iter().any(|statement| {
            matches!(statement,
            Statement::ExportDefaultDeclaration(export) if export.declaration.span() == root.span)
        });
        let mut text = if declarator {
            let bindings: Vec<_> = values
                .iter()
                .filter_map(|capture| {
                    let binding = capture.binding.as_ref()?;
                    Some(format!("{}={}", binding.name, capture.expression))
                })
                .collect();
            if bindings.is_empty() {
                let array = selection.units.iter().any(|unit| unit.node == root.owner && matches!(unit.kind,
                    UnitKind::Declarator { pattern, .. } if pattern.source_text(module.stylesheet.code).trim_start().starts_with('[')));
                if array {
                    "[]=[]".into()
                } else {
                    "{}={}".into()
                }
            } else {
                bindings.join(",")
            }
        } else if values.iter().any(|capture| capture.binding.is_some()) {
            let bindings: Vec<_> = values
                .iter()
                .filter_map(|capture| {
                    let binding = capture.binding.as_ref()?;
                    Some(format!("{}={}", binding.name, capture.expression))
                })
                .collect();
            format!("var {};", bindings.join(","))
        } else if let Some(capture) = values.first() {
            if default {
                capture.expression.clone()
            } else {
                format!("{};", capture.expression)
            }
        } else {
            "void 0;".into()
        };
        for (_, code) in result
            .emission
            .effects
            .iter()
            .filter(|(span, _)| *span == root.span)
        {
            text = if declarator {
                format!("[]=((()=>{{{code}}})(),[]),{text}")
            } else if default {
                format!("(()=>{{{code}return {text};}})()")
            } else {
                format!("{code}{text}")
            };
        }
        edits.push(Replacement {
            span: root.span,
            text,
        });
    }
    for statement in &program.body {
        let declaration = match statement {
            Statement::VariableDeclaration(declaration) => Some(declaration.as_ref()),
            Statement::ExportDeclaration(export) => match &export.declaration {
                Declaration::VariableDeclaration(declaration) => Some(declaration.as_ref()),
                _ => None,
            },
            _ => None,
        };
        if let Some(declaration) = declaration {
            edits.extend(remove_declarators(statement.span(), declaration, selection));
        } else if selection
            .consumed
            .iter()
            .any(|unit| unit.span == statement.span())
        {
            edits.push(Replacement {
                span: statement.span(),
                text: String::new(),
            });
        }
    }
    edits
}

fn remove_declarators(
    statement: Span,
    declaration: &VariableDeclaration<'_>,
    selection: &Selection,
) -> Vec<Replacement> {
    let declarations = &declaration.declarations;
    let removed: Vec<_> = declarations
        .iter()
        .map(|declarator| {
            selection.consumed.iter().any(|unit| {
                unit.span == declarator.span
                    && !selection.roots.iter().any(|root| root.owner == unit.node)
            })
        })
        .collect();
    if removed.iter().all(|removed| *removed) {
        return vec![Replacement {
            span: statement,
            text: String::new(),
        }];
    }
    let mut edits = Vec::new();
    let mut index = 0;
    while index < declarations.len() {
        if !removed[index] {
            index += 1;
            continue;
        }
        let first = index;
        while index < declarations.len() && removed[index] {
            index += 1;
        }
        let span = if first == 0 {
            Span::new(
                declarations[first].span.start,
                declarations[index].span.start,
            )
        } else {
            Span::new(
                declarations[first - 1].span.end,
                declarations[index - 1].span.end,
            )
        };
        edits.push(Replacement {
            span,
            text: String::new(),
        });
    }
    edits
}

pub(super) fn imports(
    program: &Program<'_>,
    source: &str,
    selection: &Selection,
) -> Vec<Replacement> {
    let mut edits = Vec::new();
    for statement in &program.body {
        let Statement::ImportDeclaration(import) = statement else {
            continue;
        };
        let specifiers: Vec<_> = import.specifiers.iter().flatten().collect();
        let remaining: Vec<_> = specifiers
            .iter()
            .filter(|specifier| {
                !selection.imports.iter().any(|binding| {
                    binding.native.is_some()
                        && binding.declaration == import.span
                        && binding.specifier == specifier.span()
                })
            })
            .collect();
        if remaining.len() == specifiers.len() {
            continue;
        }
        let text = if remaining.is_empty() {
            String::new()
        } else {
            let mut head = Vec::new();
            let mut named = Vec::new();
            for specifier in remaining {
                match specifier {
                    oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(_) => {
                        named.push(specifier.span().source_text(source));
                    }
                    oxc_ast::ast::ImportDeclarationSpecifier::ImportDefaultSpecifier(_)
                    | oxc_ast::ast::ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => {
                        head.push(specifier.span().source_text(source).to_string());
                    }
                }
            }
            if !named.is_empty() {
                head.push(format!("{{{}}}", named.join(",")));
            }
            format!(
                "import {} from {};",
                head.join(","),
                import.source.span.source_text(source)
            )
        };
        edits.push(Replacement {
            span: import.span,
            text,
        });
    }
    edits
}
