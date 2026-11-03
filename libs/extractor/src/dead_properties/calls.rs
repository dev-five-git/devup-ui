//! Authored styling-call metadata carried through stylesheet execution.

use super::analysis::Analysis;
use super::objects::{ObjectKind, authored_errors};
use oxc_ast::ast::{
    CallExpression, Expression, ImportDeclarationSpecifier, Statement, TaggedTemplateExpression,
};
use oxc_ast_visit::{
    Visit,
    walk::{walk_call_expression, walk_tagged_template_expression},
};
use oxc_span::GetSpan;
use rustc_hash::FxHashMap;

pub(super) fn collect(
    stylesheet: crate::vanilla_extract::Stylesheet<'_>,
    package: &str,
) -> Analysis {
    let crate::vanilla_extract::Stylesheet {
        filename,
        code,
        source,
        edits,
    } = stylesheet;
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(
        &allocator,
        code,
        oxc_span::SourceType::from_path(filename)
            .unwrap_or_else(|_| oxc_span::SourceType::ts())
            .with_module(true),
    )
    .parse();
    let semantic = oxc_semantic::SemanticBuilder::new()
        .build(&parsed.program)
        .semantic;
    let mut imports = FxHashMap::default();
    for statement in &parsed.program.body {
        if let Statement::ImportDeclaration(import) = statement
            && (import.source.value == package || import.source.value == "@vanilla-extract/css")
        {
            for specifier in import.specifiers.iter().flatten() {
                let export = match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(named) => {
                        named.imported.name().to_string()
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => "*".to_string(),
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => continue,
                };
                if let Some(symbol) = specifier.local().symbol_id.get() {
                    imports.insert(symbol, export);
                }
            }
        }
    }
    let mut visitor = Calls {
        filename,
        source,
        edits,
        imports,
        scoping: semantic.scoping(),
        insertions: Vec::new(),
        errors: Vec::new(),
        ranges: Vec::new(),
    };
    loop {
        let previous = visitor.imports.len();
        Aliases(&mut visitor).visit_program(&parsed.program);
        if visitor.imports.len() == previous {
            break;
        }
    }
    visitor.visit_program(&parsed.program);
    Analysis {
        insertions: visitor.insertions,
        errors: visitor.errors,
        ranges: visitor.ranges,
    }
}

struct Calls<'s> {
    filename: &'s str,
    source: &'s str,
    edits: &'s [&'s [crate::import_alias_visit::Edit]],
    imports: FxHashMap<oxc_syntax::symbol::SymbolId, String>,
    scoping: &'s oxc_semantic::Scoping,
    insertions: Vec<(u32, String)>,
    errors: Vec<(u32, String)>,
    ranges: Vec<(u32, u32)>,
}

struct Aliases<'v, 's>(&'v mut Calls<'s>);

impl<'a> Visit<'a> for Aliases<'_, '_> {
    fn visit_variable_declaration(&mut self, declaration: &oxc_ast::ast::VariableDeclaration<'a>) {
        if declaration.kind == oxc_ast::ast::VariableDeclarationKind::Const {
            for declarator in &declaration.declarations {
                if let Some(identifier) = declarator.id.get_binding_identifier()
                    && let Some(symbol) = identifier.symbol_id.get()
                    && let Some(initializer) = &declarator.init
                    && let Some(api) = self.0.api(initializer)
                {
                    self.0.imports.insert(symbol, api);
                }
            }
        }
        oxc_ast_visit::walk::walk_variable_declaration(self, declaration);
    }
}

impl Calls<'_> {
    fn api(&self, expression: &Expression<'_>) -> Option<String> {
        let (identifier, member) = match crate::utils::unwrap_syntax_only(expression) {
            Expression::Identifier(identifier) => (identifier.as_ref(), None),
            Expression::StaticMemberExpression(member) => match &member.object {
                Expression::Identifier(identifier) => {
                    (identifier.as_ref(), Some(member.property.name.to_string()))
                }
                _ => return None,
            },
            Expression::ComputedMemberExpression(member) => match &member.object {
                Expression::Identifier(identifier) => (
                    identifier.as_ref(),
                    Some(member.static_property_name()?.as_str()?.to_string()),
                ),
                _ => return None,
            },
            _ => return None,
        };
        let symbol = self
            .scoping
            .get_reference(identifier.reference_id.get()?)
            .symbol_id()?;
        let imported = self.imports.get(&symbol)?;
        match (imported.as_str(), member) {
            ("*", Some(member)) => Some(member),
            ("styled", Some(_)) => Some("styled".to_string()),
            (_, None) => Some(imported.clone()),
            _ => None,
        }
    }

    fn location(&self, offset: u32) -> String {
        let offset = self
            .edits
            .iter()
            .fold(usize::try_from(offset).unwrap_or(0), |offset, edits| {
                crate::import_alias_visit::source_offset(edits, offset)
            });
        crate::locate(self.filename, self.source, offset)
    }
}

impl<'a> Visit<'a> for Calls<'_> {
    fn visit_tagged_template_expression(&mut self, tagged: &TaggedTemplateExpression<'a>) {
        let tag = crate::utils::unwrap_syntax_only(&tagged.tag);
        if self.api(tag).as_deref() == Some("css") {
            // Keep the template site and its eval/await/yield expressions outside the forwarding closure.
            let receiver = match tag {
                Expression::StaticMemberExpression(member) => {
                    crate::utils::readable_code(&member.object)
                }
                Expression::ComputedMemberExpression(member) => {
                    crate::utils::readable_code(&member.object)
                }
                _ => "undefined".to_string(),
            };
            self.ranges.push((tagged.span.start, tagged.span.end));
            self.insertions.push((tagged.tag.span().start, format!(
                "((__receiver, __tag) => (...__args) => __vanilla_extract__.__at({:?}, \"css\", __tag, __receiver, __args))({receiver}, (",
                self.location(tagged.span.start),
            )));
            self.insertions
                .push((tagged.tag.span().end, "))".to_string()));
        }
        walk_tagged_template_expression(self, tagged);
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Some(api) = self.api(&call.callee) {
            let shape = match api.as_str() {
                "style" | "css" | "fontFace" => Some((0, ObjectKind::Styles)),
                "globalStyle" | "globalFontFace" => Some((1, ObjectKind::Styles)),
                "keyframes" | "styleVariants" => Some((0, ObjectKind::Records)),
                "globalCss" => Some((0, ObjectKind::Globals)),
                "styled" => Some((call.arguments.len().saturating_sub(1), ObjectKind::Styles)),
                _ => None,
            };
            if let Some((argument, kind)) = shape {
                self.ranges.push((call.span.start, call.span.end));
                if !(api == "styleVariants" && call.arguments.len() > 1)
                    && let Some(expression) = call
                        .arguments
                        .get(argument)
                        .and_then(|argument| argument.as_expression())
                {
                    for (offset, path, requirement) in authored_errors(expression, kind) {
                        self.errors.push((
                            offset,
                            crate::utils::build_time_error(&api, &path, requirement),
                        ));
                    }
                }
                self.insertions.push((
                    call.span.start,
                    format!(
                        "__vanilla_extract__.__at({:?}, {:?}, () => (",
                        self.location(call.span.start),
                        api
                    ),
                ));
                walk_call_expression(self, call);
                self.insertions.push((call.span.end, "))".to_string()));
                return;
            }
        }
        let mut suspends = crate::utils::Suspends::default();
        for argument in &call.arguments {
            if let Some(expression) = argument.as_expression() {
                Visit::visit_expression(&mut suspends, expression);
            }
        }
        let direct_eval =
            matches!(&call.callee, Expression::Identifier(identifier) if identifier.name == "eval");
        if !direct_eval && !suspends.found {
            self.insertions.push((
                call.span.start,
                format!(
                    "__vanilla_extract__.__at({:?}, {:?}, () => (",
                    self.location(call.span.start),
                    crate::utils::readable_code(&call.callee),
                ),
            ));
            walk_call_expression(self, call);
            self.insertions.push((call.span.end, "))".to_string()));
            return;
        }
        walk_call_expression(self, call);
    }
}
