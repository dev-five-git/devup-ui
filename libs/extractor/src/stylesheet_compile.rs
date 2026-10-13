//! Normal Devup extraction before an authored stylesheet's required evaluation.
use oxc_allocator::Allocator;
use oxc_ast::ast::{
    ExportDefaultDeclaration, Expression, ExpressionStatement, Program, ReturnStatement, Statement,
    VariableDeclarator,
};
use oxc_ast_visit::{Visit, VisitMut, walk};
use oxc_codegen::{Codegen, Gen};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{ExtractOption, ExtractStyleValue, ModuleResolver, vanilla_extract::Stylesheet};

mod components;
mod deferred;
mod eligibility;
pub(crate) use deferred::Evaluation;

#[derive(Default)]
pub(crate) struct Compiled {
    pub code: Option<String>,
    pub edits: Vec<crate::import_alias_visit::Edit>,
    pub styles: FxHashSet<ExtractStyleValue>,
    pub evaluation: Option<Evaluation>,
    pub scaffolding: Vec<String>,
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
    if !eligibility::required(&program, &option.package) {
        return Ok(Compiled::default());
    }
    let before = expressions(&program);
    let original_imports = imports(&program);
    let inlined = crate::imported_constants::inline_constants(
        &oxc_ast::builder::AstBuilder::new(&allocator),
        &mut program,
        stylesheet.filename,
        option,
        resolver,
        crate::css_prop::CssProp::Off,
    );
    if !inlined.errors.is_empty() {
        return Err(crate::located_errors(
            stylesheet.filename,
            stylesheet.source,
            stylesheet.edits,
            inlined.errors,
        ));
    }
    let (bucket, global, _) = crate::resolve_css_target(stylesheet.filename, option);
    let mut visitor = crate::visit::DevupVisitor::new(
        &allocator,
        stylesheet.filename,
        &option.package,
        Vec::new(),
        if global { None } else { Some(bucket) },
    );
    visitor.import_stylex(inlined.stylex_vars, inlined.stylex_themes);
    visitor.import_css(inlined.css_styles);
    visitor.import_producer_atoms(inlined.atoms);
    visitor.import_producer_references(inlined.references);
    visitor.unknown_bindings(&inlined.unknown);
    visitor.changed_bindings(inlined.changed);
    visitor.reuse_scoping(inlined.scoping);
    visitor.visit_program(&mut program);
    visitor.errors.append(&mut visitor.unknown_parts);
    if !visitor.errors.is_empty() {
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
    let generated = imports(&program);
    let generated_bindings = generated
        .keys()
        .filter(|binding| !original_imports.contains_key(*binding))
        .cloned()
        .collect();
    let generated_imports: FxHashSet<_> = generated
        .into_iter()
        .filter_map(|(binding, text)| (!original_imports.contains_key(&binding)).then_some(text))
        .collect();
    let mut generated_imports: Vec<_> = generated_imports.into_iter().collect();
    let evaluation = deferred::prepare(&mut program, &generated_bindings, &allocator);
    if !generated_imports.is_empty() {
        let pragma = crate::fresh_name::fresh_name("__devupCreateElement", stylesheet.code);
        let mut renders = false;
        for (span, expression) in &mut replacements {
            if let Some(lowered) = components::lower(expression, &pragma).map_err(|message| {
                crate::located_errors(
                    stylesheet.filename,
                    stylesheet.source,
                    stylesheet.edits,
                    vec![(span.start, message)],
                )
            })? {
                *expression = lowered;
                renders = true;
            }
        }
        if renders {
            generated_imports.push(format!(
                "import {{createElement as {pragma}}} from 'react';"
            ));
        }
    }
    generated_imports.sort();
    if !generated_imports.is_empty() && evaluation.is_none() {
        let insertion = program.directives.last().map_or_else(
            || {
                program
                    .hashbang
                    .as_ref()
                    .map_or(0, |hashbang| hashbang.span.end)
            },
            |directive| directive.span.end,
        );
        replacements.push((
            Span::new(insertion, insertion),
            format!("\n{}\n", generated_imports.join("\n")),
        ));
    }
    replacements.sort_by_key(|(span, _)| span.start);
    if replacements.is_empty() {
        return Ok(Compiled::default());
    }
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
    let scaffolding = if evaluation.is_some() {
        generated_imports
    } else {
        Vec::new()
    };
    Ok(Compiled {
        code: Some(code),
        edits,
        styles: visitor.styles,
        evaluation,
        scaffolding,
    })
}

fn expressions(program: &Program<'_>) -> FxHashMap<u32, (Span, String)> {
    let mut expressions = Expressions(FxHashMap::default());
    expressions.visit_program(program);
    expressions.0
}

fn imports(program: &Program<'_>) -> FxHashMap<String, String> {
    let mut imports = FxHashMap::default();
    for statement in &program.body {
        if let Statement::ImportDeclaration(import) = statement {
            let mut codegen = Codegen::new();
            import.print(&mut codegen, oxc_codegen::Context::default());
            let text = codegen.into_source_text();
            for specifier in import.specifiers.iter().flatten() {
                imports.insert(specifier.local().name.to_string(), text.clone());
            }
        }
    }
    imports
}

pub(crate) fn emit_scaffolding<'a>(
    program: &mut Program<'a>,
    imports: &[String],
    allocator: &'a Allocator,
) {
    for import in imports.iter().rev() {
        let source = allocator.alloc_str(import);
        let parsed = Parser::new(allocator, source, SourceType::mjs()).parse();
        for statement in parsed.program.body.into_iter().rev() {
            program.body.insert(0, statement);
        }
    }
}

#[cfg(test)]
mod coverage_tests;

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
    fn visit_export_default_declaration(&mut self, declaration: &ExportDefaultDeclaration<'a>) {
        if let Some(expression) = declaration.declaration.as_expression() {
            self.record(declaration.span.start, expression);
        }
        walk::walk_export_default_declaration(self, declaration);
    }

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
