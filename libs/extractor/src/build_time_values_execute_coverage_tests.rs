use boa_engine::{Context, Source};

use super::{compute, initialize_helpers, setup_failure, stop};
use crate::evaluation_sandbox::{Failure, Sandbox};

#[test]
fn execution_failure_preserves_guard_error_when_no_generated_script_can_locate_it()
-> Result<(), String> {
    // Given
    let mut context = Context::default();
    let sandbox = Sandbox::new(&mut context).map_err(|error| error.to_string())?;
    let failure = sandbox
        .run_source(&mut context, Source::from_bytes("Date.now()"))
        .err()
        .ok_or("guard succeeded")?;
    // When
    let errors = stop(failure, &[], "Date.now()")
        .err()
        .ok_or("failure was lost")?;
    // Then
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].0, 0);
    assert!(errors[0].1.contains("`Date`"));
    assert!(errors[0].1.starts_with("ReferenceError:"));
    Ok(())
}

#[test]
fn invalid_source_has_no_computed_values() {
    // Given
    let code = "import { css } from '@devup-ui/react'; const = css({ w: 2 });";
    // When
    let result = compute(
        code,
        "invalid.tsx",
        &crate::ExtractOption::default(),
        None,
        &crate::imported_constants::Unknown::default(),
    );
    // Then
    assert!(matches!(result, Ok(None)));
}

#[test]
fn mutable_sibling_declarations_leave_style_values_at_runtime() {
    // Given
    let code = "import { css } from '@devup-ui/react'; let unused, width = 2; export const a = css({ w: width + 1 });";
    // When
    let result = compute(
        code,
        "sibling.tsx",
        &crate::ExtractOption::default(),
        None,
        &crate::imported_constants::Unknown::default(),
    );
    // Then
    assert!(matches!(result, Ok(None)));
}

#[test]
fn definitions_skip_ambient_declarators_when_generating_initialized_values() -> Result<(), String> {
    use super::super::{Closure, Found, definitions, parse};

    // Given
    let code = "declare const external: number; const width = 2;";
    let allocator = oxc_allocator::Allocator::default();
    let program = parse(&allocator, "definitions.ts", code).ok_or("valid TypeScript rejected")?;
    let found = Found {
        span: oxc_span::Span::default(),
        closure: Closure {
            statements: [0, 1].into_iter().collect(),
            imports: Default::default(),
        },
        shorthand: None,
        rules_only: false,
    };
    // When
    let generated = definitions(&program, code, &[found]);
    // Then
    assert_eq!(generated.text, "const width = __try__(() => (2), 1);\n");
    Ok(())
}

#[test]
fn helpers_initialize_when_realm_is_fresh() -> Result<(), String> {
    // Given
    let mut context = Context::default();
    let sandbox = Sandbox::new(&mut context).map_err(|error| error.to_string())?;
    // When
    let result = initialize_helpers(&sandbox, &mut context);
    // Then
    assert!(result.is_ok());
    assert_eq!(
        context
            .eval(Source::from_bytes("__literal__({ width: 3 })"))
            .map_err(|error| error.to_string())?
            .as_string()
            .ok_or("literal helper did not serialize value")?
            .to_std_string_escaped(),
        "{\"width\":3}"
    );
    Ok(())
}

#[test]
fn setup_failure_preserves_native_syntax_error_when_binding_is_reserved() -> Result<(), String> {
    // Given
    let mut context = Context::default();
    let sandbox = Sandbox::new(&mut context).map_err(|error| error.to_string())?;
    context
        .eval(Source::from_bytes("const __failed__ = 1;"))
        .map_err(|error| error.to_string())?;
    let failure = initialize_helpers(&sandbox, &mut context)
        .err()
        .ok_or("duplicate lexical binding succeeded")?;
    let original = match &failure {
        Failure::Js(error) => error.to_string(),
        Failure::Forbidden(_) => return Err("duplicate binding must be a native JS error".into()),
    };
    assert!(original.starts_with("SyntaxError:"), "{original}");
    // When
    let errors = setup_failure(failure);
    // Then
    assert_eq!(
        errors,
        vec![(
            0,
            format!("{original}; report internal evaluation setup failure")
        )]
    );
    Ok(())
}

#[test]
fn setup_failure_preserves_forbidden_read_provenance() -> Result<(), String> {
    // Given
    let mut context = Context::default();
    let sandbox = Sandbox::new(&mut context).map_err(|error| error.to_string())?;
    let failure = sandbox
        .run_source(&mut context, Source::from_bytes("Date.now()"))
        .err()
        .ok_or("forbidden clock read succeeded")?;
    let original = match &failure {
        Failure::Forbidden(violations) => violations[0].error().to_string(),
        Failure::Js(_) => return Err("clock read must preserve forbidden evidence".into()),
    };
    // When
    let errors = setup_failure(failure);
    // Then
    assert_eq!(
        errors,
        vec![(
            0,
            format!("{original}; report internal evaluation setup failure")
        )]
    );
    Ok(())
}

#[test]
fn ordinary_js_failure_leaves_computation_at_runtime() -> Result<(), String> {
    // Given
    let mut context = Context::default();
    let sandbox = Sandbox::new(&mut context).map_err(|error| error.to_string())?;
    let failure = sandbox
        .run_source(&mut context, Source::from_bytes("null.value"))
        .err()
        .ok_or("native TypeError missing")?;
    // When
    let result = stop(failure, &[], "null.value");
    // Then
    assert!(matches!(result, Ok(None)));
    Ok(())
}
