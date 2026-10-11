use super::{CopiedSource, EvaluationLookup, StrippedSource, copied_body_offset};
use boa_engine::{Context, JsValue, NativeFunction, Source, js_string};
use css::style_origin::{RealLocation, StyleOrigin};
use rstest::rstest;
use std::{cell::RefCell, path::Path};

thread_local! {
    static LOOKUP: RefCell<Option<EvaluationLookup>> = const { RefCell::new(None) };
}

fn observed_value(context: &Context) -> JsValue {
    let location = LOOKUP.with_borrow(|lookup| {
        lookup
            .as_ref()
            .unwrap_or_else(|| panic!("lookup fixture"))
            .produced_by(context)
    });
    JsValue::from(js_string!(
        serde_json::to_string(&location).unwrap_or_else(|error| panic!("location JSON: {error}"))
    ))
}

struct LookupScope;
impl Drop for LookupScope {
    fn drop(&mut self) {
        LOOKUP.with_borrow_mut(|lookup| *lookup = None);
    }
}

#[rstest]
#[case("no-path")]
#[case("other-file")]
#[case("prefix")]
#[case("no-copy")]
#[case("no-call")]
#[case("matched")]
fn producer_lookup_requires_a_complete_authored_file_chain(#[case] mode: &str) {
    // Given
    let source = "function run(){return observed()}run();";
    let origin = StyleOrigin {
        file: "authored.css.ts".into(),
        line: 3,
        column: 7,
        expression: "opaque(style)({color:'red'})".into(),
    };
    let copies = if mode == "no-copy" {
        vec![]
    } else {
        vec![CopiedSource {
            generated: 0..source.len(),
            original: 0,
        }]
    };
    let calls = if mode == "no-call" {
        vec![]
    } else {
        vec![(
            oxc_span::Span::new(
                0,
                u32::try_from(source.len()).unwrap_or_else(|error| panic!("fixture span: {error}")),
            ),
            origin.clone(),
        )]
    };
    LOOKUP.with_borrow_mut(|lookup| {
        *lookup = Some(EvaluationLookup {
            file: "run.js".into(),
            run: source.into(),
            prefix: if mode == "prefix" {
                source.len() + 1
            } else {
                0
            },
            copies,
            calls,
        });
    });
    let _scope = LookupScope;
    let mut context = Context::default();
    context
        .register_global_builtin_callable(
            js_string!("observed"),
            0,
            NativeFunction::from_fn_ptr(|_, _, context| Ok(observed_value(context))),
        )
        .unwrap_or_else(|error| panic!("native observer: {error}"));
    let input = Source::from_bytes(source);
    let input = match mode {
        "no-path" => input,
        "other-file" => input.with_path(Path::new("other.js")),
        _ => input.with_path(Path::new("run.js")),
    };
    // When
    let actual = context
        .eval(input)
        .unwrap_or_else(|error| panic!("lookup execution: {error}"))
        .as_string()
        .unwrap_or_else(|| panic!("serialized location"))
        .to_std_string_escaped();
    // Then: incomplete chains must never invent an Exact or ProducedByCall witness.
    let expected = if mode == "matched" {
        Some(RealLocation::ProducedByCall(origin))
    } else {
        None
    };
    assert_eq!(
        serde_json::from_str::<Option<RealLocation>>(&actual)
            .unwrap_or_else(|error| panic!("location result: {error}")),
        expected
    );
}

#[test]
fn stripped_call_mapping_rejects_non_call_authored_coordinates() {
    // Given
    let stripped = StrippedSource::new("factory();", "actual.css.ts");
    // When
    let actual = stripped.authored_calls(("actual.css.ts", "notACall;"), &[]);
    // Then
    assert_eq!(actual.len(), 0);
}

#[rstest]
#[case(("a", 1, 2), Some(1))]
#[case(("a", 1, 3), None)]
#[case(("😀", 1, 3), Some(4))]
#[case(("😀", 1, 2), None)]
#[case(("a\nb", 2, 2), Some(3))]
fn end_columns_map_utf16_units_to_exact_byte_offsets(
    #[case] coordinate: (&str, u32, u32),
    #[case] expected: Option<usize>,
) {
    // Given / When
    let actual = copied_body_offset(coordinate.0, Some((coordinate.1, coordinate.2)), 0);
    // Then
    assert_eq!(actual, expected);
}

#[rstest]
#[case(("a", None, 0), None)]
#[case(("a", Some((1, 1)), 0), Some(0))]
#[case(("a", Some((1, 1)), 1), None)]
#[case(("a", Some((1, 2)), 1), Some(0))]
#[case(("a", Some((1, 2)), 2), None)]
#[case(("a", Some((1, 3)), 0), None)]
#[case(("a", Some((0, 1)), 0), None)]
#[case(("a", Some((1, 0)), 0), None)]
#[case(("a", Some((2, 1)), 0), None)]
#[case(("😀", Some((1, 2)), 0), None)]
#[case(("😀", Some((1, 3)), 0), Some(4))]
#[case(("a\n😀b", Some((2, 1)), 2), Some(0))]
#[case(("a\n😀b", Some((2, 3)), 2), Some(4))]
#[case(("a\n😀b", Some((2, 4)), 2), Some(5))]
#[case(("", Some((1, 1)), 0), None)]
#[case(("a\n", Some((2, 1)), 0), None)]
fn hard_copied_body_coordinates_preserve_absence_and_prefix_boundaries(
    #[case] input: (&str, Option<(u32, u32)>, usize),
    #[case] expected: Option<usize>,
) {
    // Given / When
    let actual = copied_body_offset(input.0, input.1, input.2);
    // Then
    assert_eq!(actual, expected);
}
