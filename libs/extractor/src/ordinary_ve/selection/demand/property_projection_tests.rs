use oxc_allocator::Allocator;
use oxc_ast::ast::{ImportDeclarationSpecifier, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;

use super::receiver_test_support::{TestResult, assert_identity, objects, observe, path, span};
use super::{
    Demand, ImportName,
    receiver_fixtures::{D4_SOURCE, P3},
};

#[test]
fn imported_property_demand_is_forwarded_when_computed_members_have_ts_wrappers() -> TestResult {
    // Given
    let demand = path(&["tokens", "spacing", "small"]);
    let declaration = P3
        .split_once("export const ")
        .ok_or("missing export")?
        .1
        .trim_end_matches(';');
    // When
    let observed = observe("/forward.ts", P3, &demand)?;
    // Then
    assert_identity(&observed, P3, &[("tokens", declaration)])?;
    let unit = &observed.view.selection.units[0];
    assert_eq!(
        observed.view.units.get(&unit.node),
        Some(&path(&["spacing", "small"]))
    );
    assert_eq!(observed.view.properties.len(), 1);
    assert_eq!(
        observed
            .view
            .properties
            .get(&unit.node)
            .ok_or("missing pruning")?
            .omitted,
        vec![span(P3, "browser:window.document")?]
    );
    let [import] = observed.view.selection.imports.as_slice() else {
        panic!("expected one selected import")
    };
    assert_eq!(import.binding.name, "palette");
    assert_eq!(import.binding.span, span(P3, "palette")?);
    assert_eq!(import.specifier, span(P3, "palette")?);
    assert_eq!(
        import.declaration,
        span(P3, "import {palette} from './base';")?
    );
    assert_eq!(import.source, "./base");
    assert!(matches!(&import.imported, ImportName::Named(name) if name == "palette"));
    assert!(!import.erased);
    assert_eq!(import.native, None);
    assert_eq!(
        observed.view.forwarded,
        vec![(
            "./base".into(),
            path(&["palette", "sizes", "small"]),
            span(P3, "palette")?
        )]
    );
    assert_eq!(
        objects(&observed.rendered)?,
        vec![("tokens".into(), vec![Some("spacing".into())])]
    );
    assert!(
        observed
            .rendered
            .mapped
            .code
            .contains("spacing:((palette['sizes'] as {small:string;browser:unknown})!)")
    );
    assert!(
        !observed
            .rendered
            .mapped
            .code
            .contains("browser:window.document")
    );
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &observed.rendered.mapped.code, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let imports: Vec<_> = parsed
        .program
        .body
        .iter()
        .filter_map(|statement| match statement {
            Statement::ImportDeclaration(import) => Some(import),
            _ => None,
        })
        .collect();
    let [import] = imports.as_slice() else {
        panic!("expected one rendered import")
    };
    assert_eq!(import.source.value.as_str(), "./base");
    let [ImportDeclarationSpecifier::ImportSpecifier(specifier)] = import
        .specifiers
        .as_ref()
        .ok_or("missing specifiers")?
        .as_slice()
    else {
        panic!("expected named import")
    };
    assert_eq!(specifier.local.name.as_str(), "palette");
    assert_eq!(specifier.imported.name().as_str(), "palette");
    assert!(!import.import_kind.is_type());
    assert!(!specifier.import_kind.is_type());
    Ok(())
}

#[test]
fn spread_object_declines_pruning_when_a_single_export_property_is_demanded() -> TestResult {
    // Given
    let demand = path(&["tokens", "space"]);
    // When
    let observed = observe("/spread.ts", D4_SOURCE, &demand)?;
    // Then
    assert_identity(
        &observed,
        D4_SOURCE,
        &[
            ("base", "base={space:'8px'}"),
            ("tokens", "tokens={...base,browser:window.document}"),
        ],
    )?;
    let units = &observed.view.selection.units;
    assert_eq!(
        observed.view.units.get(&units[0].node),
        Some(&Demand::whole())
    );
    assert_eq!(
        observed.view.units.get(&units[1].node),
        Some(&path(&["space"]))
    );
    for unit in units {
        assert!(!observed.view.properties.contains_key(&unit.node));
    }
    assert_eq!(observed.view.properties.len(), 0);
    assert_eq!(observed.view.selection.imports.len(), 0);
    assert_eq!(observed.view.forwarded, vec![]);
    assert_eq!(
        objects(&observed.rendered)?,
        vec![
            ("base".into(), vec![Some("space".into())]),
            ("tokens".into(), vec![None, Some("browser".into())])
        ]
    );
    assert!(observed.rendered.mapped.code.contains("...base"));
    assert!(
        observed
            .rendered
            .mapped
            .code
            .contains("browser:window.document")
    );
    Ok(())
}
