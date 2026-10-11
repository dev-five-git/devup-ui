use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::{Semantic, SemanticBuilder};
use oxc_span::{GetSpan, SourceType, Span};
use oxc_syntax::node::NodeId;
use rstest::rstest;

use super::{eager, extend};

fn with_semantic(
    source: &str,
    check: impl FnOnce(&Semantic<'_>) -> Result<(), String>,
) -> Result<(), String> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{source}");
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0, "{source}");
    check(&built.semantic)
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

fn reference_node(semantic: &Semantic<'_>, name: &str, site: Span) -> Result<NodeId, String> {
    let scoping = semantic.scoping();
    let symbol = scoping
        .get_root_binding(name.into())
        .ok_or("root binding")?;
    assert_eq!(scoping.symbol_scope_id(symbol), scoping.root_scope_id());
    let mut references = scoping
        .get_resolved_reference_ids(symbol)
        .iter()
        .filter_map(|id| {
            let reference = scoping.get_reference(*id);
            (reference.is_value()
                && site.contains_inclusive(semantic.nodes().kind(reference.node_id()).span()))
            .then_some(reference.node_id())
        });
    let node = references.next().ok_or("real fixture reference")?;
    assert_eq!(references.count(), 0);
    Ok(node)
}

#[rstest]
#[case::outside(
    "let state=0; const unrelated=1; state++;",
    "const unrelated=1;",
    false
)]
#[case::top_level("let state=0; state++;", "state++;", true)]
#[case::inner_boundary("let state=0; const late=()=>{state++;};", "state++;", true)]
#[case::admitted_callable("let state=0; const late=()=>{state++;};", "()=>{state++;}", true)]
#[case::deferred_callable(
    "let state=0; const late=()=>{state++;};",
    "const late=()=>{state++;};",
    false
)]
#[case::wrapped_iife(
    "let state=0; (((function(){state++;}) as (()=>void))! satisfies (()=>void))();",
    "(((function(){state++;}) as (()=>void))! satisfies (()=>void))();",
    true
)]
#[case::callback_is_not_callee(
    "let state=0; function consume(callback:()=>void){} consume((()=>{state++;}) satisfies (()=>void));",
    "consume((()=>{state++;}) satisfies (()=>void));",
    false
)]
fn classifies_eagerness_when_region_boundary_changes(
    #[case] source: &str,
    #[case] region: &str,
    #[case] expected: bool,
) -> Result<(), String> {
    // Given
    let region = span(source, region)?;
    let site = span(source, "state++")?;
    with_semantic(source, |semantic| {
        let node = reference_node(semantic, "state", site)?;
        // When
        let actual = eager(semantic, node, &[region]);
        // Then
        assert_eq!(actual, expected);
        Ok(())
    })
}

#[rstest]
#[case::direct_function(
    "let state=0; const late=()=>{state+=9;}; (function(){state++;})();",
    "(function(){state++;})();",
    (true, &["function(){state++;}"][..])
)]
#[case::direct_arrow(
    "let state=0; const late=()=>{state+=9;}; (()=>{state++;})();",
    "(()=>{state++;})();",
    (true, &["()=>{state++;}"][..])
)]
#[case::unsupported_initializer(
    "let state=0; const holder={run(){state++;}}; const alias=holder.run; alias();",
    "alias();",
    (false, &[][..])
)]
#[case::deferred_call(
    "let state=0; function declared(){state++;} const late=()=>{declared();};",
    "late=()=>{declared();}",
    (false, &[][..])
)]
fn extends_regions_when_callee_is_supported_and_eager(
    #[case] source: &str,
    #[case] initial: &str,
    #[case] outcome: (bool, &[&str]),
) -> Result<(), String> {
    // Given
    let initial = span(source, initial)?;
    let expected = std::iter::once(Ok(initial))
        .chain(outcome.1.iter().map(|fragment| span(source, fragment)))
        .collect::<Result<Vec<_>, String>>()?;
    with_semantic(source, |semantic| {
        if source.contains("const alias=") {
            reference_node(semantic, "alias", initial)?;
        }
        let mut regions = vec![initial];
        // When
        let changed = extend(semantic, &mut regions);
        // Then
        assert_eq!((changed, regions), (outcome.0, expected));
        Ok(())
    })
}

#[rstest]
#[case::expand_bindings(true)]
#[case::already_admitted(false)]
fn preserves_order_and_uniqueness_when_bound_callees_repeat(
    #[case] changed: bool,
) -> Result<(), String> {
    // Given
    let source = "let state=0; function declared(){state+=1;} const arrow=()=>{state+=2;}; const expression=function(){state+=3;}; const late=()=>{state+=9;}; {declared(); arrow(); expression(); declared();}";
    let seed = span(source, "{declared(); arrow(); expression(); declared();}")?;
    let expected = vec![
        seed,
        span(source, "function declared(){state+=1;}")?,
        span(source, "()=>{state+=2;}")?,
        span(source, "function(){state+=3;}")?,
    ];
    with_semantic(source, |semantic| {
        let scoping = semantic.scoping();
        for (name, count) in [
            ("declared", 2),
            ("arrow", 1),
            ("expression", 1),
            ("late", 0),
        ] {
            let symbol = scoping
                .get_root_binding(name.into())
                .ok_or("root binding")?;
            assert_eq!(scoping.symbol_scope_id(symbol), scoping.root_scope_id());
            let calls = scoping
                .get_resolved_reference_ids(symbol)
                .iter()
                .filter(|id| {
                    let reference = scoping.get_reference(**id);
                    reference.is_value()
                        && seed
                            .contains_inclusive(semantic.nodes().kind(reference.node_id()).span())
                })
                .count();
            assert_eq!(calls, count, "{name}");
        }
        let mut regions = if changed {
            vec![seed]
        } else {
            expected.clone()
        };
        // When
        let actual = extend(semantic, &mut regions);
        // Then
        assert_eq!((actual, regions), (changed, expected));
        Ok(())
    })
}
