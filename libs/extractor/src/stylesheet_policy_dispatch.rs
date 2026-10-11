//! Parsing and dispatch of the stylesheet's nonthrowing proof.

use oxc_allocator::Allocator;
use oxc_ast::ast::{Program, Statement};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rustc_hash::{FxHashMap, FxHashSet};

use super::{Plan, Proof, Value};
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
    Dispatch::new(option, resolver).plan(code, filename, is_static)
}

/// Proves the imported modules before value computation can inline their exports.
pub(crate) fn imports_plain(
    code: &str,
    filename: &str,
    option: &ExtractOption,
    resolver: Option<&ModuleResolver>,
) -> bool {
    let allocator = Allocator::default();
    let Ok(source_type) = SourceType::from_path(filename) else {
        return false;
    };
    let parsed = Parser::new(&allocator, code, source_type).parse();
    !parsed.fatal_error
        && parsed.diagnostics.is_empty()
        && Dispatch::new(option, resolver).imports(&parsed.program, filename)
}

struct Dispatch<'s> {
    option: &'s ExtractOption,
    resolver: Option<&'s ModuleResolver>,
    visiting: FxHashSet<String>,
    proven: FxHashMap<String, bool>,
}

impl<'s> Dispatch<'s> {
    fn new(option: &'s ExtractOption, resolver: Option<&'s ModuleResolver>) -> Self {
        Self {
            option,
            resolver,
            visiting: FxHashSet::default(),
            proven: FxHashMap::default(),
        }
    }

    fn imports(&mut self, program: &Program<'_>, filename: &str) -> bool {
        program.body.iter().all(|statement| {
            let Statement::ImportDeclaration(import) = statement else {
                return true;
            };
            let specifier = import.source.value.as_str();
            if import.import_kind.is_type()
                || specifier == self.option.package
                || specifier == format!("{}/compat", self.option.package)
            {
                return true;
            }
            let Some(module) = self
                .resolver
                .and_then(|resolve| resolve(specifier, filename))
            else {
                return false;
            };
            if let Some(proven) = self.proven.get(&module.path) {
                return *proven;
            }
            if !self.visiting.insert(module.path.clone()) {
                return false;
            }
            let proven = self.plan(&module.code, &module.path, &|_| false) == Plan::Plain;
            self.visiting.remove(&module.path);
            self.proven.insert(module.path, proven);
            proven
        })
    }

    fn plan(&mut self, code: &str, filename: &str, is_static: &dyn Fn(&str) -> bool) -> Plan {
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
        if !self.imports(&parsed.program, filename) {
            return Plan::Run;
        }
        let option = self.option;
        let changes = ChangeCheck::new(&parsed.program, filename, option, self.resolver);
        let known = |name: &str| {
            if is_static(name) {
                return Some(Value::Scalar);
            }
            if changes.is_changed(name) {
                return None;
            }
            literal(&changes.known(name)?)
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
}

fn literal(code: &str) -> Option<Value> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, SourceType::default())
        .parse_expression()
        .ok()?;
    let scoping = oxc_semantic::Scoping::default();
    let proof = Proof {
        scoping: &scoping,
        apis: FxHashMap::default(),
        namespaces: FxHashSet::default(),
        values: FxHashMap::default(),
    };
    proof.value(&parsed)
}
