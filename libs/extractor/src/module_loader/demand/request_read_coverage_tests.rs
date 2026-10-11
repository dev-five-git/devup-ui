use oxc_allocator::Allocator;
use oxc_ast::{AstKind, ast::PropertyKey};
use oxc_span::{GetSpan, Span};
use rstest::rstest;

use super::{Demand, loader_coverage_tests::support, reads, requests};
use crate::module_loader::ModuleLoader;
use crate::ordinary_ve::selection::select_resolved;
use support::{program, semantic, stylesheet};

#[test]
fn requests_ignore_imports_when_all_references_are_type_only() -> Result<(), String> {
    // Given
    let source = "import {tokens} from './tokens';type T=typeof tokens;export type {tokens};export const value=1;";
    let allocator = Allocator::default();
    let program = program(&allocator, source);
    let semantic = &semantic(&program);
    let symbol = semantic
        .scoping()
        .get_root_binding("tokens".into())
        .ok_or("missing tokens binding")?;
    let references = semantic.scoping().get_resolved_reference_ids(symbol);
    assert_ne!(references.len(), 0);
    assert!(
        references
            .iter()
            .all(|reference| !semantic.scoping().get_reference(*reference).is_value())
    );
    let selection = select_resolved(
        (&program, semantic),
        ("/requests.ts", "@vanilla-extract/css"),
        None,
    );
    // When
    let actual = requests::entry((&program, semantic), false, &selection);
    // Then
    assert_eq!(actual, Vec::<(String, Demand, Span)>::new());
    Ok(())
}

#[rstest]
#[case("(ns).palette['tone']")]
#[case("(ns as typeof ns).palette['tone']")]
#[case("(ns satisfies typeof ns).palette['tone']")]
#[case("ns!.palette['tone']")]
#[case("(ns<string>).palette['tone']")]
fn requests_remain_precise_when_the_imported_object_has_a_syntax_wrapper(
    #[case] expression: &str,
) -> Result<(), String> {
    // Given
    let source = format!("import * as ns from './tokens';const value={expression};");
    let allocator = Allocator::default();
    let program = program(&allocator, &source);
    let semantic = semantic(&program);
    let selection = select_resolved(
        (&program, &semantic),
        ("/requests.ts", "@vanilla-extract/css"),
        None,
    );
    let start = source
        .find("'./tokens'")
        .ok_or("missing authored source literal")?;
    let end = start
        .checked_add("'./tokens'".len())
        .ok_or("import span overflow")?;
    let site = Span::new(
        u32::try_from(start).map_err(|error| error.to_string())?,
        u32::try_from(end).map_err(|error| error.to_string())?,
    );
    let expected = Demand::prefixed("palette", &Demand::prefixed("tone", &Demand::whole()));
    // When
    let actual = requests::entry((&program, &semantic), false, &selection);
    // Then
    assert_eq!(actual.len(), 1);
    assert_eq!(actual, vec![("./tokens".into(), expected, site)]);
    Ok(())
}

#[test]
fn producer_shorthand_keeps_its_key_when_the_value_becomes_a_live_read() -> Result<(), String> {
    // Given
    let source = "import {style} from '@vanilla-extract/css';import {token,read} from './producer';export const box=style({margin:token,color:read().color});";
    let authored = "import {createVar} from '@vanilla-extract/css';export const token=createVar();const color='blue';export function read(){return {color};}";
    let resolver = move |specifier: &str, importer: &str| {
        ((specifier, importer) == ("./producer", "/read-consumer.ts")).then(|| {
            crate::ResolvedModule {
                path: "/read-producer.ts".into(),
                code: authored.into(),
            }
        })
    };
    let option = support::option();
    let mut loader = ModuleLoader::new(Some(&resolver), &option);
    loader.select_demands(stylesheet("/read-consumer.ts", source), true)?;
    let producer = loader
        .producers()
        .iter()
        .find(|producer| producer.filename == "/read-producer.ts")
        .ok_or("missing actual native producer")?;
    assert_eq!(producer.source, authored);
    let allocator = Allocator::default();
    let producer_program = program(&allocator, &producer.source);
    let producer_semantic = semantic(&producer_program);
    let start = authored
        .rfind("color}")
        .ok_or("missing authored shorthand")?;
    let end = start
        .checked_add("color".len())
        .ok_or("shorthand span overflow")?;
    let site = Span::new(
        u32::try_from(start).map_err(|error| error.to_string())?,
        u32::try_from(end).map_err(|error| error.to_string())?,
    );
    assert!(producer_semantic.nodes().iter().any(|node| matches!(node.kind(), AstKind::ObjectProperty(property) if property.shorthand && property.value.span() == site)));
    // When
    let mut replacements = reads::rewrite(&producer_semantic, producer);
    // Then
    assert_eq!(
        replacements
            .iter()
            .filter(|(start, end, _)| (*start, *end) == (site.start, site.end))
            .cloned()
            .collect::<Vec<_>>(),
        vec![(
            site.start,
            site.end,
            format!("color: {}$read(color)", producer.namespace)
        )]
    );
    replacements.sort_by_key(|(start, ..)| *start);
    let mut rewritten = producer.source.clone();
    for (start, end, replacement) in replacements.into_iter().rev() {
        rewritten.replace_range(
            usize::try_from(start).map_err(|error| error.to_string())?
                ..usize::try_from(end).map_err(|error| error.to_string())?,
            &replacement,
        );
    }
    let rewritten_arena = Allocator::default();
    let rewritten_program = program(&rewritten_arena, &rewritten);
    let rewritten_semantic = semantic(&rewritten_program);
    let properties: Vec<_> = rewritten_semantic.nodes().iter().filter_map(|node| {
        match node.kind() {
            AstKind::ObjectProperty(property) if matches!(&property.key, PropertyKey::StaticIdentifier(key) if key.name == "color") => Some(property.shorthand),
            _ => None,
        }
    }).collect();
    assert_eq!(properties, [false]);
    Ok(())
}
