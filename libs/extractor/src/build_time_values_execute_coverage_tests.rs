use boa_engine::{Context, Source};

use super::stop;
use crate::evaluation_sandbox::Sandbox;

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
