use super::{
    Allocator, BTreeMap, Binding, Cow, Errors, GetSpan, HashMap, HashSet, ImportAlias, Normalized,
    Normalizer, Parser, Root, SemanticBuilder, SourceType, Span, identifier_names, macro_name,
};
use oxc_ast::ast::{
    Declaration, ImportDeclaration, ImportDeclarationSpecifier, Statement,
    TSImportEqualsDeclaration, TSModuleReference,
};

fn is_emotion_require(declaration: &TSImportEqualsDeclaration<'_>) -> bool {
    !declaration.import_kind.is_type()
        && matches!(
            &declaration.module_reference,
            TSModuleReference::ExternalModuleReference(reference)
                if reference.expression.value == "@emotion/css"
        )
}

impl Normalizer<'_, '_> {
    /// Bind the namespaces a module statement imports; the span of an ES
    /// namespace import goes to `namespaces`, to drop it when nothing reads it.
    fn collect_statement(&mut self, statement: &Statement<'_>, namespaces: &mut Vec<Span>) {
        match statement {
            Statement::ExportAllDeclaration(export)
                if export.source.value == "@emotion/css" && !export.export_kind.is_type() =>
            {
                self.error(
                    export.span,
                    "a namespace containing styling functions cannot be re-exported",
                );
            }
            Statement::ExportFromDeclaration(export)
                if export.source.value == "@emotion/css" && !export.export_kind.is_type() =>
            {
                for specifier in &export.specifiers {
                    if !specifier.export_kind.is_type()
                        && (macro_name(specifier.local.name().as_str()).is_some()
                            || specifier.local.name() == "default")
                    {
                        self.error(specifier.span, "styling functions cannot be re-exported");
                    }
                }
            }
            Statement::ExportDeclaration(export) if matches!(&export.declaration, Declaration::TSImportEqualsDeclaration(declaration) if is_emotion_require(declaration)) =>
            {
                self.error(
                    export.span,
                    "a namespace containing styling functions cannot be exported",
                );
            }
            Statement::TSImportEqualsDeclaration(declaration)
                if is_emotion_require(declaration) =>
            {
                if let Some(symbol) = declaration.id.symbol_id.get() {
                    self.bindings
                        .insert(symbol, Binding::Namespace(Root::Require(declaration.span)));
                }
            }
            Statement::ImportDeclaration(import) => {
                self.collect_module_api(import);
                self.collect_import(import, namespaces);
            }
            _ => {}
        }
    }

    fn collect_import(&mut self, import: &ImportDeclaration<'_>, namespaces: &mut Vec<Span>) {
        if import.source.value != "@emotion/css" || import.import_kind.is_type() {
            return;
        }
        for specifier in import.specifiers.iter().flatten() {
            let Some(symbol) = specifier.local().symbol_id.get() else {
                continue;
            };
            match specifier {
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => {
                    self.bindings
                        .insert(symbol, Binding::Namespace(Root::Module(import.span)));
                    namespaces.push(import.span);
                }
                ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => self.error(
                    specifier.span(),
                    "the root module has no default export; use named or namespace imports",
                ),
                ImportDeclarationSpecifier::ImportSpecifier(named)
                    if !named.import_kind.is_type() =>
                {
                    let name = named.imported.name();
                    if name == "default" {
                        self.error(named.span, "the root module has no default export");
                    }
                    if let Some(api) = macro_name(name.as_str()) {
                        self.bindings.insert(symbol, Binding::Macro(api));
                    }
                }
                ImportDeclarationSpecifier::ImportSpecifier(_) => {}
            }
        }
    }
}

/// Lower only the enabled root Emotion alias; offsets belong to the input source.
pub(crate) fn normalize<'a>(
    code: &'a str,
    filename: &str,
    aliases: &HashMap<String, ImportAlias>,
) -> Result<Normalized<'a>, Errors> {
    if aliases.get("@emotion/css") != Some(&ImportAlias::NamedToNamed)
        || !code.contains("@emotion/css")
    {
        return Ok((Cow::Borrowed(code), vec![]));
    }
    let allocator = Allocator::default();
    let parsed = Parser::new(
        &allocator,
        code,
        SourceType::from_path(filename).unwrap_or_default(),
    )
    .parse();
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program)
        .semantic;
    let mut normalizer = Normalizer {
        code,
        semantic: &semantic,
        bindings: HashMap::new(),
        aliases: HashSet::new(),
        runtime: HashSet::new(),
        loaders: HashSet::new(),
        module_api: HashSet::new(),
        names: identifier_names(&semantic),
        imports: BTreeMap::new(),
        replacements: vec![],
        errors: vec![],
    };
    let mut namespaces = Vec::new();
    for statement in &parsed.program.body {
        normalizer.collect_statement(statement, &mut namespaces);
    }
    normalizer.collect_bindings();
    normalizer.lower_reads();
    normalizer.reject_unawaited_imports();
    normalizer.lower_bindings();
    for span in namespaces {
        if !normalizer.runtime.contains(&Root::Module(span)) {
            normalizer.replace(span, String::new());
        }
    }
    if !normalizer.errors.is_empty() {
        return Err(normalizer.errors);
    }
    if !normalizer.imports.is_empty() {
        let imports = normalizer
            .imports
            .iter()
            .map(|(api, name)| format!("{api} as {name}"))
            .collect::<Vec<_>>()
            .join(",");
        // Insert at the first statement rather than before a possible hashbang.
        let offset = parsed
            .program
            .body
            .first()
            .map_or(0, |statement| statement.span().start);
        normalizer.replace(
            Span::new(offset, offset),
            format!("import {{{imports}}} from '@emotion/css';\n"),
        );
    }
    normalizer
        .replacements
        .sort_by_key(|(start, end, _)| (*start, *end));
    let edits = normalizer
        .replacements
        .iter()
        .map(|(start, end, value)| (*start, *end, value.len()))
        .collect();
    let mut output = code.to_string();
    for (start, end, value) in normalizer.replacements.into_iter().rev() {
        output.replace_range(start..end, &value);
    }
    Ok((Cow::Owned(output), edits))
}
