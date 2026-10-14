use super::{dynamic, expression};
use crate::{ExtractStyleProp, ExtractStyleValue};
use css::style_origin::{Origin, RealLocation, StyleOrigin};
use oxc_allocator::Allocator;
use rstest::rstest;

#[test]
fn expression_records_receive_missing_origins_without_overwriting_exact_ones() {
    // Given
    let allocator = Allocator::default();
    let expected = StyleOrigin {
        file: "authored.tsx".into(),
        line: 4,
        column: 9,
        expression: "state.color".into(),
    };
    let original = StyleOrigin {
        file: "inner.tsx".into(),
        line: 2,
        column: 3,
        expression: "makeFrames()".into(),
    };
    let keyframes = crate::extract_style::extract_keyframes::ExtractKeyframes {
        origin: Origin(Some(Box::new(original.clone())), None),
        ..Default::default()
    };
    let mut props = [ExtractStyleProp::Expression {
        expression: expression(&allocator, "state.color"),
        styles: vec![
            dynamic("color", "state.color"),
            ExtractStyleValue::Keyframes(keyframes),
        ],
    }];
    // When
    crate::style_origin::fill(&mut props, &Origin(Some(Box::new(expected.clone())), None));
    // Then
    let records = props[0].extract();
    let ExtractStyleValue::Dynamic(value) = &records[0] else {
        panic!("dynamic fixture")
    };
    assert_eq!(value.origin.0.as_deref(), Some(&expected));
    let ExtractStyleValue::Keyframes(value) = &records[1] else {
        panic!("keyframes fixture")
    };
    assert_eq!(value.origin.0.as_deref(), Some(&original));
}

#[test]
fn cyclic_composition_gets_export_locations_without_revisiting_bases() {
    // Given
    let mut collected = crate::vanilla_extract::CollectedStyles::default();
    collected.styles.insert(
        "a".into(),
        crate::vanilla_extract::StyleEntry {
            bases: std::iter::once("b".to_string()).collect(),
            ..Default::default()
        },
    );
    collected.styles.insert(
        "b".into(),
        crate::vanilla_extract::StyleEntry {
            bases: std::iter::once("a".to_string()).collect(),
            ..Default::default()
        },
    );
    collected
        .keyframes
        .insert("unexported".into(), Default::default());
    // When
    crate::style_export_locations::associate(
        &mut collected,
        &[("public".into(), "a".into())],
        "actual.css.ts",
    );
    // Then
    let exported = Some(RealLocation::ModuleExport {
        file: "actual.css.ts".into(),
        binding: Some("public".into()),
    });
    assert_eq!(collected.styles["a"].location, exported);
    assert_eq!(collected.styles["b"].location, exported);
    assert_eq!(
        collected.keyframes["unexported"].location,
        Some(RealLocation::ModuleExport {
            file: "actual.css.ts".into(),
            binding: None
        })
    );
}

#[test]
fn absent_location_preserves_source_bytes() {
    // Given
    let source = "({get value(){return '\\u0061'}}) /* authored */";
    // When
    let actual = crate::evaluation_origin::mark_location(source.into(), None);
    // Then
    assert_eq!(actual, source);
}

#[rstest]
#[case(
    "import * as api from '@devup-ui/react';const x=api['style']({color:'red'});",
    "api['style']({color:'red'})"
)]
#[case(
    "import {'style' as factory} from '@devup-ui/react';const x=factory({color:'red'});",
    "factory({color:'red'})"
)]
fn source_packets_keep_exact_authored_factory_calls(#[case] source: &str, #[case] expected: &str) {
    use oxc_ast_visit::Visit;

    struct Packets(Vec<StyleOrigin>);
    impl<'a> oxc_ast_visit::Visit<'a> for Packets {
        fn visit_call_expression(&mut self, call: &oxc_ast::ast::CallExpression<'a>) {
            if let Some(oxc_ast::ast::Argument::StringLiteral(packet)) = call.arguments.last()
                && let Ok(origin) = serde_json::from_str::<StyleOrigin>(&packet.value)
            {
                self.0.push(origin);
            }
            oxc_ast_visit::walk::walk_call_expression(self, call);
        }
    }
    // Given
    let allocator = Allocator::default();
    // When
    let instrumented = crate::evaluation_origin::instrument(
        source,
        "packet.css.ts",
        &crate::ExtractOption::default(),
    );
    let parsed =
        oxc_parser::Parser::new(&allocator, &instrumented, oxc_span::SourceType::ts()).parse();
    // Then: inspect the packet consumed by the evaluator, not prose/code formatting.
    assert_eq!(parsed.diagnostics.len(), 0);
    let mut packets = Packets(vec![]);
    packets.visit_program(&parsed.program);
    assert_eq!(packets.0.len(), 1);
    assert_eq!(packets.0[0].file, "packet.css.ts");
    assert_eq!(packets.0[0].expression, expected);
}

#[rstest]
#[case("import factory from '@devup-ui/react';factory({color:'red'});")]
#[case("import * as api from '@devup-ui/react';({style:x=>x}).style({color:'red'});")]
#[case(
    "import * as api from '@devup-ui/react';function run(api){return api['style']({color:'red'})}"
)]
fn non_factory_bindings_receive_no_origin_arguments(#[case] source: &str) {
    // Given / When
    let actual = crate::evaluation_origin::instrument(
        source,
        "nonfactory.css.ts",
        &crate::ExtractOption::default(),
    );
    // Then
    assert_eq!(actual, source);
}
