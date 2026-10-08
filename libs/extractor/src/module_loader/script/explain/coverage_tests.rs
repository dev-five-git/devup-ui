use super::describe;

#[test]
fn internal_guard_and_instrumentation_frames_are_removed_when_reporting_author_calls()
-> Result<(), String> {
    // Given
    let error = "ReferenceError: read denied\n    at guard (<devup-sandbox>:1:4)\n    at __devup_read_site_123__ (devup-ui-stylesheet:2:3)\n    at __devup_operation_site_456__ (devup-ui-stylesheet:2:4)\n    at user (devup-ui-stylesheet:3:7)";
    // When
    let result = describe(
        error,
        |line, column| Some(format!("original.ts:{line}:{column}")),
        "fallback.ts:1:1",
    );
    // Then
    let (head, rest) = result
        .split_once(". Fix: ")
        .ok_or("missing diagnostic delimiter")?;
    assert_eq!(
        head,
        "original.ts:3:7: JS execution error: ReferenceError: read denied"
    );
    let (_, calls) = rest.split_once('\n').ok_or("missing author call")?;
    assert_eq!(calls, "    at user (original.ts:3:7)");
    Ok(())
}
