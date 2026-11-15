use std::rc::Rc;

use rstest::rstest;

use super::{
    native_tests::*,
    state::{FaultScope, State},
};

#[test]
fn ordinary_context_construction_is_empty_and_independent_of_tls_faults() {
    // Given an existing context with a retained original exception.
    let _fixture = Fixture::new();
    STATE.with(|state| state.remember(&Value::text("existing context")));
    // When the production constructor runs as ordinary Rust, not a const TLS initializer.
    let fresh = State::<Value>::new();
    // Then a fresh context is empty without borrowing or copying another context's state.
    assert_eq!(fresh.importer(), None);
    assert_eq!(fresh.take_cause(), None);
    assert_eq!(cause(), Some(Value::text("existing context")));
}

#[rstest]
#[case(Value::Undefined)]
#[case(Value::object(&[]))]
fn first_original_cause_survives_later_diagnostics(#[case] original: Value) {
    // Given a first thrown reference or explicit undefined and two importer scopes.
    let _fixture = Fixture::new();
    STATE.with(|state| {
        let _first = state.enter("/src/first.ts");
        state.remember(&original);
    });
    // When another resolver edge records a distinct exception.
    STATE.with(|state| {
        let _later = state.enter("/src/later.ts");
        state.remember(&Value::object(&[]));
    });
    // Then first identity, presence and origin remain independent of current context.
    assert_eq!(cause(), Some(original));
    assert_eq!(importer(), Some("/src/first.ts".into()));
}

#[test]
fn retained_origin_precedes_current_importer() {
    // Given a retained first origin and an active later importer.
    let _fixture = Fixture::new();
    STATE.with(|state| {
        {
            let _first = state.enter("/src/first.ts");
            state.retain_importer();
        }
        let _current = state.enter("/src/current.ts");
        // When the production accessor selects the origin of the pending failure.
        let actual = state.importer();
        // Then an active later importer cannot replace the retained first origin.
        assert_eq!(actual, Some("/src/first.ts".into()));
    });
}

#[test]
fn clearing_cause_and_origin_preserves_current_importer_scope() {
    // Given a pending first failure and a later active importer scope.
    let _fixture = Fixture::new();
    STATE.with(|state| {
        {
            let _first = state.enter("/src/first.ts");
            state.remember(&Value::object(&[]));
        }
        let _current = state.enter("/src/current.ts");
        // When a new boundary clears cause/origin while that scope remains active.
        state.clear_cause();
        // Then only the active importer remains available.
        assert_eq!(state.importer(), Some("/src/current.ts".into()));
        assert_eq!(cause(), None);
    });
}

#[rstest]
#[case(Value::Undefined)]
#[case(Value::object(&[]))]
fn consuming_cause_preserves_its_presence_and_identity_exactly_once(#[case] original: Value) {
    // Given an original reference or explicit undefined in the pending state.
    let _fixture = Fixture::new();
    STATE.with(|state| state.remember(&original));
    // When the production attachment boundary takes ownership of that pending cause.
    let taken = STATE.with(State::take_cause);
    // Then Some(undefined) is still a cause, but there is no pending value to replay.
    assert_eq!(taken, Some(original));
    assert_eq!(cause(), None);
}

#[test]
fn nested_importer_scope_restores_outer_context_on_panic() {
    // Given an outer importer scope and a nested native unwind.
    let _fixture = Fixture::new();
    STATE.with(|state| {
        let _outer = state.enter("/outer.ts");
        // When inner resolution unwinds without completing.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _inner = state.enter("/inner.ts");
            assert_eq!(state.importer(), Some("/inner.ts".into()));
            panic!("native scoped unwind");
        }));
        // Then Drop restores the outer importer despite the panic.
        assert!(result.is_err());
        assert_eq!(state.importer(), Some("/outer.ts".into()));
    });
    assert_eq!(importer(), None);
}

#[rstest]
#[case(false)]
#[case(true)]
fn fault_scope_drop_retains_cause_only_while_export_active(#[case] exported: bool) {
    // Given prior residue and a new fault scope with an active-export decision.
    let _fixture = Fixture::new();
    ACTIVE.set(exported);
    let scope = FaultScope::new(&STATE, active);
    let original = Value::object(&[]);
    STATE.with(|state| {
        let _importer = state.enter("/row.ts");
        state.remember(&original);
    });
    // When the scope exits without invoking callbacks under a state borrow.
    drop(scope);
    // Then retention follows export admission, including the original origin.
    assert_eq!(cause(), exported.then_some(original));
    assert_eq!(importer(), exported.then(|| "/row.ts".to_owned()));
}

#[test]
fn new_fault_scope_clears_previous_cause_and_origin_even_during_export() {
    // Given stale row diagnostics retained while an export is active.
    let _fixture = Fixture::new();
    ACTIVE.set(true);
    STATE.with(|state| {
        let _importer = state.enter("/old-row.ts");
        state.remember(&Value::object(&[]));
    });
    // When a new production fault scope begins.
    let _scope = FaultScope::new(&STATE, active);
    // Then the next row starts without old cause or importer-origin contamination.
    assert_eq!(cause(), None);
    assert_eq!(importer(), None);
}

#[test]
fn new_row_scope_cannot_reuse_suppressed_previous_row_cause() {
    // Given row A's suppressed failure retained during one admitted reservation export.
    let _fixture = Fixture::new();
    ACTIVE.set(true);
    let a = Value::object(&[]);
    {
        let _row = FaultScope::new(&STATE, active);
        STATE.with(|state| {
            let _importer = state.enter("/row-a.ts");
            state.remember(&a);
        });
    }
    let b = Value::object(&[]);
    // When row B starts its actual production scope and encounters a structural failure.
    {
        let _row = FaultScope::new(&STATE, active);
        assert_eq!(cause(), None);
        STATE.with(|state| {
            let _importer = state.enter("/row-b.ts");
            state.remember(&b);
        });
    }
    // Then B, never A, owns both the pending cause and origin.
    assert_eq!(cause(), Some(b));
    assert_eq!(importer(), Some("/row-b.ts".into()));
}

#[test]
fn nested_resolver_call_restores_outer_importer_after_inner_error() {
    // Given an outer callback which catches a nested resolver failure.
    let _fixture = Fixture::new();
    let inner = callback(Err(Value::text("inner")));
    let outer: Callback = Rc::new(move |specifier, importer_value| {
        assert_eq!(*specifier, Value::text("outer"));
        assert_eq!(*importer_value, Value::text("/outer.ts"));
        assert!(RESOLVER.call(&inner, "inner", "/inner.ts").is_err());
        STATE.with(|state| assert_eq!(*state.current.borrow(), Some("/outer.ts".into())));
        Ok(Value::Undefined)
    });
    // When the shared production call algorithm performs both scopes.
    assert!(matches!(
        RESOLVER.call(&outer, "outer", "/outer.ts"),
        Ok(None)
    ));
    // Then current importer unwinds while the first inner fault origin persists.
    STATE.with(|state| assert_eq!(*state.current.borrow(), None));
    assert_eq!(importer(), Some("/inner.ts".into()));
    assert_eq!(cause(), Some(Value::text("inner")));
}

#[test]
#[serial_test::serial]
fn stable_root_context_helpers_delegate_plain_rust_state() {
    // Given the real resolver TLS with no JS values, guarded at a native engine boundary.
    let _engine = crate::TestStateGuard::new();
    super::clear_cause();
    super::STATE.with(|state| {
        let _current = state.enter("/root.ts");
        // When frozen owner-context targets read, retain and clear their importer state.
        assert_eq!(super::importer(), Some("/root.ts".into()));
        super::retain_importer();
        super::clear_cause();
        let _scope = super::Scope::new(&super::STATE, crate::resident_owner::export_active);
        // Then the same ordinary Rust state functions serve those stable entry points.
        assert_eq!(super::importer(), Some("/root.ts".into()));
    });
    assert_eq!(super::importer(), None);
}

#[test]
fn extraction_error_constructor_passes_owned_error_to_the_attachment_transport() {
    fn new_error(message: &str) -> Value {
        Value::object(&[("message", Value::text(message))])
    }
    fn attach(value: Value) -> Value {
        value.set("attached", Value::Bool(true));
        value
    }
    // Given native Error construction and an observable attachment primitive.
    let _fixture = Fixture::new();
    // When the same generic constructor wired by the extraction driver runs.
    let error = super::extraction_error("input diagnostic", new_error, attach);
    // Then attachment receives the constructed object rather than its diagnostic string.
    assert_eq!(error.get("message"), Ok(Value::text("input diagnostic")));
    assert_eq!(error.get("attached"), Ok(Value::Bool(true)));
}

#[rstest]
#[case(None)]
#[case(Some(Value::Undefined))]
#[case(Some(Value::object(&[])))]
fn cause_attachment_uses_presence_and_consumes_original_outside_state_borrow(
    #[case] original: Option<Value>,
) {
    fn attach(_: &(), error: Value, cause: Value) -> Value {
        unborrowed();
        error.set("cause", cause);
        error
    }
    // Given no cause, an explicit undefined cause, or an original object identity.
    let _fixture = Fixture::new();
    if let Some(original) = &original {
        STATE.with(|state| state.remember(original));
    }
    let error = Value::object(&[]);
    // When production selects and calls the attachment transport.
    let result = STATE.with(|state| state.attach_cause(error.clone(), &(), attach));
    // Then attachment is presence-based, preserves error identity, and releases the pending value.
    assert_eq!(result, error);
    assert_eq!(
        result.get("cause"),
        Ok(original.clone().unwrap_or(Value::Undefined))
    );
    let Value::Object(fields) = &result else {
        panic!("attachment replaced the error object")
    };
    assert_eq!(fields.borrow().contains_key("cause"), original.is_some());
    assert_eq!(cause(), None);
}

#[test]
#[serial_test::serial]
fn empty_root_cause_attachment_never_calls_the_js_transport() {
    // Given an empty real cause state and a reserved JsValue with native-safe drop.
    let _engine = crate::TestStateGuard::new();
    super::clear_cause();
    // When the unchanged owner-context entry point forwards through generic state.
    drop(super::attach_cause(wasm_bindgen::JsValue::UNDEFINED));
    // Then the pending state stays empty without invoking unavailable native JS imports.
    assert!(super::STATE.with(|state| state.cause.borrow().is_none()));
}
