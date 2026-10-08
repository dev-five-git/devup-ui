use super::demand_support::{TestResult, reset};
use super::*;

#[rstest]
#[serial]
fn changed_terminal_when_raw_forwarder_is_validated_retains_original_mutation_error(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    reset();
    let origin = format!("/b-changed.{suffix}");
    let producer = "const 한글='😀';\r\nimport * as ve from '@vanilla-extract/css';\r\nconst make=ve.style;make.extra=1;export {make};";
    let place = crate::locate(
        &origin,
        producer,
        producer.find("make.extra").ok_or("mutation missing")?,
    );
    let resolved = ResolvedModule {
        path: origin,
        code: producer.into(),
    };
    let resolver = move |specifier: &str, _: &str| {
        (specifier == "./origin").then(|| ResolvedModule {
            path: resolved.path.clone(),
            code: resolved.code.clone(),
        })
    };
    let mut option = option();
    option.single_css = single;
    // When
    let error = extract_with_modules(
        &format!("/b-changed-forward.{suffix}"),
        "export {make} from './origin';",
        option,
        true,
        &resolver,
    )
    .err()
    .ok_or("changed raw forwarder succeeded")?
    .to_string();
    // Then
    assert!(
        error.contains(&place) && error.contains("may be changed"),
        "{error}"
    );
    assert!(error.contains("Fix:"));
    assert!(!error.contains("Invalid exports"));
    Ok(())
}

#[rstest]
#[serial]
fn ambiguous_terminal_when_raw_forwarder_is_validated_is_not_generic_escape(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    reset();
    let path = format!("/b-ambiguous-forward.{suffix}");
    let source = "const 한글='😀';\r\n\r\nexport {style as make} from './api';";
    let place = crate::locate(
        &path,
        source,
        source.find("style as").ok_or("export missing")?,
    );
    let resolver = |specifier: &str, _: &str| match specifier {
        "./api" => Some(ResolvedModule {
            path: "/b-ambiguous.ts".into(),
            code: "export * from './native';export * from './ordinary';".into(),
        }),
        "./native" => Some(ResolvedModule {
            path: "/b-native.ts".into(),
            code: "export {style} from '@vanilla-extract/css';".into(),
        }),
        "./ordinary" => Some(ResolvedModule {
            path: "/b-plain.ts".into(),
            code: "export const style=value=>value;".into(),
        }),
        _ => None,
    };
    let mut option = option();
    option.single_css = single;
    // When
    let error = extract_with_modules(&path, source, option, true, &resolver)
        .err()
        .ok_or("ambiguous raw forwarder succeeded")?
        .to_string();
    // Then
    assert!(
        error.contains(&place) && error.contains("ambiguous"),
        "{error}"
    );
    assert!(error.contains("Fix:"));
    assert!(!error.contains("escapes its exact initialization slice"));
    Ok(())
}

#[rstest]
#[serial]
fn failed_namespace_terminal_when_raw_forwarder_is_validated_keeps_failure_cause(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    reset();
    let path = format!("/b-failed-forward.{suffix}");
    let source = "const 한글='😀';\r\n\r\nexport * as api from './a';";
    let place = crate::locate(&path, source, source.find("'./a'").ok_or("export missing")?);
    let resolver = |specifier: &str, _: &str| match specifier {
        "./a" => Some(ResolvedModule {
            path: "/b-recursive-a.ts".into(),
            code: "export * as nested from './b';export {style} from '@vanilla-extract/css';"
                .into(),
        }),
        "./b" => Some(ResolvedModule {
            path: "/b-recursive-b.ts".into(),
            code: "export * as back from './a';".into(),
        }),
        _ => None,
    };
    let mut option = option();
    option.single_css = single;
    // When
    let error = extract_with_modules(&path, source, option, true, &resolver)
        .err()
        .ok_or("failed namespace forwarder succeeded")?
        .to_string();
    // Then
    assert!(error.contains(&place), "{error}");
    assert!(
        error.contains("recursive native namespace needs an exact terminal member"),
        "{error}"
    );
    assert!(error.contains("Fix:"));
    assert!(!error.contains("escapes its exact initialization slice"));
    Ok(())
}
