use boa_engine::{Context, Source};
use rstest::rstest;

use super::{Generated, Mapped};
use crate::evaluation_sandbox::{Failure, Sandbox};

#[rstest]
#[case("Date.now()", "Date")]
#[case("Math.random()", "Math.random")]
fn fallback_frames_trace_to_copied_source_when_execution_has_no_instrumentation(
    #[case] read: &str,
    #[case] name: &str,
) -> Result<(), String> {
    // Given
    let script = format!("function read() {{ return {read}; }} read();");
    let original = format!("// original\n{script}");
    let start = original.find(&script).ok_or("missing fixture read")?;
    let mut generated = Generated::default();
    generated.copy(&original, start, original.len());
    let mapped = Mapped::new(generated, "boundary.ts", "mapped.js");
    let mut context = Context::default();
    let sandbox = Sandbox::new(&mut context).map_err(|error| error.to_string())?;
    // When
    let result = sandbox.run_source(
        &mut context,
        Source::from_bytes(&script).with_path(std::path::Path::new("mapped.js")),
    );
    let Err(Failure::Forbidden(reads)) = result else {
        return Err("missing forbidden read".to_string());
    };
    // Then
    assert_eq!(reads[0].name(), name);
    assert_eq!(
        mapped.locate(&reads[0]),
        Some(original.find('{').ok_or("missing original function body")?),
        "{}",
        reads[0].error()
    );
    Ok(())
}
