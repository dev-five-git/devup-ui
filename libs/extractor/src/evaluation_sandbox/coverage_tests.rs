use boa_engine::{Context, JsValue, Source};
use rstest::rstest;

use super::{Failure, Sandbox, instrument};

fn evaluate(script: &str) -> Result<String, String> {
    let mut context = Context::default();
    let sandbox = Sandbox::new(&mut context).map_err(|error| error.to_string())?;
    let source = instrument(script, "boundary.js");
    sandbox
        .prepare(&mut context, &source)
        .map_err(|error| error.to_string())?;
    match sandbox.run_source(&mut context, Source::from_bytes(&source.code)) {
        Ok(value) => value
            .to_string(&mut context)
            .map(|text| text.to_std_string_escaped())
            .map_err(|error| error.to_string()),
        Err(Failure::Js(error)) => Err(error.to_string()),
        Err(Failure::Forbidden(reads)) => Err(format!(
            "forbidden: {}",
            reads
                .iter()
                .map(super::Violation::name)
                .collect::<Vec<_>>()
                .join(",")
        )),
    }
}

#[rstest]
#[case("Math.random = () => 0.25; Math.random()", "0.25")]
#[case(
    "String.prototype.normalize = function() { return this + '!'; }; 'a'.normalize()",
    "a!"
)]
#[case(
    "const o = { x: 4 }; Object.getOwnPropertyDescriptor(o, 'x').value",
    "4"
)]
#[case("Object.getOwnPropertyDescriptor({}, 'missing')", "undefined")]
#[case(
    "const o = { get x() { return 5 } }; Object.getOwnPropertyDescriptor(o, 'x').get.call(o)",
    "5"
)]
#[case("const key = 'x'; const { [key]: value } = { x: 6 }; value", "6")]
#[case("const { ['x']: value } = { x: 8 }; value", "8")]
#[case(
    "const o = { n: 2, add(v) { this.n += v; return this.n } }; o?.add?.(3)",
    "5"
)]
#[case("let n = 1; eval('n += 2'); n", "3")]
fn semantics_survive_instrumentation_when_values_are_local(
    #[case] script: &str,
    #[case] expected: &str,
) -> Result<(), String> {
    // Given / When
    let result = evaluate(script)?;
    // Then
    assert_eq!(result, expected);
    Ok(())
}

#[rstest]
#[case("({ window })", "window")]
#[case("Object.getOwnPropertyDescriptor(Math, 'random')", "Math.random")]
#[case(
    "Object.getOwnPropertyDescriptor(String.prototype, 'normalize')",
    "String.prototype.normalize"
)]
fn reflection_and_shorthand_fail_when_they_reach_real_guards(
    #[case] script: &str,
    #[case] name: &str,
) -> Result<(), String> {
    // Given / When
    let error = evaluate(script)
        .err()
        .ok_or("guard did not reject the read")?;
    // Then
    assert_eq!(error, format!("forbidden: {name}"));
    Ok(())
}

#[rstest]
#[case("Date.now()", true)]
#[case("try { Date.now() } catch {} 4", false)]
fn uninstrumented_reads_keep_evidence_when_errors_are_thrown_or_caught(
    #[case] script: &str,
    #[case] thrown: bool,
) -> Result<(), String> {
    // Given
    let mut context = Context::default();
    let sandbox = Sandbox::new(&mut context).map_err(|error| error.to_string())?;
    // When
    let result = sandbox.run_source(&mut context, Source::from_bytes(script));
    let Err(Failure::Forbidden(reads)) = result else {
        return Err("missing forbidden read".to_string());
    };
    // Then
    assert_eq!(reads.len(), 1);
    assert_eq!(reads[0].name(), "Date");
    assert_eq!(reads[0].site(), None);
    assert!(reads[0].error().as_opaque().is_some());
    if thrown {
        assert!(reads[0].error().to_string().contains("<main>"));
    }
    Ok(())
}

#[test]
fn prepare_helper_returns_identity_when_an_index_has_no_read_site() -> Result<(), String> {
    // Given
    let mut context = Context::default();
    let sandbox = Sandbox::new(&mut context).map_err(|error| error.to_string())?;
    let source = instrument("1", "boundary.js");
    sandbox
        .prepare(&mut context, &source)
        .map_err(|error| error.to_string())?;
    // When
    let value = context
        .eval(Source::from_bytes(&format!("{}(999, 7)", source.helper)))
        .map_err(|error| error.to_string())?;
    // Then
    assert!(value.strict_equals(&JsValue::from(7)));
    assert!(sandbox.check(&[]).is_ok());
    Ok(())
}

#[test]
fn ordinary_javascript_failures_remain_errors_instead_of_becoming_forbidden_reads()
-> Result<(), String> {
    // Given / When
    let error = evaluate("throw new TypeError('local failure')")
        .err()
        .ok_or("throw succeeded")?;
    // Then
    assert!(error.starts_with("TypeError: local failure"));
    Ok(())
}

#[rstest]
#[case("TypeError: bad\n    at eval (boundary.js:unknown)")]
#[case("TypeError: bad\n    at eval (boundary.js:x:y)")]
fn frame_rebasing_preserves_foreign_coordinates_when_eval_has_no_numeric_site(#[case] error: &str) {
    // Given
    let source = instrument("eval('throw new Error()')", "boundary.js");
    // When / Then
    assert_eq!(source.explain(error), error);
}
