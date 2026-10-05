use css::style_origin::{RealLocation, StyleOrigin};
use oxc_allocator::Allocator;
use oxc_ast::ast::{
    CallExpression, Expression, ImportDeclarationSpecifier, ModuleExportName, Statement,
    VariableDeclarator,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::{GetSpan, SourceType};
use oxc_syntax::symbol::SymbolId;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const PACKET: &str = "__devup_source_v1";

pub(crate) fn json(origin: &StyleOrigin) -> String {
    format!(
        "{{\"file\":{},\"line\":{},\"col\":{},\"originalExpression\":{}}}",
        crate::vanilla_extract::json_string(&origin.file),
        origin.line,
        origin.column,
        crate::vanilla_extract::json_string(&origin.expression)
    )
}

pub(crate) fn mark_location(code: String, location: Option<&RealLocation>) -> String {
    match location {
        None => code,
        Some(location) => {
            let serialized = match location {
                RealLocation::Exact(origin) => format!("{{\"Exact\":{}}}", json(origin)),
                RealLocation::ProducedByCall(origin) => {
                    format!("{{\"ProducedByCall\":{}}}", json(origin))
                }
                RealLocation::ModuleExport { file, binding } => format!(
                    "{{\"ModuleExport\":{{\"file\":{},\"binding\":{}}}}}",
                    crate::vanilla_extract::json_string(file),
                    binding
                        .as_deref()
                        .map_or_else(|| "null".into(), crate::vanilla_extract::json_string)
                ),
            };
            format!(
                "{}({}, {code})",
                crate::style_origin::MARKER,
                crate::vanilla_extract::json_string(&serialized)
            )
        }
    }
}

pub(crate) fn instrument(code: &str, filename: &str, option: &crate::ExtractOption) -> String {
    instrument_with_edits(code, filename, option).0
}

pub(crate) fn instrument_with_edits(
    code: &str,
    filename: &str,
    option: &crate::ExtractOption,
) -> (String, Vec<crate::import_alias_visit::Edit>) {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename).unwrap_or_else(|_| SourceType::ts());
    let program = Parser::new(&allocator, code, source_type).parse().program;
    let scoping = SemanticBuilder::new()
        .build(&program)
        .semantic
        .into_scoping();
    let mut factories = BTreeSet::new();
    let mut namespaces = BTreeSet::new();
    for statement in &program.body {
        if let Statement::ImportDeclaration(import) = statement
            && (import.source.value.as_str() == option.package.as_str()
                || option
                    .import_aliases
                    .contains_key(import.source.value.as_str()))
            && let Some(specifiers) = &import.specifiers
        {
            for specifier in specifiers {
                match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(import) => {
                        let name = match &import.imported {
                            ModuleExportName::IdentifierName(name) => name.name.as_str(),
                            ModuleExportName::IdentifierReference(name) => name.name.as_str(),
                            ModuleExportName::StringLiteral(name) => name.value.as_str(),
                        };
                        if factory_name(name)
                            && let Some(symbol) = import.local.symbol_id.get()
                        {
                            factories.insert(symbol);
                        }
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(import) => {
                        if let Some(symbol) = import.local.symbol_id.get() {
                            namespaces.insert(symbol);
                        }
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {}
                }
            }
        }
    }
    loop {
        let before = factories.len();
        Aliases {
            scoping: &scoping,
            factories: &mut factories,
            namespaces: &namespaces,
        }
        .visit_program(&program);
        if factories.len() == before {
            break;
        }
    }
    let mut calls = Calls {
        file: filename,
        source: code,
        scoping: &scoping,
        factories: &factories,
        namespaces: &namespaces,
        insertions: BTreeMap::new(),
    };
    calls.visit_program(&program);
    let mut instrumented = code.to_string();
    let edits = calls
        .insertions
        .iter()
        .map(|(position, text)| (*position, *position, text.len()))
        .collect();
    for (position, text) in calls.insertions.into_iter().rev() {
        instrumented.insert_str(position, &text);
    }
    (instrumented, edits)
}

fn factory_name(name: &str) -> bool {
    matches!(name, "style" | "styleVariants" | "keyframes")
}

fn symbol(expression: &Expression<'_>, scoping: &Scoping) -> Option<SymbolId> {
    match crate::utils::unwrap_syntax_only(expression) {
        Expression::Identifier(name) => name
            .reference_id
            .get()
            .and_then(|reference| scoping.get_reference(reference).symbol_id()),
        _ => None,
    }
}

struct ApiBindings<'a> {
    scoping: &'a Scoping,
    factories: &'a BTreeSet<SymbolId>,
    namespaces: &'a BTreeSet<SymbolId>,
}
impl ApiBindings<'_> {
    fn factory(&self, expression: &Expression<'_>) -> bool {
        match crate::utils::unwrap_syntax_only(expression) {
            Expression::Identifier(_) => symbol(expression, self.scoping)
                .is_some_and(|symbol| self.factories.contains(&symbol)),
            Expression::StaticMemberExpression(member) => {
                factory_name(member.property.name.as_str())
                    && symbol(&member.object, self.scoping)
                        .is_some_and(|symbol| self.namespaces.contains(&symbol))
            }
            Expression::ComputedMemberExpression(member) => {
                crate::utils::get_string_by_literal_expression(&member.expression)
                    .is_some_and(|name| factory_name(&name))
                    && symbol(&member.object, self.scoping)
                        .is_some_and(|symbol| self.namespaces.contains(&symbol))
            }
            _ => false,
        }
    }
}

struct Aliases<'a> {
    scoping: &'a Scoping,
    factories: &'a mut BTreeSet<SymbolId>,
    namespaces: &'a BTreeSet<SymbolId>,
}
impl<'a> Visit<'a> for Aliases<'_> {
    fn visit_variable_declarator(&mut self, declaration: &VariableDeclarator<'a>) {
        if let Some(symbol) = declaration
            .id
            .get_binding_identifier()
            .and_then(|binding| binding.symbol_id.get())
            && self.scoping.symbol_flags(symbol).is_const_variable()
            && let Some(init) = &declaration.init
            && (ApiBindings {
                scoping: self.scoping,
                factories: self.factories,
                namespaces: self.namespaces,
            })
            .factory(init)
        {
            self.factories.insert(symbol);
        }
        walk::walk_variable_declarator(self, declaration);
    }
}

struct Calls<'a> {
    file: &'a str,
    source: &'a str,
    scoping: &'a Scoping,
    factories: &'a BTreeSet<SymbolId>,
    namespaces: &'a BTreeSet<SymbolId>,
    insertions: BTreeMap<usize, String>,
}
impl<'a> Visit<'a> for Calls<'_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if (ApiBindings {
            scoping: self.scoping,
            factories: self.factories,
            namespaces: self.namespaces,
        })
        .factory(&call.callee)
            && let Some(origin) =
                crate::style_origin::evaluated_at(self.file, self.source, call.span)
            && let Ok(end) = usize::try_from(call.span.end)
        {
            let mut packet = if call.arguments.is_empty() {
                String::from("undefined,undefined,")
            } else {
                String::from(",undefined,undefined,")
            };
            packet.push_str(&crate::vanilla_extract::json_string(PACKET));
            packet.push(',');
            packet.push_str(&crate::vanilla_extract::json_string(&json(&origin)));
            let position = call.arguments.last().map_or(end - 1, |argument| {
                usize::try_from(argument.span().end).unwrap_or(end - 1)
            });
            self.insertions
                .entry(position)
                .or_default()
                .push_str(&packet);
        }
        walk::walk_call_expression(self, call);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_packets_preserve_call_evaluation_parentheses_and_trailing_comments() {
        // Given
        let source = "import {style} from '@devup-ui/react';const alias=style;alias(({color:eval('value')}),/* , */);";
        // When
        let instrumented = instrument(source, "actual.css.ts", &crate::ExtractOption::default());
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, &instrumented, SourceType::ts()).parse();
        // Then
        assert_eq!(parsed.diagnostics.len(), 0, "{instrumented}");
        assert!(!instrumented.contains("=>"), "{instrumented}");
        assert!(
            instrumented.contains("alias(({color:eval('value')}),undefined,undefined,"),
            "{instrumented}"
        );
        assert!(instrumented.contains(PACKET), "{instrumented}");
    }

    #[test]
    fn a_shadowed_style_function_does_not_receive_compiler_arguments() {
        // Given
        let source = "import {style} from '@devup-ui/react';function run(style){return style({color:'red'});}const custom=(...args)=>args.length;run(custom);";
        // When
        let instrumented = instrument(source, "shadow.css.ts", &crate::ExtractOption::default());
        // Then
        assert_eq!(instrumented, source);
    }
}
