use std::collections::BTreeSet;

use oxc_allocator::Allocator;
use oxc_ast::{AstKind, ast::Statement};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rustc_hash::FxHashMap;

use super::edits::{Replacement, apply};
use crate::vanilla_extract::{
    CollectedStyles, collected_styles_to_code_with_keyframes, keyframes_to_code,
    referenced_keyframes,
};

pub(crate) struct Lowered {
    pub code: String,
    pub css: Option<String>,
}

pub(crate) fn lower(
    stylesheet: crate::vanilla_extract::Stylesheet<'_>,
    collected: &CollectedStyles,
    option: &crate::ExtractOption,
) -> Result<Lowered, String> {
    let referenced = referenced_keyframes(collected);
    let names = if referenced.is_empty() {
        FxHashMap::default()
    } else {
        crate::extract_class_map_from_code(
            stylesheet.filename,
            &keyframes_to_code(collected, &option.package, &referenced),
            option,
            &referenced,
        )
        .map_err(|error| error.to_string())?
    };
    Ok(Lowered {
        code: collected_styles_to_code_with_keyframes(collected, &option.package, &names),
        css: None,
    })
}

pub(super) fn hygienic(
    lowered: Lowered,
    reserved: &mut BTreeSet<String>,
) -> Result<Lowered, String> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &lowered.code, SourceType::mjs()).parse();
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program)
        .semantic;
    let mut edits = Vec::new();
    let mut css = None;
    for statement in &parsed.program.body {
        let Statement::ImportDeclaration(import) = statement else {
            continue;
        };
        for specifier in import.specifiers.iter().flatten() {
            let local = specifier.local();
            let Some(symbol) = local.symbol_id.get() else {
                continue;
            };
            let mut name = format!("__ve_{}__", local.name);
            while reserved.contains(&name) {
                name.push('_');
            }
            reserved.insert(name.clone());
            if local.name == "css" {
                css = Some(name.clone());
            }
            edits.push(Replacement {
                span: local.span,
                text: format!("{} as {name}", local.name),
            });
            for reference in semantic.scoping().get_resolved_reference_ids(symbol) {
                let node = semantic.scoping().get_reference(*reference).node_id();
                if let AstKind::IdentifierReference(identifier) = semantic.nodes().kind(node) {
                    edits.push(Replacement {
                        span: identifier.span,
                        text: name.clone(),
                    });
                }
            }
        }
    }
    let (code, _) = apply(&lowered.code, edits).map_err(|error| error.message)?;
    Ok(Lowered { code, css })
}

pub(super) fn literal(expression: &str, reserved: &BTreeSet<String>) -> Result<String, String> {
    if !reserved.contains("undefined") {
        return Ok(expression.to_string());
    }
    let code = format!("({expression});");
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &code, SourceType::mjs()).parse();
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program)
        .semantic;
    let edits = semantic
        .nodes()
        .iter()
        .filter_map(|node| match node.kind() {
            AstKind::IdentifierReference(identifier) if identifier.name == "undefined" => {
                Some(Replacement {
                    span: identifier.span,
                    text: "(void 0)".into(),
                })
            }
            _ => None,
        })
        .collect();
    apply(&code, edits)
        .map(|(code, _)| code.trim_end_matches(';').to_string())
        .map_err(|error| error.message)
}
