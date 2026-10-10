use css::style_origin::RealLocation;

use crate::{ExtractOption, vanilla_extract::execute_stylesheet};

#[test]
fn opaque_calls_use_authored_outermost_producers_when_boa_returns_parent_positions() {
    // Given
    let cases = [
        (
            "const identity=f=>f;export const cls=identity(style)({color:'red'});",
            "identity(style)({color:'red'})",
        ),
        (
            "const bound=style.bind(null);export const cls=bound({color:'red'});",
            "bound({color:'red'})",
        ),
        (
            "export const cls=style.call(null,{color:'red'});",
            "style.call(null,{color:'red'})",
        ),
        (
            "export const cls=style.apply(null,[{color:'red'}]);",
            "style.apply(null,[{color:'red'}])",
        ),
        (
            "export const cls=[{color:'red'}].map(style);",
            "[{color:'red'}].map(style)",
        ),
        (
            "function run(cb,f){return cb(f);}export const cls=run(function callback(f){return f({color:'red'});},style);",
            "run(function callback(f){return f({color:'red'});},style)",
        ),
        (
            "const identity=x=>x;const native=identity(style);export const cls=identity(native({color:'red'}));",
            "identity(native({color:'red'}))",
        ),
        (
            "function pair(a,b){return [a,b];}const opaque=f=>f;export const cls=pair(opaque(style)({color:'red'}),opaque(style)({color:'blue'}));",
            "pair(opaque(style)({color:'red'}),opaque(style)({color:'blue'}))",
        ),
    ];
    // When / Then
    for (body, expected) in cases {
        let source = format!("import {{style}} from '@devup-ui/react';\n{body}");
        let (collected, _) = execute_stylesheet(
            &source,
            "call-proof.css.ts",
            &ExtractOption::default(),
            None,
        )
        .unwrap_or_else(|error| panic!("{error}"));
        assert_ne!(collected.styles.len(), 0);
        for entry in collected.styles.values() {
            assert_eq!(entry.origin, None, "{body}");
            assert!(
                matches!(&entry.location, Some(RealLocation::ProducedByCall(origin))
                if origin.file == "call-proof.css.ts" && origin.expression == expected),
                "{body}: {entry:?}"
            );
        }
    }
}

#[test]
fn nested_opaque_native_entry_clears_enclosing_factory_witness_when_no_packet_exists() {
    // Given
    let source = "import {style,styleVariants} from '@devup-ui/react';const identity=f=>f;export const classes=styleVariants({one:1},()=>identity(style)({color:'red'}));";
    // When
    let (collected, _) =
        execute_stylesheet(source, "nested.css.ts", &ExtractOption::default(), None)
            .unwrap_or_else(|error| panic!("{error}"));
    // Then
    let inner = collected
        .styles
        .values()
        .find(|entry| entry.json.contains("red"))
        .unwrap_or_else(|| panic!("missing inner style"));
    assert_eq!(inner.origin, None);
    assert!(
        matches!(&inner.location, Some(RealLocation::ProducedByCall(origin)) if origin.expression.starts_with("styleVariants(")),
        "{inner:?}"
    );
    assert!(collected.styles.values().any(|entry| {
        entry
            .origin
            .as_ref()
            .is_some_and(|origin| origin.expression.starts_with("styleVariants("))
    }));
}

#[test]
fn eval_uses_the_actual_nested_export_binding_when_source_path_has_no_file_linkage() {
    // Given
    let source = "import {style} from '@devup-ui/react';const native=style;const result={nested:[eval(\"native({color:'red'})\"),eval(\"native({color:'blue'})\")],get untouched(){throw new Error('metadata invoked a getter');}};result.self=result;export {result as actualExport};";
    // When
    let (collected, _) = execute_stylesheet(source, "eval.css.ts", &ExtractOption::default(), None)
        .unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert_eq!(collected.styles.len(), 2);
    for entry in collected.styles.values() {
        assert_eq!(entry.origin, None);
        assert_eq!(
            entry.location,
            Some(RealLocation::ModuleExport {
                file: "eval.css.ts".into(),
                binding: Some("actualExport".into())
            })
        );
    }
}

#[test]
fn eval_default_export_uses_default_binding_when_generated_local_name_is_not_authored() {
    // Given
    let source = "import {style} from '@devup-ui/react';const native=style;export default eval(\"native({color:'red'})\");";
    // When
    let (collected, _) =
        execute_stylesheet(source, "default.css.ts", &ExtractOption::default(), None)
            .unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert_eq!(collected.styles.len(), 1);
    assert_eq!(
        collected
            .styles
            .values()
            .next()
            .unwrap_or_else(|| panic!("missing style"))
            .location,
        Some(RealLocation::ModuleExport {
            file: "default.css.ts".into(),
            binding: Some("default".into())
        })
    );
}
