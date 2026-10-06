use std::collections::BTreeSet;

use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SourceType, Span};
use rustc_hash::FxHashMap;

use super::{
    edits::{Replacement, apply},
    execution::{self, SelectedModule},
    lowering, rewrite, selection,
};
use crate::vanilla_extract::{
    Reference, Stylesheet, StylesheetImports, style_references::StyleReferences,
};

pub(crate) struct Prepared {
    pub code: String,
    pub edits: Vec<crate::import_alias_visit::Edit>,
    pub imports: StylesheetImports,
    pub references: StyleReferences,
    pub bindings: FxHashMap<String, String>,
    pub css: Option<String>,
}

pub(crate) fn prepare(
    stylesheet: Stylesheet<'_>,
    option: &crate::ExtractOption,
    resolver: Option<&crate::ModuleResolver>,
) -> Result<Option<Prepared>, String> {
    if !option.import_aliases.contains_key("@vanilla-extract/css") {
        return Ok(None);
    }
    let allocator = Allocator::default();
    let parsed = Parser::new(
        &allocator,
        stylesheet.code,
        SourceType::from_path(stylesheet.filename).unwrap_or_default(),
    )
    .parse();
    if !parsed.diagnostics.is_empty() {
        return Ok(None);
    }
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program)
        .semantic;
    let selection = selection::select(&parsed.program, &semantic);
    if !selection
        .imports
        .iter()
        .any(|import| import.native.is_some())
    {
        return Ok(None);
    }
    crate::module_loader::validate(stylesheet)?;
    if selection.escapes.is_empty()
        && selection.checks.is_empty()
        && !super::dispatch::mixed(&parsed.program, &semantic, &selection)
        && (super::is_module(stylesheet.filename, stylesheet.code)
            || crate::utils::is_vanilla_extract_file(stylesheet.filename))
    {
        return Ok(None);
    }
    let mut result = execution::execute(
        SelectedModule {
            stylesheet,
            selection: &selection,
        },
        option,
        resolver,
    )?;
    let mut reserved = selection.reserved_names.clone();
    reserved.extend(result.captures.iter().map(|capture| capture.name.clone()));
    reserved.extend(
        result
            .collected
            .styles
            .keys()
            .chain(result.collected.keyframes.keys())
            .cloned(),
    );
    let lowered = lowering::hygienic(
        lowering::lower(stylesheet, &result.collected, option)?,
        &mut reserved,
    )
    .map_err(|message| {
        crate::located_errors(
            stylesheet.filename,
            stylesheet.source,
            stylesheet.edits,
            vec![(0, message)],
        )
    })?;
    let mut bindings: FxHashMap<_, _> = result
        .collected
        .references
        .values()
        .filter_map(|reference| match reference {
            Reference::Style { name, class_name } => Some((name.clone(), class_name.clone())),
            Reference::Keyframes(_) => None,
        })
        .collect();
    for capture in &mut result.captures {
        if let Some(binding) = &capture.binding
            && let Some(class) = bindings.get(&capture.expression).cloned()
        {
            bindings.insert(binding.name.clone(), class);
        }
        capture.expression =
            lowering::literal(&capture.expression, &reserved).map_err(|message| {
                crate::located_errors(
                    stylesheet.filename,
                    stylesheet.source,
                    stylesheet.edits,
                    vec![(capture.root.start, message)],
                )
            })?;
    }
    let mut replacements = rewrite::replacements(
        SelectedModule {
            stylesheet,
            selection: &selection,
        },
        &parsed.program,
        &result,
    );
    replacements.extend(rewrite::imports(
        &parsed.program,
        stylesheet.code,
        &selection,
    ));
    super::emission::apply(&result.emission, &mut replacements);
    for import in selection
        .imports
        .iter()
        .filter(|import| import.native.is_some())
    {
        for reference in semantic
            .scoping()
            .get_resolved_reference_ids(import.binding.symbol)
        {
            let reference = semantic.scoping().get_reference(*reference);
            let span = semantic.nodes().kind(reference.node_id()).span();
            if reference.is_value()
                && !replacements
                    .iter()
                    .any(|edit| edit.span.contains_inclusive(span))
            {
                return Err(crate::located_errors(
                    stylesheet.filename,
                    stylesheet.source,
                    stylesheet.edits,
                    vec![(
                        span.start,
                        format!(
                            "native binding `{}` escapes preserved source. Fix: use the API only in exact initializers and retain computed values",
                            import.binding.name
                        ),
                    )],
                ));
            }
        }
    }
    for call in &selection.native_calls {
        if !selection
            .roots
            .iter()
            .any(|root| root.native_calls.contains(&call.node))
            || !replacements
                .iter()
                .any(|edit| edit.span.contains_inclusive(call.span))
        {
            return Err(crate::located_errors(
                stylesheet.filename,
                stylesheet.source,
                stylesheet.edits,
                vec![(
                    call.callee.start,
                    format!(
                        "native `{}` call cannot remain in preserved source. Fix: report this extraction error with the original module",
                        call.api
                    ),
                )],
            ));
        }
    }
    let imported: BTreeSet<_> = parsed
        .program
        .body
        .iter()
        .filter_map(|statement| match statement {
            Statement::ImportDeclaration(import) => Some(import.source.value.to_string()),
            _ => None,
        })
        .collect();
    let mut edges = String::new();
    for source in result
        .imports
        .kept_imports
        .iter()
        .filter(|source| !imported.contains(*source))
    {
        edges.push_str("import ");
        edges.push_str(&crate::vanilla_extract::json_string(source));
        edges.push_str(";\n");
    }
    let insertion = parsed
        .program
        .directives
        .last()
        .map(|directive| directive.span.end)
        .or_else(|| {
            parsed
                .program
                .hashbang
                .as_ref()
                .map(|hashbang| hashbang.span.end)
        })
        .unwrap_or(0);
    replacements.push(Replacement {
        span: Span::new(insertion, insertion),
        text: format!("\n{edges}{}\n{}\n", lowered.code, result.emission.header),
    });
    let (code, edits) = apply(stylesheet.code, replacements).map_err(|error| {
        crate::located_errors(
            stylesheet.filename,
            stylesheet.source,
            stylesheet.edits,
            vec![(error.at, error.message)],
        )
    })?;
    let mut references = result.imports.references.clone();
    references.merge(result.collected.class_references);
    Ok(Some(Prepared {
        code,
        edits,
        imports: result.imports,
        references,
        bindings,
        css: lowered.css,
    }))
}
