//! Normal Devup extraction before a native-free stylesheet's required evaluation.
use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, ExpressionStatement, Program, ReturnStatement, VariableDeclarator};
use oxc_ast_visit::{Visit, VisitMut, walk};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{ExtractOption, ExtractStyleValue, ModuleResolver, vanilla_extract::Stylesheet};

#[derive(Default)]
pub(crate) struct Compiled {
    pub code: Option<String>,
    pub edits: Vec<crate::import_alias_visit::Edit>,
    pub styles: FxHashSet<ExtractStyleValue>,
}

pub(crate) fn compile(
    stylesheet: Stylesheet<'_>,
    option: &ExtractOption,
    resolver: Option<&ModuleResolver>,
) -> Result<Compiled, String> {
    let allocator = Allocator::default();
    let parsed = Parser::new(
        &allocator,
        stylesheet.code,
        SourceType::from_path(stylesheet.filename).unwrap_or_default(),
    )
    .parse();
    if !parsed.diagnostics.is_empty() {
        return Ok(Compiled::default());
    }
    let mut program = parsed.program;
    let semantic = oxc_semantic::SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&program)
        .semantic;
    let native = crate::ordinary_ve::selection::select_resolved(
        (&program, &semantic),
        (stylesheet.filename, &option.package),
        resolver,
    );
    if !native.native_calls.is_empty() {
        return Ok(Compiled::default());
    }
    let before = expressions(&program);
    let (bucket, global, _) = crate::resolve_css_target(stylesheet.filename, option);
    let mut visitor = crate::visit::DevupVisitor::new(
        &allocator,
        stylesheet.filename,
        &option.package,
        Vec::new(),
        if global { None } else { Some(bucket) },
    );
    visitor.visit_program(&mut program);
    if visitor.styles.is_empty() || !visitor.errors.is_empty() || !visitor.unknown_parts.is_empty()
    {
        return Ok(Compiled::default());
    }
    let after = expressions(&program);
    let mut replacements: Vec<_> = before
        .into_iter()
        .filter_map(|(key, (span, old))| {
            let (_, new) = after.get(&key)?;
            (old != *new).then(|| (span, new.clone()))
        })
        .collect();
    let spans: Vec<_> = replacements.iter().map(|(span, _)| *span).collect();
    replacements.retain(|(span, _)| {
        !spans
            .iter()
            .any(|child| child != span && span.contains_inclusive(*child))
    });
    replacements.sort_by_key(|(span, _)| span.start);
    let mut code = stylesheet.code.to_string();
    let edits = replacements
        .iter()
        .map(|(span, text)| {
            Ok((
                usize::try_from(span.start).map_err(|error| error.to_string())?,
                usize::try_from(span.end).map_err(|error| error.to_string())?,
                text.len(),
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    for (span, text) in replacements.into_iter().rev() {
        let from = usize::try_from(span.start).map_err(|error| error.to_string())?;
        let to = usize::try_from(span.end).map_err(|error| error.to_string())?;
        code.replace_range(from..to, &text);
    }
    Ok(Compiled {
        code: Some(code),
        edits,
        styles: visitor.styles,
    })
}

fn expressions(program: &Program<'_>) -> FxHashMap<u32, (Span, String)> {
    let mut expressions = Expressions(FxHashMap::default());
    expressions.visit_program(program);
    expressions.0
}

struct Expressions(FxHashMap<u32, (Span, String)>);

impl Expressions {
    fn record(&mut self, key: u32, expression: &Expression<'_>) {
        let text = match expression {
            Expression::Identifier(identifier) if identifier.name.is_empty() => {
                "void 0".to_string()
            }
            expression => crate::utils::readable_code(expression),
        };
        self.0.insert(key, (expression.span(), text));
    }
}

impl<'a> Visit<'a> for Expressions {
    fn visit_variable_declarator(&mut self, declaration: &VariableDeclarator<'a>) {
        if let Some(init) = &declaration.init {
            self.record(declaration.span.start, init);
        }
        walk::walk_variable_declarator(self, declaration);
    }

    fn visit_expression_statement(&mut self, statement: &ExpressionStatement<'a>) {
        self.record(statement.span.start, &statement.expression);
        walk::walk_expression_statement(self, statement);
    }

    fn visit_return_statement(&mut self, statement: &ReturnStatement<'a>) {
        if let Some(argument) = &statement.argument {
            self.record(statement.span.start, argument);
        }
        walk::walk_return_statement(self, statement);
    }
}
