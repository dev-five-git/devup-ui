use oxc_allocator::Allocator;
use oxc_ast::ast::{Declaration, Statement, VariableDeclarationKind};
use oxc_codegen::{Codegen, Context, Gen};
use oxc_parser::Parser;
use oxc_span::SourceType;

#[derive(Debug, PartialEq, Eq)]
enum Part {
    Declarator {
        kind: VariableDeclarationKind,
        exported: bool,
        code: String,
    },
    Statement(String),
}

fn canonical_node(node: &impl Gen) -> String {
    let mut codegen = Codegen::new().with_source_type(SourceType::tsx());
    node.print(&mut codegen, Context::default());
    codegen.into_source_text()
}

fn parts(source: &str) -> Vec<Part> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{source}");
    let mut parts = vec![];
    for statement in &parsed.program.body {
        let declaration = match statement {
            Statement::VariableDeclaration(declaration) => Some((declaration.as_ref(), false)),
            Statement::ExportDeclaration(export) => match &export.declaration {
                Declaration::VariableDeclaration(declaration) => Some((declaration.as_ref(), true)),
                _ => None,
            },
            _ => None,
        };
        match declaration {
            Some((declaration, exported)) => {
                parts.extend(
                    declaration
                        .declarations
                        .iter()
                        .map(|declarator| Part::Declarator {
                            kind: declaration.kind,
                            exported,
                            code: canonical_node(declarator),
                        }),
                );
            }
            None => parts.push(Part::Statement(canonical_node(statement))),
        }
    }
    parts
}

pub(super) fn assert_preserved(actual: &str, expected: &str) {
    let actual_parts = parts(actual);
    let mut remaining = actual_parts.iter();
    for expected in parts(expected) {
        assert!(
            remaining.any(|actual| *actual == expected),
            "missing or reordered {expected:?} in {actual}"
        );
    }
}

#[test]
fn canonical_parts_match_when_only_quotes_spacing_and_declaration_groups_change() {
    // Given
    let authored = "const browser=window.document, handler=()=>document.body.append(window.name);export const view=<button onClick={handler}>{window.name}</button>;throw new Error('runtime only');";
    let expected = parts(authored);
    let counterfeit = parts(
        "const browser='window.document';const handler=()=>{};export const view=null;throw new Error('runtime only');",
    );
    // When
    let canonical = parts(
        "const browser = window.document;const handler = () => document.body.append(window.name);export const view = <button onClick={handler}>{window.name}</button>;throw new Error(\"runtime only\");",
    );
    // Then
    assert_eq!(canonical.len(), 4);
    assert_eq!(canonical, expected);
    assert_ne!(canonical, counterfeit);
}
