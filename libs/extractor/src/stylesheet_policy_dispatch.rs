//! Parsing and dispatch of the stylesheet's nonthrowing proof.

use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rustc_hash::{FxHashMap, FxHashSet};

use super::{Plan, Proof, Value, values};
use crate::imported_constants::ChangeCheck;
use crate::{ExtractOption, ModuleResolver};

/// Decides whether extraction preserves every effect before anything runs.
pub(crate) fn plan(
    code: &str,
    filename: &str,
    option: &ExtractOption,
    resolver: Option<&ModuleResolver>,
    is_static: &dyn Fn(&str) -> bool,
) -> Plan {
    let allocator = Allocator::default();
    let Ok(source_type) = SourceType::from_path(filename) else {
        return Plan::Run;
    };
    let parsed = Parser::new(&allocator, code, source_type).parse();
    if parsed.fatal_error || !parsed.diagnostics.is_empty() {
        return Plan::Run;
    }
    let built = SemanticBuilder::new()
        .with_check_syntax_error(true)
        .build(&parsed.program);
    if !built.diagnostics.is_empty() {
        return Plan::Run;
    }
    let scoping = built.semantic.into_scoping();
    let changes = ChangeCheck::new(&parsed.program, filename, option, resolver);
    let known = |name: &str| {
        if is_static(name) {
            return Some(Value::Scalar);
        }
        if changes.is_changed(name) {
            return None;
        }
        values::literal(&changes.known(name)?)
    };
    let compat = format!("{}/compat", option.package);
    let mut proof = Proof {
        scoping: &scoping,
        apis: FxHashMap::default(),
        namespaces: FxHashSet::default(),
        values: FxHashMap::default(),
    };
    // Imports are bound before any statement runs.
    let imports = parsed.program.body.iter().all(|statement| match statement {
        Statement::ImportDeclaration(import) => {
            proof.import(import, [option.package.as_str(), compat.as_str()], &known)
        }
        _ => true,
    });
    if imports
        && parsed
            .program
            .body
            .iter()
            .all(|statement| proof.statement(statement))
    {
        Plan::Plain
    } else {
        Plan::Run
    }
}
