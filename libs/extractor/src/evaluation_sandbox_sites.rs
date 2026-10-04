//! Lexically bound source instrumentation for exact sandbox read sites.

use std::hash::{Hash, Hasher};

use oxc_ast::ast::{Expression, IdentifierReference, ObjectProperty};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::{GetSpan, SourceType, Span};

/// Source edits and read coordinates, reusable before module mapper code generation.
pub(crate) struct Instrumented {
    pub(crate) code: String,
    pub(crate) edits: Vec<crate::import_alias_visit::Edit>,
    pub(super) sites: Vec<(String, u32)>,
    pub(super) helper: String,
    source: String,
    path: String,
}

struct Reads<'s> {
    scoping: &'s Scoping,
    source: &'s str,
    helper: &'s str,
    changes: Vec<(Span, String)>,
    sites: Vec<(String, u32)>,
    path: &'s str,
}

impl Reads<'_> {
    fn site(&mut self, span: Span) -> usize {
        let id = self.sites.len();
        self.sites.push((
            crate::locate(self.path, self.source, span.start as usize),
            span.start,
        ));
        id
    }

    fn external(&self, identifier: &IdentifierReference<'_>) -> bool {
        let Some(reference) = identifier.reference_id.get() else {
            return false;
        };
        let reference = self.scoping.get_reference(reference);
        reference.symbol_id().is_none()
            && reference.is_read()
            && !reference.is_write()
            && !super::guards::EXACT_GLOBALS.contains(&identifier.name.as_str())
    }

    fn write(&mut self, span: Span) {
        let id = self.site(span);
        self.changes.push((
            Span::new(span.start, span.start),
            format!("({}({id}, undefined), ", self.helper),
        ));
        self.changes
            .push((Span::new(span.end, span.end), ")".to_string()));
    }
}

impl<'a> Visit<'a> for Reads<'_> {
    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        if self.external(identifier) {
            let id = self.site(identifier.span);
            self.changes.push((
                identifier.span,
                format!("({}({id}, () => {}, true))", self.helper, identifier.name),
            ));
        }
    }

    fn visit_object_property(&mut self, property: &ObjectProperty<'a>) {
        if property.shorthand
            && let Expression::Identifier(identifier) = &property.value
            && self.external(identifier)
        {
            let id = self.site(identifier.span);
            self.changes.push((
                identifier.span,
                format!(
                    "{}: {}({id}, () => {}, true)",
                    identifier.name, self.helper, identifier.name
                ),
            ));
        } else {
            walk::walk_object_property(self, property);
        }
    }

    fn visit_static_member_expression(
        &mut self,
        member: &oxc_ast::ast::StaticMemberExpression<'a>,
    ) {
        let id = self.site(member.span);
        let access = Span::new(member.object.span().end, member.property.span.end);
        let optional = if member.optional { "?." } else { "" };
        self.changes.push((
            access,
            format!(
                "{optional}[{}({id}, \"{}\")]",
                self.helper, member.property.name
            ),
        ));
        walk::walk_static_member_expression(self, member);
    }

    fn visit_computed_member_expression(
        &mut self,
        member: &oxc_ast::ast::ComputedMemberExpression<'a>,
    ) {
        // The key may itself read globals; establish the member site after it evaluates.
        let id = self.site(member.span);
        let key = member.expression.span();
        self.changes.push((
            Span::new(key.start, key.start),
            format!("{}({id}, ", self.helper),
        ));
        self.changes
            .push((Span::new(key.end, key.end), ")".to_string()));
        walk::walk_computed_member_expression(self, member);
    }

    fn visit_call_expression(&mut self, call: &oxc_ast::ast::CallExpression<'a>) {
        self.visit_expression(&call.callee);
        for (index, argument) in call.arguments.iter().enumerate() {
            if index + 1 == call.arguments.len() && !argument.is_spread() {
                let id = self.site(call.span);
                let span = argument.span();
                self.changes.push((
                    Span::new(span.start, span.start),
                    format!("{}({id}, ", self.helper),
                ));
                self.changes
                    .push((Span::new(span.end, span.end), ")".to_string()));
            }
            self.visit_argument(argument);
        }
    }

    fn visit_binding_property(&mut self, property: &oxc_ast::ast::BindingProperty<'a>) {
        let id = self.site(property.key.span());
        let span = property.key.span();
        let written = &self.source[span.start as usize..span.end as usize];
        if property.computed {
            self.changes.push((
                Span::new(span.start, span.start),
                format!("{}({id}, ", self.helper),
            ));
            self.changes
                .push((Span::new(span.end, span.end), ")".to_string()));
        } else {
            let key = match &property.key {
                oxc_ast::ast::PropertyKey::StaticIdentifier(_) => format!("\"{written}\""),
                _ => written.to_string(),
            };
            let binding = if property.shorthand {
                format!(": {written}")
            } else {
                String::new()
            };
            self.changes
                .push((span, format!("[{}({id}, {key})]{binding}", self.helper)));
        }
        walk::walk_binding_property(self, property);
    }

    fn visit_assignment_expression(&mut self, assignment: &oxc_ast::ast::AssignmentExpression<'a>) {
        self.write(assignment.span);
        self.visit_assignment_target(&assignment.left);
        self.visit_expression(&assignment.right);
    }

    fn visit_update_expression(&mut self, update: &oxc_ast::ast::UpdateExpression<'a>) {
        self.write(update.span);
        walk::walk_update_expression(self, update);
    }
}

/// Adds identity calls to member keys and thunk calls around unbound identifiers.
/// Receivers, short-circuit optional chains and direct `eval` calls are unchanged.
pub(crate) fn instrument(source: &str, path: &str) -> Instrumented {
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, SourceType::default()).parse();
    let scoping = SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    let mut hash = rustc_hash::FxHasher::default();
    source.hash(&mut hash);
    path.hash(&mut hash);
    let helper =
        crate::fresh_name::fresh_name(&format!("__devup_read_site_{:x}__", hash.finish()), source);
    let mut reads = Reads {
        scoping: &scoping,
        source,
        helper: &helper,
        changes: Vec::new(),
        sites: Vec::new(),
        path,
    };
    if parsed.diagnostics.is_empty() {
        reads.visit_program(&parsed.program);
    }
    reads
        .changes
        .sort_by_key(|(span, _)| (span.start, span.end));
    let mut code = String::with_capacity(source.len());
    let mut edits = Vec::new();
    let mut copied = 0;
    for (span, text) in reads.changes {
        let (start, end) = (span.start as usize, span.end as usize);
        code.push_str(&source[copied..start]);
        code.push_str(&text);
        edits.push((start, end, text.len()));
        copied = end;
    }
    code.push_str(&source[copied..]);
    Instrumented {
        code,
        edits,
        sites: reads.sites,
        helper,
        source: source.to_string(),
        path: path.to_string(),
    }
}

impl Instrumented {
    /// Maps engine frames to the pre-instrumentation text; explicit read frames already name it.
    pub(crate) fn explain(&self, error: &str) -> String {
        error
            .lines()
            .map(|line| {
                if line.contains("at <read> (") {
                    return line.to_string();
                }
                let marker = format!("({}:", self.path);
                let Some((prefix, position)) = line.rsplit_once(&marker) else {
                    return line.to_string();
                };
                let Some((row, column)) = position.trim_end_matches(')').split_once(':') else {
                    return line.to_string();
                };
                let (Ok(row), Ok(column)) = (row.parse::<u32>(), column.parse::<u32>()) else {
                    return line.to_string();
                };
                let offset = crate::source_map::Lines::new(&self.code)
                    .code_point_offset(row.saturating_sub(1), column.saturating_sub(1));
                let offset = crate::import_alias_visit::source_offset(&self.edits, offset);
                format!(
                    "{prefix}({})",
                    crate::locate(&self.path, &self.source, offset)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
#[path = "evaluation_sandbox_sites_ci_tests.rs"]
mod ci_tests;
