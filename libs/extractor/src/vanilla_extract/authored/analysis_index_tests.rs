use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::{Semantic, SemanticBuilder};
use oxc_span::{SourceType, Span};
use oxc_syntax::symbol::SymbolId;
use rstest::rstest;

use super::Index;

fn with_index(
    source: &str,
    check: impl FnOnce(&Index<'_, '_>) -> Result<(), String>,
) -> Result<(), String> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{source}");
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0, "{source}");
    let index = Index::new(&parsed.program, &built.semantic, source);
    assert_eq!(index.code, source);
    assert!(std::ptr::eq(
        std::ptr::from_ref(index.semantic),
        std::ptr::from_ref(&built.semantic)
    ));
    check(&index)
}

fn root(semantic: &Semantic<'_>, name: &str) -> Result<SymbolId, String> {
    let scoping = semantic.scoping();
    let symbol = scoping
        .get_root_binding(name.into())
        .ok_or("root binding")?;
    assert_eq!(scoping.symbol_scope_id(symbol), scoping.root_scope_id());
    Ok(symbol)
}

fn span(source: &str, fragment: &str) -> Result<Span, String> {
    assert_eq!(source.match_indices(fragment).count(), 1, "{fragment}");
    let start = source.find(fragment).ok_or("unique fixture fragment")?;
    let end = start.checked_add(fragment.len()).ok_or("span end fits")?;
    Ok(Span::new(
        u32::try_from(start).map_err(|_| "span start fits u32")?,
        u32::try_from(end).map_err(|_| "span end fits u32")?,
    ))
}

#[rstest]
#[case::function(
    "// lead\nexport default function Named(){return 7;}",
    "export default function Named(){return 7;}"
)]
#[case::class(
    "// lead\nexport default class Named{value=7;}",
    "export default class Named{value=7;}"
)]
fn default_owns_declaration_when_named(
    #[case] source: &str,
    #[case] declaration: &str,
) -> Result<(), String> {
    // Given
    let expected = span(source, declaration)?;
    // When
    with_index(source, |index| {
        // Then
        let named = root(index.semantic, "Named")?;
        assert_eq!(index.units.len(), 1);
        let units = &index.units[&named];
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].span, expected);
        assert_eq!(units[0].text, declaration);
        assert_eq!(units[0].symbols, [named]);
        assert_eq!(index.exports.len(), 1);
        let export = &index.exports["default"];
        assert_eq!(
            (export.target, export.span, export.text.as_str()),
            (Some(named), expected, "")
        );
        assert_eq!(index.stars.len(), 0);
        Ok(())
    })
}

#[test]
fn local_alias_excludes_types_when_exports_are_mixed() -> Result<(), String> {
    // Given
    let source = "type Shape=string; const value=7; export { type Shape, value as publicValue }; export type { Shape as TypeOnly };";
    let expected = span(source, "value as publicValue")?;
    let declarator = span(source, "value=7")?;
    // When
    with_index(source, |index| {
        // Then
        let value = root(index.semantic, "value")?;
        let units = &index.units[&value];
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].span, declarator);
        assert_eq!(units[0].text, "const value=7;");
        assert_eq!(units[0].symbols, [value]);
        assert_eq!(index.exports.len(), 1);
        let export = &index.exports["publicValue"];
        assert_eq!(export.target, Some(value));
        assert_eq!(export.span, expected);
        assert_eq!(export.text, "export { value as publicValue };");
        assert_eq!(index.stars.len(), 0);
        Ok(())
    })
}

#[test]
fn forwarding_excludes_types_when_exports_have_a_source() -> Result<(), String> {
    // Given
    let source = "export { type Shape, value as forwarded } from './dep'; export type { Hidden } from './types';";
    let expected = span(source, "value as forwarded")?;
    // When
    with_index(source, |index| {
        // Then
        assert_eq!(index.units.len(), 0);
        assert_eq!(index.exports.len(), 1);
        let export = &index.exports["forwarded"];
        assert_eq!(export.target, None);
        assert_eq!(export.span, expected);
        assert_eq!(export.text, "export { value as forwarded } from './dep';");
        assert_eq!(index.stars.len(), 0);
        Ok(())
    })
}

#[test]
fn stars_preserve_order_when_namespace_and_type_exports_are_present() -> Result<(), String> {
    // Given
    let source = "export * as namespaceValue from './ns'; export * from './first'; export * from './second'; export type * from './types';";
    let namespace = "export * as namespaceValue from './ns';";
    let first = "export * from './first';";
    let second = "export * from './second';";
    let expected = span(source, namespace)?;
    let stars = [span(source, first)?, span(source, second)?];
    // When
    with_index(source, |index| {
        // Then
        assert_eq!(index.units.len(), 0);
        assert_eq!(index.exports.len(), 1);
        let export = &index.exports["namespaceValue"];
        assert_eq!(export.target, None);
        assert_eq!(export.span, expected);
        assert_eq!(export.text, namespace);
        assert_eq!(index.stars, stars);
        assert_eq!(index.stars[0].source_text(source), first);
        assert_eq!(index.stars[1].source_text(source), second);
        Ok(())
    })
}

#[test]
fn runtime_owns_units_when_ambient_bindings_are_present() -> Result<(), String> {
    // Given
    let source =
        "declare const ambient: number; export declare let external: string; export const live=3;";
    let expected = span(source, "live=3")?;
    // When
    with_index(source, |index| {
        // Then
        let ambient = root(index.semantic, "ambient")?;
        let external = root(index.semantic, "external")?;
        let live = root(index.semantic, "live")?;
        assert_eq!(index.units.len(), 1);
        assert!(!index.units.contains_key(&ambient));
        assert!(!index.units.contains_key(&external));
        let units = &index.units[&live];
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].span, expected);
        assert_eq!(units[0].text, "export const live=3;");
        assert_eq!(units[0].symbols, [live]);
        assert_eq!(index.exports.len(), 1);
        let export = &index.exports["live"];
        assert_eq!(
            (export.target, export.span, export.text.as_str()),
            (Some(live), expected, "")
        );
        assert_eq!(index.stars.len(), 0);
        Ok(())
    })
}
