use boa_engine::{Context, JsValue, Source, js_string};
use rstest::rstest;

use super::{Failure, Sandbox};

fn run(script: &str) -> Result<JsValue, Failure> {
    let mut context = Context::default();
    let sandbox = Sandbox::new(&mut context).map_err(Failure::Js)?;
    let source = super::instrument(script, "fixture.js");
    sandbox
        .prepare(&mut context, &source)
        .map_err(Failure::Js)?;
    sandbox.run_source(
        &mut context,
        Source::from_bytes(&source.code).with_path(std::path::Path::new("fixture.js")),
    )
}

fn text(script: &str) -> Result<String, String> {
    let mut context = Context::default();
    let sandbox = Sandbox::new(&mut context).map_err(|error| error.to_string())?;
    let source = super::instrument(script, "fixture.js");
    sandbox
        .prepare(&mut context, &source)
        .map_err(|error| error.to_string())?;
    let value = match sandbox.run_source(
        &mut context,
        Source::from_bytes(&source.code).with_path(std::path::Path::new("fixture.js")),
    ) {
        Ok(value) => value,
        Err(Failure::Js(error)) => return Err(error.to_string()),
        Err(Failure::Forbidden(_)) => return Err("unexpected forbidden read".to_string()),
    };
    value
        .to_string(&mut context)
        .map(|value| value.to_std_string_escaped())
        .map_err(|error| error.to_string())
}

fn reads(script: &str) -> Result<Vec<String>, String> {
    match run(script) {
        Err(Failure::Forbidden(violations)) => Ok(violations
            .iter()
            .map(|violation| violation.name().to_string())
            .collect()),
        Err(Failure::Js(error)) => Err(format!("failed another way: {error}")),
        Ok(_) => Err("read nothing forbidden".to_string()),
    }
}

#[test]
fn deterministic_computation_and_console_calls_run() -> Result<(), String> {
    assert_eq!(
        text(
            "console.log('a'); console.table(1); `${Math.max(1, 2)}${JSON.stringify({ a: [1] })}${'b'.toUpperCase()}`"
        )?,
        r#"2{"a":[1]}B"#
    );
    Ok(())
}

#[rstest]
#[case("Date.now()", "Date")]
#[case("new Date()", "Date")]
#[case("Date()", "Date")]
#[case("Date.UTC(2020, 0)", "Date")]
#[case("globalThis['Da' + 'te'].parse('x')", "Date")]
#[case("const clock = Date; clock.now()", "Date")]
#[case("Math.random()", "Math.random")]
#[case("Math.random", "Math.random")]
#[case("performance.now()", "performance")]
#[case("performance.timeOrigin", "performance")]
#[case("performance.memory", "performance")]
#[case("crypto.randomUUID()", "crypto")]
#[case("crypto.getRandomValues(new Uint8Array(1))", "crypto")]
#[case("crypto.subtle", "crypto")]
#[case("typeof window", "window")]
#[case("typeof process", "process")]
#[case("navigator.language", "navigator")]
#[case("fetch('x')", "fetch")]
#[case("new Intl.NumberFormat('en')", "Intl")]
#[case("Intl.getCanonicalLocales('en')", "Intl")]
#[case("Boolean(Intl)", "Intl")]
#[case("typeof Intl", "Intl")]
#[case("typeof Date", "Date")]
#[case("typeof Temporal", "Temporal")]
#[case("(1).toLocaleString()", "Number.prototype.toLocaleString")]
#[case("[1].toLocaleString()", "Array.prototype.toLocaleString")]
#[case("({}).toLocaleString()", "Object.prototype.toLocaleString")]
#[case("1n.toLocaleString()", "BigInt.prototype.toLocaleString")]
#[case(
    "new Uint8Array(1).toLocaleString()",
    "TypedArray.prototype.toLocaleString"
)]
#[case("'a'.localeCompare('b')", "String.prototype.localeCompare")]
#[case("'a'.toLocaleUpperCase()", "String.prototype.toLocaleUpperCase")]
#[case("'A'.toLocaleLowerCase()", "String.prototype.toLocaleLowerCase")]
#[case("'a'.normalize()", "String.prototype.normalize")]
#[case("const normalize = 'a'.normalize; 1", "String.prototype.normalize")]
#[case(
    "const key = 'normalize'; const normalize = 'a'[key]; 1",
    "String.prototype.normalize"
)]
#[case("const global = globalThis; global['Intl']", "Intl")]
#[case("setTimeout", "setTimeout")]
#[case("typeof require", "require")]
#[case("const { random } = Math; 1", "Math.random")]
#[case("const { normalize: method } = 'a'; 1", "String.prototype.normalize")]
#[case("eval('typeof Intl')", "Intl")]
fn a_forbidden_read_fails_the_run(#[case] script: &str, #[case] name: &str) -> Result<(), String> {
    assert_eq!(reads(script)?, [name]);
    Ok(())
}

#[rstest]
#[case("try { Date.now() } catch { 1 }")]
#[case("try { Math.random() } catch (error) { throw new Error('other') }")]
#[case("try { window } catch {} 'fine'")]
#[case("const read = () => { try { return performance.now() } catch { return 0 } }; read()")]
#[case("function f() { return Date.now() } try { f() } catch (error) { delete error.message }")]
#[case("({ get a() { return Date.now() } }).a")]
#[case("JSON.stringify({ get a() { return Math.random() } })")]
#[case("[1, 2].map(() => crypto.getRandomValues(new Uint8Array(1)))")]
fn a_caught_forbidden_read_still_fails_the_run(#[case] script: &str) -> Result<(), String> {
    assert_eq!(reads(script)?.len(), 1, "{script}");
    Ok(())
}

#[test]
fn every_read_is_listed_in_order() -> Result<(), String> {
    assert_eq!(
        reads(
            "try { Date.now() } catch {} try { window } catch {} try { 'a'.normalize() } catch {}"
        )?,
        ["Date", "window", "String.prototype.normalize"]
    );
    Ok(())
}

#[test]
fn code_cannot_clear_the_evidence() -> Result<(), String> {
    assert_eq!(reads("try { Date.now() } catch {} const global = globalThis; for (const key of Reflect.ownKeys(global)) { try { delete global[key] } catch {} } Object.freeze(Object.prototype); 1")?.first().map(String::as_str), Some("Date"));
    Ok(())
}

#[rstest]
#[case(
    "const history = [1, 2]; var screen = 'big'; globalThis.document = { a: 1 }; const Date = 7; `${history.length}${screen}${document.a}${Date}`",
    "2big17"
)]
#[case("const Intl = 3; Boolean(Intl)", "true")]
#[case(
    "const o = { value: 7, get read() { return this.value }, method() { return this.read } }; o.method()",
    "7"
)]
#[case("let o = null; o?.normalize()", "undefined")]
#[case("let o = null; o?.x.y", "undefined")]
#[case("let o = null; o?.['x'].y()", "undefined")]
#[case("let o = null; o?.['x']?.y", "undefined")]
#[case("const o = { normalize() { return 4 } }; o.normalize()", "4")]
#[case("let x = 1; eval('x = 3'); x", "3")]
#[case(
    "let o = { x: 1 }; o.x++; o['x'] = 4; delete o.x; JSON.stringify(o)",
    "{}"
)]
#[case("const { x = 4, y: value } = { y: 3 }; `${x}${value}`", "43")]
#[case("const Intl = 3; eval('Intl')", "3")]
fn declared_values_and_receivers_keep_their_semantics(
    #[case] script: &str,
    #[case] expected: &str,
) -> Result<(), String> {
    assert_eq!(text(script)?, expected);
    Ok(())
}

#[test]
fn reads_in_a_host_run_function_are_found_by_the_check() -> Result<(), String> {
    let mut context = Context::default();
    let sandbox = Sandbox::new(&mut context).map_err(|error| error.to_string())?;
    let source = super::instrument("({ get width() { return Date.now() } })", "fixture.js");
    sandbox
        .prepare(&mut context, &source)
        .map_err(|error| error.to_string())?;
    let value = sandbox
        .run_source(
            &mut context,
            Source::from_bytes(&source.code).with_path(std::path::Path::new("fixture.js")),
        )
        .map_err(|_| "creation failed".to_string())?;
    let json = context.intrinsics().objects().json();
    let stringify = json
        .get(js_string!("stringify"), &mut context)
        .map_err(|error| error.to_string())?;
    let error = stringify
        .to_object(&mut context)
        .map_err(|error| error.to_string())?
        .call(&JsValue::undefined(), &[value], &mut context)
        .err()
        .ok_or("serialization succeeded")?;
    let Err(Failure::Forbidden(violations)) = sandbox.check(&[&error]) else {
        return Err("no violation".to_string());
    };
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].name(), "Date");
    assert_eq!(violations[0].site().map(|(_, offset)| offset), Some(24));
    Ok(())
}

#[rstest]
#[case("function helper() {\n  return typeof window\n}\nhelper()", "window")]
#[case(
    "function helper() {\n  return Date\n}\ntry { helper() } catch {}",
    "Date"
)]
#[case(
    "function helper() {\n  return globalThis['Intl']\n}\nhelper()",
    "globalThis"
)]
fn a_read_reports_its_actual_original_site(
    #[case] script: &str,
    #[case] occurrence: &str,
) -> Result<(), String> {
    let Err(Failure::Forbidden(violations)) = run(script) else {
        return Err("no violation".to_string());
    };
    let offset = script
        .find(occurrence)
        .ok_or("fixture occurrence missing")?;
    assert_eq!(violations[0].site().map(|(_, at)| at), Some(offset));
    assert!(violations[0].error().to_string().contains(&crate::locate(
        "fixture.js",
        script,
        offset
    )));
    Ok(())
}

#[test]
fn other_failures_stay_failures_of_their_own() {
    assert!(matches!(
        run("throw new Error('boom')"),
        Err(Failure::Js(_))
    ));
    assert!(matches!(run("for (;;) {}"), Err(Failure::Js(_))));
    assert!(matches!(run("missing.read"), Err(Failure::Forbidden(_))));
    assert!(matches!(run("let a = ;"), Err(Failure::Js(_))));
}

#[test]
fn generated_call_frames_map_back_to_original_columns() -> Result<(), String> {
    let script = "const value = { fail() { throw new TypeError('bad') } };\nvalue.fail();";
    let mut context = Context::default();
    let sandbox = Sandbox::new(&mut context).map_err(|error| error.to_string())?;
    let source = super::instrument(script, "fixture.js");
    sandbox
        .prepare(&mut context, &source)
        .map_err(|error| error.to_string())?;
    let Err(Failure::Js(error)) = sandbox.run_source(
        &mut context,
        Source::from_bytes(&source.code).with_path(std::path::Path::new("fixture.js")),
    ) else {
        return Err("no TypeError".to_string());
    };
    let mapped = source.explain(&error.to_string());
    assert!(mapped.contains("TypeError: bad"), "{mapped}");
    assert!(mapped.contains("fixture.js:2:11"), "{mapped}");
    Ok(())
}
