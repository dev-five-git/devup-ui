use boa_engine::{Context, Source, js_string};

use super::super::{Failure, Sandbox};

#[test]
fn css_observation_remains_fatal_when_host_serializes_after_evaluation() -> Result<(), String> {
    // Given
    let mut context = Context::default();
    let sandbox = Sandbox::new(&mut context).map_err(|error| error.to_string())?;
    let object = super::css_object(&mut context, "/src/styles.module.css")
        .map_err(|error| error.to_string())?;
    let error = object
        .get(js_string!("card"), &mut context)
        .err()
        .ok_or("no CSS value is invented")?;
    // When
    let Err(Failure::Forbidden(reads)) = sandbox.check(&[&error]) else {
        return Err("host read must be a fatal CSS observation".to_string());
    };
    // Then
    assert_eq!(reads.len(), 1);
    assert!(
        reads[0]
            .css_message()
            .is_some_and(|message| message.contains("export 'card' of '/src/styles.module.css'"))
    );
    assert_eq!(reads[0].site(), None);
    Ok(())
}

#[test]
fn css_observation_reports_context_boundary_when_proxy_is_used_without_sandbox()
-> Result<(), String> {
    // Given
    let mut context = Context::default();
    let object = super::css_object(&mut context, "/src/styles.module.css")
        .map_err(|error| error.to_string())?;
    // When
    let error = object
        .get(js_string!("card"), &mut context)
        .err()
        .ok_or("a host observer needs its sandbox")?;
    // Then
    assert!(
        error
            .to_string()
            .contains("CSS observation requires a sandbox")
    );
    Ok(())
}

#[test]
fn css_observation_is_immutable_when_caught_error_prototype_is_replaced() -> Result<(), String> {
    // Given
    let mut context = Context::default();
    let sandbox = Sandbox::new(&mut context).map_err(|error| error.to_string())?;
    let object = super::css_object(&mut context, "/src/styles.module.css")
        .map_err(|error| error.to_string())?;
    context
        .register_global_property(
            js_string!("styles"),
            object,
            boa_engine::property::Attribute::empty(),
        )
        .map_err(|error| error.to_string())?;
    let source = super::super::instrument(
        "try{styles.card}catch(error){error.message='mutated';ReferenceError.prototype.name='Changed';}try{styles.other}catch{}",
        "fixture.js",
    );
    sandbox
        .prepare(&mut context, &source)
        .map_err(|error| error.to_string())?;
    // When
    let Err(Failure::Forbidden(reads)) =
        sandbox.run_source(&mut context, Source::from_bytes(&source.code))
    else {
        return Err("caught errors cannot erase host evidence".to_string());
    };
    // Then
    assert_eq!(reads.len(), 2);
    assert!(reads[0].css_message().is_some_and(|message| {
        message.starts_with("ReferenceError: Cannot read CSS export 'card'")
    }));
    assert!(
        !reads[0]
            .css_message()
            .unwrap_or_default()
            .contains("mutated")
    );
    Ok(())
}
