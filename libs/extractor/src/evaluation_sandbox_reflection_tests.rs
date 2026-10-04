use std::path::Path;

use boa_engine::{Context, JsValue, Source};
use rstest::rstest;

use super::{Failure, Sandbox};

fn run(script: &str) -> Result<JsValue, Failure> {
    let mut context = Context::default();
    let sandbox = Sandbox::new(&mut context).map_err(Failure::Js)?;
    let source = super::instrument(script, "reflection.js");
    sandbox
        .prepare(&mut context, &source)
        .map_err(Failure::Js)?;
    sandbox.run_source(
        &mut context,
        Source::from_bytes(&source.code).with_path(Path::new("reflection.js")),
    )
}

#[rstest]
#[case("Object.getOwnPropertyDescriptor(Math, 'random')", "Math.random")]
#[case("Object.getOwnPropertyDescriptors(Math)", "Math.random")]
#[case("Reflect.getOwnPropertyDescriptor(Math, 'random')", "Math.random")]
#[case(
    "const api = Reflect;\napi['getOwnProperty' + 'Descriptor'](Math, 'random')",
    "Math.random"
)]
#[case(
    "const read = Reflect.getOwnPropertyDescriptor;\nread(Math, 'random')",
    "Math.random"
)]
#[case(
    "const { getOwnPropertyDescriptor: read } = Reflect;\nread(Math, 'random')",
    "Math.random"
)]
#[case(
    "const read = Reflect.getOwnPropertyDescriptor.bind(null, Math);\nread('random')",
    "Math.random"
)]
#[case(
    "const read = Object.getOwnPropertyDescriptors.bind(null);\nread(Math)",
    "Math.random"
)]
#[case(
    "Object.getOwnPropertyDescriptors(String.prototype)",
    "String.prototype.localeCompare"
)]
#[case(
    "Reflect.getOwnPropertyDescriptor(String.prototype, 'normalize')",
    "String.prototype.normalize"
)]
#[case("Object.getOwnPropertyDescriptors(globalThis)", "Date")]
fn guarded_descriptors_fail_at_the_reflection_call(
    #[case] script: &str,
    #[case] name: &str,
) -> Result<(), String> {
    // Given
    let offset = script.rfind('\n').map_or(0, |at| at + 1);
    // When
    let Err(Failure::Forbidden(violations)) = run(script) else {
        return Err("reflection did not record a forbidden read".to_string());
    };
    // Then
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].name(), name);
    let place = crate::locate("reflection.js", script, offset);
    assert_eq!(violations[0].site(), Some((place.as_str(), offset)));
    assert!(violations[0].error().to_string().contains(&place));
    Ok(())
}

#[rstest]
fn generated_global_getter_descriptors_are_guarded(
    #[values("Object.getOwnPropertyDescriptor", "Reflect.getOwnPropertyDescriptor")] reader: &str,
    #[values("Date", "Intl", "performance", "crypto", "Temporal")] name: &str,
) -> Result<(), String> {
    // Given
    let script = format!("{reader}(globalThis, '{name}')");
    // When
    let Err(Failure::Forbidden(violations)) = run(&script) else {
        return Err("global guard descriptor escaped".to_string());
    };
    // Then
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].name(), name);
    assert_eq!(violations[0].site().map(|(_, offset)| offset), Some(0));
    Ok(())
}

#[rstest]
#[case("Object.getOwnPropertyDescriptor(Math, 'random')")]
#[case("Object.getOwnPropertyDescriptors(Math)")]
#[case("Reflect.getOwnPropertyDescriptor(Math, 'random')")]
#[case("Object.getOwnPropertyDescriptor(globalThis, 'Date')")]
#[case("Reflect.getOwnPropertyDescriptor(globalThis, 'Date')")]
fn caught_descriptor_errors_cannot_mutate_the_host_evidence(
    #[case] read: &str,
) -> Result<(), String> {
    // Given
    let script = format!(
        "try {{ {read} }} catch (error) {{ error.message = 'changed'; error.name = 'changed'; }}\nMath.random = () => 1; 'accepted'"
    );
    // When
    let Err(Failure::Forbidden(violations)) = run(&script) else {
        return Err("caught reflection was accepted".to_string());
    };
    // Then
    assert_eq!(violations.len(), 1);
    let name = if read.contains("globalThis") {
        "Date"
    } else {
        "Math.random"
    };
    assert_eq!(violations[0].name(), name);
    assert_eq!(violations[0].site().map(|(_, offset)| offset), Some(6));
    assert_eq!(
        violations[0].requirement(),
        super::Kind::of(name).requirement()
    );
    assert!(
        violations[0]
            .error()
            .to_string()
            .contains(&format!("`{name}`"))
    );
    Ok(())
}

#[test]
fn key_coercion_cannot_replace_the_reflection_call_site() -> Result<(), String> {
    // Given
    let script = "const key = { toString() { Math.max(1, 2); return 'random' } };\nReflect.getOwnPropertyDescriptor(Math, key)";
    let offset = script
        .find("Reflect.getOwnPropertyDescriptor")
        .ok_or("missing read")?;
    // When
    let Err(Failure::Forbidden(violations)) = run(script) else {
        return Err("coerced reflection was accepted".to_string());
    };
    // Then
    assert_eq!(violations[0].site().map(|(_, at)| at), Some(offset));
    Ok(())
}

#[rstest]
#[case("String(Object.getOwnPropertyDescriptor({ x: 1 }, 'x').value)", "1")]
#[case("String(Reflect.getOwnPropertyDescriptor({ x: 1 }, 'x').value)", "1")]
#[case("String(Object.getOwnPropertyDescriptors({ x: 1 }).x.value)", "1")]
#[case(
    "`${Object.getOwnPropertyDescriptor.length}:${Object.getOwnPropertyDescriptors.length}:${Reflect.getOwnPropertyDescriptor.length}`",
    "2:1:2"
)]
#[case("String(Object.getOwnPropertyDescriptor({}, 'missing'))", "undefined")]
#[case("String(Reflect.getOwnPropertyDescriptor({}, 'missing'))", "undefined")]
#[case("String(Object.getOwnPropertyDescriptor('a', '0').value)", "a")]
#[case("String(Object.getOwnPropertyDescriptors('a')['0'].value)", "a")]
#[case(
    "const o = { get random() { return 4 } }; const d = Reflect.getOwnPropertyDescriptor(o, 'random'); String(d.get.call(o))",
    "4"
)]
#[case(
    "const o = { get x() { throw new Error('getter executed') } }; String(typeof Object.getOwnPropertyDescriptors(o).x.get)",
    "function"
)]
#[case(
    "const o = {}; Object.defineProperty(o, 'x', { value: 4, writable: false, enumerable: false, configurable: false }); JSON.stringify(Reflect.getOwnPropertyDescriptor(o, 'x'))",
    "{\"value\":4,\"writable\":false,\"enumerable\":false,\"configurable\":false}"
)]
#[case(
    "const o = { normalize() { return 5 } }; String(Object.getOwnPropertyDescriptors(o).normalize.value())",
    "5"
)]
#[case(
    "Math.random = () => 7; String(Object.getOwnPropertyDescriptors(Math).random.value())",
    "7"
)]
#[case(
    "String.prototype.normalize = () => 'own'; String(Reflect.getOwnPropertyDescriptor(String.prototype, 'normalize').value())",
    "own"
)]
#[case(
    "globalThis.Date = 9; String(Object.getOwnPropertyDescriptor(globalThis, 'Date').value)",
    "9"
)]
#[case(
    "globalThis.Date = 9; globalThis.Intl = 8; globalThis.performance = 7; globalThis.crypto = 6; globalThis.Temporal = 5; String(Object.getOwnPropertyDescriptors(globalThis).Date.value)",
    "9"
)]
#[case(
    "Object.defineProperty(globalThis, 'Intl', { get() { return 8 } }); String(Reflect.getOwnPropertyDescriptor(globalThis, 'Intl').get())",
    "8"
)]
#[case(
    "String(Reflect.getOwnPropertyDescriptor(Math, 'PI').value === Math.PI)",
    "true"
)]
#[case(
    "const key = Symbol('x'); const o = { [key]: 6 }; String(Object.getOwnPropertyDescriptors(o)[key].value)",
    "6"
)]
#[case(
    "Object.defineProperty(Object.prototype, 'get', { get() { throw new Error('inherited get') } }); String(Object.getOwnPropertyDescriptor({ x: 1 }, 'x').value)",
    "1"
)]
#[case(
    "const read = { getOwnPropertyDescriptors() { return { x: { value: 3 } } } }; String(read.getOwnPropertyDescriptors().x.value)",
    "3"
)]
#[case(
    "let order = ''; const o = new Proxy({ x: 2 }, { getOwnPropertyDescriptor(target, key) { order += 'd'; return Reflect.getOwnPropertyDescriptor(target, key) } }); const key = { toString() { order += 'k'; return 'x' } }; const d = Reflect.getOwnPropertyDescriptor.call(null, o, key); `${order}:${d.value}`",
    "kd:2"
)]
fn user_owned_descriptors_keep_their_semantics(
    #[case] script: &str,
    #[case] expected: &str,
) -> Result<(), String> {
    // Given / When
    let value = match run(script) {
        Ok(value) => value,
        Err(Failure::Js(error)) => return Err(error.to_string()),
        Err(Failure::Forbidden(_)) => return Err("ordinary descriptor was forbidden".to_string()),
    };
    // Then
    assert_eq!(
        value
            .as_string()
            .ok_or("expected a string")?
            .to_std_string_escaped(),
        expected
    );
    Ok(())
}

#[test]
fn reflected_getter_reads_stay_forbidden_during_serialization() -> Result<(), String> {
    // Given
    let mut context = Context::default();
    let sandbox = Sandbox::new(&mut context).map_err(|error| error.to_string())?;
    let script = "({ get value() { return Object.getOwnPropertyDescriptors(Math).random.value } })";
    let source = super::instrument(script, "reflection.js");
    sandbox
        .prepare(&mut context, &source)
        .map_err(|error| error.to_string())?;
    let value = sandbox
        .run_source(&mut context, Source::from_bytes(&source.code))
        .map_err(|_| "object creation failed")?;
    let json = context.intrinsics().objects().json();
    let stringify = json
        .get(boa_engine::js_string!("stringify"), &mut context)
        .map_err(|error| error.to_string())?
        .to_object(&mut context)
        .map_err(|error| error.to_string())?;
    // When
    let error = stringify
        .call(&JsValue::undefined(), &[value], &mut context)
        .err()
        .ok_or("reflection serialized silently")?;
    // Then
    let Err(Failure::Forbidden(violations)) = sandbox.check(&[&error]) else {
        return Err("serialization lost forbidden evidence".to_string());
    };
    assert_eq!(violations[0].name(), "Math.random");
    assert_eq!(
        violations[0].site().map(|(_, offset)| offset),
        script.find("Object.getOwnPropertyDescriptors")
    );
    Ok(())
}

#[test]
fn reflect_primitive_target_keeps_its_native_type_error() -> Result<(), String> {
    // Given / When
    let value = run("try { Reflect.getOwnPropertyDescriptor(1, 'x') } catch (error) { String(error instanceof TypeError) }")
        .map_err(|_| "primitive reflection did not preserve its catchable error")?;
    // Then
    assert_eq!(
        value
            .as_string()
            .ok_or("expected a caught error")?
            .to_std_string_escaped(),
        "true"
    );
    Ok(())
}

#[test]
#[serial_test::serial]
fn stylesheet_descriptor_branch_fails_instead_of_emitting_blue_css() -> Result<(), String> {
    // Given
    let code = "import { style } from '@vanilla-extract/css';\nexport const card = style({ color: Object.getOwnPropertyDescriptors(Math).random.value ? 'red' : 'blue' });";
    let offset = code
        .find("Object.getOwnPropertyDescriptors")
        .ok_or("missing read")?;
    // When
    let error = crate::extract(
        "reflected-random.css.ts",
        code,
        crate::ExtractOption {
            import_aliases: std::collections::HashMap::from([(
                "@vanilla-extract/css".to_string(),
                crate::ImportAlias::NamedToNamed,
            )]),
            ..crate::ExtractOption::default()
        },
    )
    .err()
    .ok_or("reflection silently emitted CSS")?
    .to_string();
    // Then
    let place = crate::locate("reflected-random.css.ts", code, offset);
    assert!(error.starts_with(&format!("{place}:")), "{error}");
    assert!(error.contains("`Math.random`"), "{error}");
    assert!(error.contains(super::Kind::Random.requirement()), "{error}");
    assert!(
        error.contains("Fix: use a literal or a CSS variable"),
        "{error}"
    );
    Ok(())
}
