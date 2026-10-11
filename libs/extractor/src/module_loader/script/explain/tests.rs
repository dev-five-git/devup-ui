use super::*;

/// The code as written has lines 2 to 5; the rest of the script is generated
fn locate(line: u32, column: u32) -> Option<String> {
    (2..=5)
        .contains(&line)
        .then(|| format!("/a.ts:{}:{column}", line - 1))
}

fn tell(error: &str) -> String {
    describe(error, locate, "/fallback.ts")
}

/// `told` split into what leads it, the fix and the call stack
fn parts(told: &str) -> (&str, &str, &str) {
    let (lead, rest) = told.split_once(". Fix: ").unwrap_or((told, ""));
    let (fix, calls) = rest.split_once('\n').unwrap_or((rest, ""));
    (lead, fix, calls)
}

#[test]
fn the_innermost_place_in_written_code_leads() {
    let error = "Error: boom (devup-ui-stylesheet:3:7)\n    at get (devup-ui-stylesheet:9:3)\n    at fail (devup-ui-stylesheet:3:7)\n    at <main> (devup-ui-stylesheet:4:2)";
    let told = tell(error);
    let (lead, fix, calls) = parts(&told);
    assert_eq!(lead, "/a.ts:2:7: JS execution error: Error: boom");
    assert!(fix.starts_with("remove or guard the code"), "{fix}");
    assert_eq!(calls, "    at fail (/a.ts:2:7)\n    at <main> (/a.ts:3:2)");
}

#[test]
fn calls_into_generated_code_are_dropped() {
    let error = "Error: boom (devup-ui-stylesheet:9:3)\n    at get (devup-ui-stylesheet:9:3)\n    at <main> (devup-ui-stylesheet:2:5)";
    let told = tell(error);
    let (lead, _, calls) = parts(&told);
    assert_eq!(lead, "/a.ts:1:5: JS execution error: Error: boom");
    assert_eq!(calls, "    at <main> (/a.ts:1:5)");
}

#[test]
fn what_is_native_is_kept_and_what_has_no_place_is_dropped() {
    let error = "Error: boom (native)\n    at map (native)\n    at e (eval at :1:2)\n    at j (json at :1:2)\n    at u (unknown at :4:5)\n    at p (devup-ui-stylesheet:?:?)\n    at <main> (devup-ui-stylesheet:3:1)";
    let told = tell(error);
    let (lead, _, calls) = parts(&told);
    assert_eq!(lead, "/a.ts:2:1: JS execution error: Error: boom");
    assert_eq!(calls, "    at map (native)\n    at <main> (/a.ts:2:1)");
}

#[test]
fn a_syntax_error_is_told_where_the_engine_says() {
    let told = tell("SyntaxError: unexpected token '}', primary expression at line 4, col 2");
    let (lead, fix, calls) = parts(&told);
    assert_eq!(
        lead,
        "/a.ts:3:2: JS execution error: SyntaxError: unexpected token '}', primary expression"
    );
    assert_eq!(fix, "correct the syntax at this location");
    assert_eq!(calls, "");
}

#[test]
fn the_early_errors_of_a_script_have_no_place() {
    let told = tell("SyntaxError: lexical name declared multiple times at line 1, col 1 (native)");
    let (lead, _, calls) = parts(&told);
    assert_eq!(
        lead,
        "/fallback.ts: JS execution error: SyntaxError: lexical name declared multiple times"
    );
    assert_eq!(calls, "");
}

#[test]
fn an_error_without_calls_is_told_where_it_was_raised() {
    let told = tell("Error: x (devup-ui-stylesheet:2:3)");
    assert_eq!(parts(&told).0, "/a.ts:1:3: JS execution error: Error: x");
}

#[test]
fn an_error_in_generated_code_or_without_a_place_leads_with_the_file() {
    for error in [
        "Error: x (devup-ui-stylesheet:9:3)",
        "Error: x (unknown at :9:3)",
        "Error: x (native)",
        "x",
    ] {
        let told = tell(error);
        let (lead, _, calls) = parts(&told);
        assert!(
            lead.starts_with("/fallback.ts: JS execution error: "),
            "{lead}"
        );
        assert_eq!(calls, "");
    }
}

#[test]
fn what_the_message_holds_stays_in_it() {
    let told = tell("Error: a\n    at b (c)\n    at <main> (devup-ui-stylesheet:3:1)");
    let (lead, _, calls) = parts(&told);
    assert_eq!(
        lead,
        "/a.ts:2:1: JS execution error: Error: a\n    at b (c)"
    );
    assert_eq!(calls, "    at <main> (/a.ts:2:1)");

    let told = tell("Error: value (x)");
    assert_eq!(
        parts(&told).0,
        "/fallback.ts: JS execution error: Error: value (x)"
    );

    let told = tell("Error: done...");
    assert_eq!(
        parts(&told).0,
        "/fallback.ts: JS execution error: Error: done"
    );
}

#[test]
fn every_kind_of_error_has_a_fix() {
    assert!(
        fix(&format!(
            "ReferenceError: Cannot access 'a' of 'b' {IMPORT_CYCLE}"
        ))
        .starts_with("break the import cycle")
    );
    assert_eq!(
        fix("ReferenceError: window is not defined"),
        "declare or import `window` before this code reads it; it is not available when the stylesheet is evaluated at build time"
    );
    assert_eq!(fix("SyntaxError: x"), "correct the syntax at this location");
    assert!(fix("TypeError: x is not a function").starts_with("check that every value"));
    assert!(fix("RangeError: x").starts_with("remove or guard the code"));
    assert!(fix("ReferenceError: x is not initialized").starts_with("remove or guard the code"));
}

#[test]
fn generic_fix_is_preserved_when_user_prose_mentions_css_exports() {
    // Given
    for error in [
        "Error: Cannot read CSS export 'card' of '/styles.css'",
        "ReferenceError: user says ReferenceError: Cannot read CSS export 'card'",
    ] {
        // When
        let told = tell(error);
        // Then
        assert_eq!(
            parts(&told).0,
            format!("/fallback.ts: JS execution error: {error}")
        );
        assert_eq!(
            parts(&told).1,
            "remove or guard the code that fails here, or correct the cause above, so the stylesheet evaluates without throwing"
        );
        assert_eq!(told.matches("Fix:").count(), 1);
    }
}
