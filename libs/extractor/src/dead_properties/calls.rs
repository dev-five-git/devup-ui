//! Authored styling-call metadata carried through stylesheet execution.

use super::objects::{ObjectKind, authored_errors};
use oxc_ast::ast::{CallExpression, Expression, ImportDeclarationSpecifier, Statement};
use oxc_ast_visit::{Visit, walk::walk_call_expression};
use rustc_hash::FxHashMap;

pub(crate) struct Instrumented {
    pub code: String,
    pub calls: Vec<(u32, u32)>,
}

pub(crate) fn instrument(
    code: &str,
    filename: &str,
    package: &str,
) -> Result<Instrumented, String> {
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(
        &allocator,
        code,
        oxc_span::SourceType::from_path(filename).unwrap_or_else(|_| oxc_span::SourceType::ts()),
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
        code,
        filename,
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
    if !visitor.errors.is_empty() {
        return Err(visitor.errors.join("\n"));
    }
    visitor.insertions.sort_by_key(|(offset, _)| *offset);
    let mut output = String::new();
    let mut copied = 0;
    for (offset, insertion) in visitor.insertions {
        let offset = usize::try_from(offset).map_err(|error| error.to_string())?;
        output.push_str(&code[copied..offset]);
        output.push_str(&insertion);
        copied = offset;
    }
    output.push_str(&code[copied..]);
    Ok(Instrumented {
        code: output,
        calls: visitor.ranges,
    })
}

struct Calls<'s> {
    code: &'s str,
    filename: &'s str,
    imports: FxHashMap<oxc_syntax::symbol::SymbolId, String>,
    scoping: &'s oxc_semantic::Scoping,
    insertions: Vec<(u32, String)>,
    errors: Vec<String>,
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
        let (identifier, member) = match expression {
            Expression::Identifier(identifier) => (identifier.as_ref(), None),
            Expression::StaticMemberExpression(member) => match &member.object {
                Expression::Identifier(identifier) => {
                    (identifier.as_ref(), Some(member.property.name.as_str()))
                }
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
            ("*", Some(member)) => Some(member.to_string()),
            ("styled", Some(_)) => Some("styled".to_string()),
            (_, None) => Some(imported.clone()),
            _ => None,
        }
    }

    fn location(&self, offset: u32) -> String {
        let prefix = &self.code[..usize::try_from(offset).unwrap_or(0)];
        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let column = prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1;
        format!("{}:{line}:{column}", self.filename)
    }
}

impl<'a> Visit<'a> for Calls<'_> {
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
                        self.errors.push(format!(
                            "{}: {}",
                            self.location(offset),
                            crate::utils::build_time_error(&api, &path, requirement)
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
