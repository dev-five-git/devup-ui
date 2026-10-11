use rstest::rstest;
use serial_test::serial;

use super::consumer_support::located_failure;
use super::demand_support::{TestResult, reset, run, static_value};
use super::mixed_review::emitted_predicate;

#[rstest]
#[case("(()=>1)", "input.extra=2", "input.extra=2")]
#[case("({get value(){return 1}})", "input.extra=2", "input.extra=2")]
#[case("new Proxy({ value:1 },{})", "input.extra=2", "input.extra=2")]
#[serial]
fn coverage_capture_rejects_mutation_when_retained_input_is_unsupported(
    #[case] value: &str,
    #[case] mutation: &str,
    #[case] site: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';\nconst input={value};\nexport const result=(()=>{{{mutation};style({{padding:7}});return 1}})();const browser=window.document;"
    );
    let place = crate::locate(
        "/retained.css.ts",
        &source,
        source.find(site).ok_or("mutation missing")?,
    );
    // When
    let result = run("/retained.css.ts", &source, &[]);
    // Then
    located_failure(
        result,
        &place,
        "retained native input `input` cannot capture its mutation exactly: callable, accessor or proxy state",
    );
    Ok(())
}

#[rstest]
#[case(
    "let input={ child:{ value:1 } }",
    "Object.defineProperty(input.child,'value',{get(){return 2}});input=1",
    "retained native input cannot be captured exactly: accessor property"
)]
#[case(
    "let input={ value:1 }",
    "input=()=>2",
    "retained binding `input` cannot capture its state exactly: function or proxy"
)]
#[case(
    "let input={ value:1 }",
    "input=new Proxy({ value:2 },{})",
    "retained binding `input` cannot capture its state exactly: function or proxy"
)]
#[case(
    "let input={ value:1 }",
    "Object.defineProperty(input,'value',{get(){return 2}})",
    "retained binding `input` cannot capture its state exactly: accessor property"
)]
#[serial]
fn coverage_capture_rejects_state_when_selected_initializer_makes_it_inexact(
    #[case] declaration: &str,
    #[case] mutation: &str,
    #[case] cause: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';\n{declaration};\nexport const result=(()=>{{{mutation};style({{padding:7}});return 1}})();const browser=window.document;"
    );
    let place = crate::locate(
        "/state.css.ts",
        &source,
        source.find("result=").ok_or("result missing")?,
    );
    // When
    let result = run("/state.css.ts", &source, &[]);
    // Then
    located_failure(result, &place, cause);
    Ok(())
}

#[test]
#[serial]
fn coverage_capture_preserves_identity_when_retained_paths_share_a_child() -> TestResult {
    // Given
    reset();
    let source = "import {style} from '@vanilla-extract/css';const input=(()=>{const child={ value:1 };return { a:child,b:child }})();export const result=(()=>{input.a.value=2;style({ padding:7 });return input.b})();const browser=window.document;";
    // When
    let output = run("/shared.css.ts", source, &[])?;
    // Then
    assert_eq!(static_value(&output, "padding"), "7px");
    assert!(
        emitted_predicate(
            &output.code,
            "input.a===input.b && result===input.a && result.value===2"
        ),
        "{}",
        output.code
    );
    Ok(())
}

#[test]
#[serial]
fn coverage_capture_materialization_errors_when_native_data_needs_a_poisoned_constructor()
-> TestResult {
    // Given
    reset();
    let producer = "import {style} from '@vanilla-extract/css';const token=style({ margin:1 });const storage=new Map([['value',token]]);export function make(){const data=Object.create(null);data.value=storage.get('value');Object.prototype.constructor=null;return data}const browser=window.document;";
    let source = "import {style} from '@vanilla-extract/css';import {make} from './producer';\nexport const result=(()=>{style({padding:7});return make()})();const browser=window.document;";
    let place = "/terminal.css.ts:1:1";
    // When
    let result = run(
        "/terminal.css.ts",
        source,
        &[("./producer", "/producer.ts", producer)],
    );
    // Then
    located_failure(
        result,
        place,
        "JS execution error: TypeError: TypeError: cannot convert 'null' or 'undefined' to object",
    );
    Ok(())
}

#[test]
#[serial]
fn coverage_capture_allows_preserved_calls_when_the_mutation_path_reaches_a_scalar() -> TestResult {
    // Given
    reset();
    let source = "import {style} from '@vanilla-extract/css';const input={ child:7 };export const result=style({ padding:input.child });export const later=()=>input.child.deep.touch();const browser=window.document;";
    // When
    let output = run("/scalar.css.ts", source, &[])?;
    // Then
    assert_eq!(static_value(&output, "padding"), "7px");
    assert!(
        emitted_predicate(
            &output.code,
            "input.child===7 && typeof later==='function' && typeof result==='string'"
        ),
        "{}",
        output.code
    );
    Ok(())
}

#[test]
#[serial]
fn coverage_capture_rejects_preserved_calls_when_the_mutation_path_reaches_a_proxy() -> TestResult {
    // Given
    reset();
    let source = "import {style} from '@vanilla-extract/css';const input=new Proxy({ value:7 },{});export const result=style({ padding:input.value });export const later=()=>input.value.touch();const browser=window.document;";
    let place = crate::locate(
        "/proxy.css.ts",
        source,
        source.rfind("input.value").ok_or("call missing")?,
    );
    // When
    let result = run("/proxy.css.ts", source, &[]);
    // Then
    located_failure(
        result,
        &place,
        "required native input `input` may be changed outside its selected initialization slice",
    );
    Ok(())
}
