//! Options that change what a styling runtime produces, given to APIs Devup UI
//! keeps as no-ops: styled-components' `StyleSheetManager` and the cache of
//! `createCache()` from `@emotion/cache`, which `CacheProvider` takes. Devup UI
//! writes every rule at build time, so the options would be dropped silently;
//! they are located build errors instead, as nothing may stay at runtime.

use crate::ImportAlias;
use crate::module_loads::module_loads;
use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, CallExpression, Expression, IdentifierReference, ImportDeclaration,
    ImportDeclarationSpecifier, JSXAttributeItem, JSXElementName, JSXMemberExpressionObject,
    JSXOpeningElement, ObjectPropertyKind, Statement,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::GetSpan;
use oxc_syntax::symbol::SymbolId;
use std::collections::HashMap;

const MANAGER: &str = "StyleSheetManager";
const CACHE: &str = "@emotion/cache";
/// The aliases whose APIs read the cache `CacheProvider` gives
const CACHE_READERS: [&str; 2] = ["@emotion/react", "@emotion/styled"];

/// Where `code` gives `StyleSheetManager` of an aliased package, or
/// `createCache()` from `@emotion/cache` while Emotion is aliased, an option
/// Devup UI would drop, or options it cannot read, as errors at their offsets
pub(crate) fn runtime_option_errors(
    code: &str,
    filename: &str,
    import_aliases: &HashMap<String, ImportAlias>,
) -> Vec<(u32, String)> {
    let manager = code.contains(MANAGER);
    let cache = code.contains(CACHE)
        && CACHE_READERS
            .iter()
            .any(|alias| import_aliases.contains_key(*alias));
    if !manager && !cache {
        return Vec::new();
    }
    let allocator = Allocator::default();
    let source_type = crate::parsing::source_type(filename).unwrap_or_default();
    let program = Parser::new(&allocator, code, source_type).parse().program;
    let mut options = Options {
        scoping: SemanticBuilder::new()
            .build(&program)
            .semantic
            .into_scoping(),
        managers: HashMap::new(),
        namespaces: HashMap::new(),
        caches: Vec::new(),
        called: Vec::new(),
        errors: Vec::new(),
    };
    let mut followed = Vec::new();
    for statement in &program.body {
        if let Statement::ImportDeclaration(import) = statement
            && !import.import_kind.is_type()
        {
            if manager
                && let Some(source) = import.source.value.as_str()
                && import_aliases.contains_key(source)
            {
                options.collect_managers(import, source);
            }
            if cache && import.source.value == CACHE && options.collect_caches(import) {
                followed.push(import.span.start);
            }
        }
    }
    let packages: &[&'static str] = if cache { &[CACHE] } else { &[] };
    options.errors.extend(
        module_loads(&program, packages)
            .into_iter()
            .filter(|(offset, _)| !followed.contains(offset))
            .map(|(offset, _)| {
                (
                    offset,
                    unknown_cache_options(
                        "`@emotion/cache` is loaded other than by an import of its default export",
                    ),
                )
            }),
    );
    options.visit_program(&program);
    options.errors
}

struct Options {
    scoping: Scoping,
    /// Bindings of `StyleSheetManager`, with the package they import it from
    managers: HashMap<SymbolId, String>,
    /// Namespace bindings of aliased packages, with the package
    namespaces: HashMap<SymbolId, String>,
    /// Bindings of `createCache` from `@emotion/cache`
    caches: Vec<SymbolId>,
    /// Where `createCache` is the callee of a call
    called: Vec<u32>,
    errors: Vec<(u32, String)>,
}

impl Options {
    fn symbol(&self, reference: &IdentifierReference<'_>) -> Option<SymbolId> {
        let reference = self.scoping.get_reference(reference.reference_id.get()?);
        reference
            .is_value()
            .then(|| reference.symbol_id())
            .flatten()
    }

    fn collect_managers(&mut self, import: &ImportDeclaration<'_>, source: &str) {
        let source = source.to_string();
        for specifier in import.specifiers.iter().flatten() {
            let symbol = specifier.local().symbol_id.get();
            match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(spec)
                    if !spec.import_kind.is_type() && spec.imported.name() == MANAGER =>
                {
                    self.managers.extend(symbol.map(|s| (s, source.clone())));
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => {
                    self.namespaces.extend(symbol.map(|s| (s, source.clone())));
                }
                _ => {}
            }
        }
    }

    /// Bind the default exports `import` takes, and whether it takes nothing
    /// else, so that every use of what it loads can be followed
    fn collect_caches(&mut self, import: &ImportDeclaration<'_>) -> bool {
        let mut followed = true;
        for specifier in import.specifiers.iter().flatten() {
            match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(spec) if spec.import_kind.is_type() => {
                }
                ImportDeclarationSpecifier::ImportSpecifier(spec)
                    if spec.imported.name() != "default" =>
                {
                    followed = false;
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => followed = false,
                specifier => self.caches.extend(specifier.local().symbol_id.get()),
            }
        }
        followed
    }

    fn is_cache(&self, reference: &IdentifierReference<'_>) -> bool {
        self.symbol(reference)
            .is_some_and(|symbol| self.caches.contains(&symbol))
    }

    /// The package `name` is `StyleSheetManager` of, by its binding or as a
    /// member of a namespace
    fn manager_source(&self, name: &JSXElementName<'_>) -> Option<&String> {
        match name {
            JSXElementName::IdentifierReference(name) => self.managers.get(&self.symbol(name)?),
            JSXElementName::MemberExpression(member) if member.property.name == MANAGER => {
                match &member.object {
                    JSXMemberExpressionObject::IdentifierReference(object) => {
                        self.namespaces.get(&self.symbol(object)?)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

impl<'a> Visit<'a> for Options {
    fn visit_jsx_opening_element(&mut self, element: &JSXOpeningElement<'a>) {
        if let Some(source) = self.manager_source(&element.name) {
            let errors = manager_errors(&element.attributes, source);
            self.errors.extend(errors);
        }
        walk::walk_jsx_opening_element(self, element);
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if let Expression::Identifier(callee) = &call.callee
            && self.is_cache(callee)
        {
            self.called.push(callee.span.start);
            self.errors.extend(cache_call_errors(call));
        }
        walk::walk_call_expression(self, call);
    }

    fn visit_identifier_reference(&mut self, reference: &IdentifierReference<'a>) {
        if self.is_cache(reference) && !self.called.contains(&reference.span.start) {
            self.errors.push((
                reference.span.start,
                unknown_cache_options(
                    "`createCache` from `@emotion/cache` is used other than by calling it",
                ),
            ));
        }
    }
}

/// Errors at the props of a `StyleSheetManager` from `source` Devup UI would
/// drop, and at its spread props, which may hold them
fn manager_errors(attributes: &[JSXAttributeItem<'_>], source: &str) -> Vec<(u32, String)> {
    attributes
        .iter()
        .filter_map(|attribute| match attribute {
            JSXAttributeItem::Attribute(attribute) => attribute
                .name
                .as_identifier()
                .and_then(|name| manager_option_error(name.name.as_str(), source))
                .map(|error| (attribute.span.start, error)),
            JSXAttributeItem::SpreadAttribute(spread) => Some((
                spread.span.start,
                format!(
                    "`{MANAGER}` from `{source}` takes spread props, so whether they set `stylisPlugins`, `shouldForwardProp` or `namespace`, which Devup UI does not compile, cannot be known at build time. Fix: write its props out by name, or set `importAliases: {{ '{source}': false }}` and build with styled-components"
                ),
            )),
        })
        .collect()
}

fn manager_option_error(name: &str, source: &str) -> Option<String> {
    let (effect, lost, fix) = match name {
        "stylisPlugins" => (
            "runs stylis plugins over the CSS at runtime",
            "the plugins would never reach the stylesheet",
            "write the CSS the plugins would produce into the styles",
        ),
        "shouldForwardProp" => (
            "filters at runtime the props every styled component passes on",
            "no prop would be filtered by it",
            "pass `shouldForwardProp` to `.withConfig()` of each styled component",
        ),
        "namespace" => (
            "prefixes every selector at runtime",
            "no selector would get the prefix",
            "write the scope into the selectors, such as `#app &`",
        ),
        _ => return None,
    };
    Some(format!(
        "`{name}` of `{MANAGER}` from `{source}` {effect}, which Devup UI does not compile, so {lost}. Fix: {fix}, or set `importAliases: {{ '{source}': false }}` and build with styled-components"
    ))
}

/// Errors at the options of a call of `createCache` that set `stylisPlugins`,
/// or that cannot be read
fn cache_call_errors(call: &CallExpression<'_>) -> Vec<(u32, String)> {
    match call.arguments.as_slice() {
        [] => Vec::new(),
        [Argument::ObjectExpression(options), ..] => options
            .properties
            .iter()
            .filter_map(|property| match property {
                ObjectPropertyKind::ObjectProperty(property) => match property.key.static_name() {
                    Some(key) if key == "stylisPlugins" => Some((
                        property.span.start,
                        format!(
                            "`stylisPlugins` of `createCache()` from `{CACHE}` runs stylis plugins over the CSS at runtime, which Devup UI does not compile, so the plugins would never reach the stylesheet. Fix: write the CSS the plugins would produce into the styles, or set `importAliases: {{ '@emotion/react': false, '@emotion/styled': false }}` and build with Emotion"
                        ),
                    )),
                    Some(_) => None,
                    None => Some((
                        property.span.start,
                        unknown_cache_options(
                            "`createCache()` from `@emotion/cache` takes an option of a computed key",
                        ),
                    )),
                },
                ObjectPropertyKind::SpreadProperty(spread) => Some((
                    spread.span.start,
                    unknown_cache_options(
                        "`createCache()` from `@emotion/cache` takes spread options",
                    ),
                )),
            })
            .collect(),
        [argument, ..] => vec![(
            argument.span().start,
            unknown_cache_options(
                "`createCache()` from `@emotion/cache` takes options that are not an object literal",
            ),
        )],
    }
}

fn unknown_cache_options(what: &str) -> String {
    format!(
        "{what}, so whether the options of `createCache()` set `stylisPlugins`, which Devup UI does not compile, cannot be known at build time. Fix: `import createCache from '{CACHE}'` and call it with an object literal of options, or set `importAliases: {{ '@emotion/react': false, '@emotion/styled': false }}` and build with Emotion"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;
    use rstest::rstest;

    fn aliases(names: &[&str]) -> HashMap<String, ImportAlias> {
        names
            .iter()
            .map(|name| ((*name).to_string(), ImportAlias::NamedToNamed))
            .collect()
    }

    fn errors(code: &str, names: &[&str]) -> Vec<(u32, String)> {
        let mut errors = runtime_option_errors(code, "App.tsx", &aliases(names));
        errors.sort_unstable();
        errors
    }

    #[rstest]
    #[case::manager_props(
        "manager_props",
        "import { StyleSheetManager as M, type StyleSheetManagerProps } from 'styled-components';\nimport * as sc from 'styled-components';\nexport const a = <M stylisPlugins={[rtl]} shouldForwardProp={valid} namespace=\"#app\" target={el} enableVendorPrefixes a:b=\"c\">x</M>;\nexport const b = <sc.StyleSheetManager {...props} nonce=\"n\" />;\nexport const c = <><sc.Other stylisPlugins={[]} /><a.b.StyleSheetManager stylisPlugins={[]} /><M.Inner stylisPlugins={[]} /><div stylisPlugins /></>;",
        &["styled-components"]
    )]
    #[case::cache_options(
        "cache_options",
        "import createCache, { type EmotionCache } from '@emotion/cache';\nimport { default as make } from '@emotion/cache';\nimport type Cache from '@emotion/cache';\nexport const a = createCache({ key: 'a', stylisPlugins: [rtl] });\nexport const b = make({ key: 'b', ['stylisPlugins']: [], [name]: 1, ...rest });\nexport const c = createCache(options);\nexport const d = createCache();\nexport const e = createCache({ key: 'e', prepend: true });\nexport const f = [createCache];\nexport type G = ReturnType<typeof createCache>;",
        &["@emotion/react"]
    )]
    #[case::cache_loads(
        "cache_loads",
        "import * as caches from '@emotion/cache';\nimport cache, { other } from '@emotion/cache';\nimport '@emotion/cache';\nconst a = require('@emotion/cache');\nexport { default } from '@emotion/cache';",
        &["@emotion/styled"]
    )]
    fn reports_options_dropped_at_runtime(
        #[case] name: &str,
        #[case] code: &str,
        #[case] names: &[&str],
    ) {
        assert_debug_snapshot!(name, errors(code, names));
    }

    #[rstest]
    #[case("import createCache from '@emotion/cache';\nexport const a = createCache({ stylisPlugins: [] });", &["@emotion/css", "styled-components"])]
    #[case("import { css } from 'styled-components';\nexport const a = css`color: red;`;", &["styled-components"])]
    #[case("import { StyleSheetManager } from 'styled-components';\nexport const a = <StyleSheetManager stylisPlugins={[]} />;", &[])]
    #[case("import { StyleSheetManager } from 'styled-components';\nimport { StyleSheetManager as Real } from 'other';\nexport const a = <Real stylisPlugins={[]} />;\nexport const b = (StyleSheetManager) => <StyleSheetManager stylisPlugins={[]} />;", &["styled-components"])]
    fn leaves_runtimes_it_does_not_replace(#[case] code: &str, #[case] names: &[&str]) {
        assert_eq!(errors(code, names), Vec::new());
    }
}
