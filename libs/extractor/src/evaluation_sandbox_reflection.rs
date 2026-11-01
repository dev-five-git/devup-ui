//! Identity-based protection of synthetic accessors exposed by reflection.

use super::{
    Evidence, INTERNAL_SOURCE, SETUP, forbid_native, function, guards::GUARDED_GLOBALS, record,
};
use boa_engine::{
    Context, JsError, JsResult, JsString, JsValue, NativeFunction, Source, js_string,
};

pub(super) fn guard_globals(context: &mut Context) -> JsResult<()> {
    let setup = context.eval(Source::from_reader(
        SETUP.as_bytes(),
        Some(std::path::Path::new(INTERNAL_SOURCE)),
    ))?;
    let forbid = function(
        context,
        "forbid",
        NativeFunction::from_fn_ptr(forbid_native),
    );
    let config = JsValue::from_json(&serde_json::json!({ "names": GUARDED_GLOBALS }), context)?;
    setup
        .to_object(context)?
        .call(&JsValue::undefined(), &[forbid.into(), config], context)?;
    for name in GUARDED_GLOBALS {
        let descriptor = boa_engine::builtins::object::OrdinaryObject::get_own_property_descriptor(
            &JsValue::undefined(),
            &[context.global_object().into(), JsString::from(name).into()],
            context,
        )?
        .to_object(context)?;
        for key in [js_string!("get"), js_string!("set")] {
            let accessor = descriptor.get(key, context)?.to_object(context)?;
            if let Some(evidence) = context.get_data::<Evidence>() {
                evidence
                    .methods
                    .borrow_mut()
                    .push((accessor, name.to_string()));
            }
        }
    }
    Ok(())
}

pub(super) fn check_descriptor(
    value: &JsValue,
    site: Option<(String, u32)>,
    context: &mut Context,
) -> JsResult<()> {
    if let Some(descriptor) = value.as_object() {
        let getter = descriptor
            .borrow()
            .properties()
            .get(&js_string!("get").into())
            .and_then(|property| property.value().cloned());
        if let Some(getter) = getter {
            check_accessor(&getter, site, context)?;
        }
    }
    Ok(())
}

pub(super) fn check_accessor(
    value: &JsValue,
    site: Option<(String, u32)>,
    context: &mut Context,
) -> JsResult<()> {
    let name = context.get_data::<Evidence>().and_then(|evidence| {
        evidence.methods.borrow().iter().find_map(|(method, name)| {
            value
                .as_object()
                .filter(|accessor| accessor == method)
                .map(|_| name.clone())
        })
    });
    match name {
        Some(name) => Err(JsError::from_opaque(record(context, &name, site))),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Failure, reflection_tests::run};
    use rstest::rstest;

    #[rstest]
    #[case("(()=>{}).__lookupGetter__('toString')", "Function.prototype.toString")]
    #[case("Math.__lookupGetter__('random')", "Math.random")]
    #[case("Math.__lookupSetter__('random')", "Math.random")]
    #[case(
        "const read = Object.prototype.__lookupGetter__;\nread.call(Math, 'random')",
        "Math.random"
    )]
    #[case(
        "const read = Object.prototype.__lookupSetter__.bind(Math);\nread('random')",
        "Math.random"
    )]
    #[case(
        "const key = 'random';\nMath['__lookup' + 'Getter__'](key)",
        "Math.random"
    )]
    #[case(
        "const key = { toString() { Math.max(1, 2); return 'random' } };\nMath.__lookupSetter__(key)",
        "Math.random"
    )]
    #[case(
        "const child = Object.create(Math);\nchild.__lookupGetter__('random')",
        "Math.random"
    )]
    fn legacy_protected_accessors_fail_at_actual_call(
        #[case] script: &str,
        #[case] name: &str,
    ) -> Result<(), String> {
        // Given
        let offset = script.rfind('\n').map_or(0, |at| at + 1);
        // When
        let Err(Failure::Forbidden(reads)) = run(script) else {
            return Err("legacy reflection escaped".into());
        };
        // Then
        assert_eq!(reads.len(), 1);
        assert_eq!(reads[0].name(), name);
        assert_eq!(reads[0].site().map(|(_, at)| at), Some(offset));
        Ok(())
    }

    #[rstest]
    fn legacy_global_accessors_record_before_throw_even_when_errors_are_mutated(
        #[values("__lookupGetter__", "__lookupSetter__")] reader: &str,
        #[values("Date", "Intl", "performance", "crypto", "Temporal")] name: &str,
    ) -> Result<(), String> {
        // Given
        let script = format!(
            "try {{ globalThis.{reader}('{name}') }} catch (error) {{ error.message = 'changed'; error.name = 'changed'; }}\nglobalThis.{name} = 7; 'accepted'"
        );
        // When
        let Err(Failure::Forbidden(reads)) = run(&script) else {
            return Err("caught legacy reflection escaped".into());
        };
        // Then
        assert_eq!(reads.len(), 1);
        assert_eq!(reads[0].name(), name);
        assert_eq!(reads[0].site().map(|(_, at)| at), Some(6));
        assert!(reads[0].error().to_string().contains(&format!("`{name}`")));
        assert_eq!(
            reads[0].requirement(),
            super::super::Kind::of(name).requirement()
        );
        Ok(())
    }

    #[rstest]
    #[case(
        "Math.random = () => 7; String(Math.__lookupGetter__('random'))",
        "undefined"
    )]
    #[case(
        "Math.random = () => 7; String(Math.__lookupSetter__('random'))",
        "undefined"
    )]
    #[case(
        "Function.prototype.toString = () => 'own'; String((()=>{}).__lookupGetter__('toString'))",
        "undefined"
    )]
    #[case(
        "globalThis.Date = 7; String(globalThis.__lookupSetter__('Date'))",
        "undefined"
    )]
    #[case(
        "const o = { n: 3, get random() { return this.n }, set random(v) { this.n = v } }; const child = Object.create(o); child.__lookupSetter__('random').call(child, 8); String(child.__lookupGetter__('random').call(child))",
        "8"
    )]
    #[case(
        "Object.defineProperty(Math, 'random', { get() { return 4 }, set(v) {} }); String(Math.__lookupGetter__('random')())",
        "4"
    )]
    #[case(
        "Object.defineProperty(globalThis, 'Intl', { set(v) { this.saved = v } }); globalThis.__lookupSetter__('Intl').call(globalThis, 9); String(globalThis.saved)",
        "9"
    )]
    #[case(
        "String(Object.prototype.__lookupGetter__.call('a', '0'))",
        "undefined"
    )]
    #[case("String(Object.prototype.__lookupSetter__.call(1, 'x'))", "undefined")]
    #[case("String(({}).__lookupGetter__('missing'))", "undefined")]
    #[case("String(({ random: 2 }).__lookupSetter__('random'))", "undefined")]
    #[case(
        "String(Object.prototype.__lookupGetter__.length) + ':' + Object.prototype.__lookupSetter__.length",
        "1:1"
    )]
    #[case(
        "let order = ''; const o = new Proxy({ get x() { return 2 } }, { getOwnPropertyDescriptor(t, k) { order += 'd'; return Reflect.getOwnPropertyDescriptor(t, k) } }); const key = { toString() { order += 'k'; return 'x' } }; const get = Object.prototype.__lookupGetter__.call(o, key); `${order}:${get()}`",
        "kd:2"
    )]
    #[case(
        "let order = ''; try { Object.prototype.__lookupGetter__.call(null, { toString() { order += 'k'; return 'x' } }) } catch(e) { order += e instanceof TypeError ? 't' : 'e' } order",
        "t"
    )]
    #[case(
        "let order = ''; try { Object.prototype.__lookupSetter__.call(undefined, { toString() { order += 'k'; return 'x' } }) } catch(e) { order += e instanceof TypeError ? 't' : 'e' } order",
        "t"
    )]
    fn legacy_ordinary_accessors_preserve_native_semantics(
        #[case] script: &str,
        #[case] expected: &str,
    ) -> Result<(), String> {
        // Given / When
        let value = match run(script) {
            Ok(value) => value,
            Err(Failure::Js(error)) => return Err(error.to_string()),
            Err(Failure::Forbidden(_)) => return Err("ordinary legacy accessor forbidden".into()),
        };
        // Then
        assert_eq!(
            value
                .as_string()
                .ok_or("expected string")?
                .to_std_string_escaped(),
            expected
        );
        Ok(())
    }

    #[rstest]
    #[case(
        "(()=>{}).__lookupGetter__('toString')",
        "Function.prototype.toString",
        "()=>{}"
    )]
    #[case(
        "Math.__lookupGetter__('random')",
        "Math.random",
        "Math.__lookupGetter__"
    )]
    #[serial_test::serial]
    fn legacy_stylesheet_branches_fail_instead_of_emitting_blue_css(
        #[case] read: &str,
        #[case] name: &str,
        #[case] callsite: &str,
    ) -> Result<(), String> {
        // Given
        let code = format!(
            "import {{style}} from '@devup-ui/react';\nexport const card=style({{color:{read}?'blue':'red'}});"
        );
        let offset = code.find(callsite).ok_or("missing read")?;
        // When
        let error = crate::extract("legacy.css.ts", &code, crate::ExtractOption::default())
            .err()
            .ok_or("legacy reflection silently emitted CSS")?
            .to_string();
        // Then
        assert!(
            error.starts_with(&format!(
                "{}:",
                crate::locate("legacy.css.ts", &code, offset)
            )),
            "{error}"
        );
        assert!(error.contains(&format!("`{name}`")), "{error}");
        assert!(
            error.contains("Fix: use a literal or a CSS variable"),
            "{error}"
        );
        Ok(())
    }
}
