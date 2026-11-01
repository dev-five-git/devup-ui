use super::extract_value;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::clock_call("Date()")]
#[case::call_ignores_arguments("Date(0)")]
#[case::clock_constructor("new Date()")]
#[case::clock_method("Date.now()")]
#[case::clock_read("Date.now")]
#[case::parse("Date.parse('2020-01-01')")]
#[case::parse_read("Date.parse")]
#[case::string_constructor("new Date('2020-01-01T00:00:00Z')")]
#[case::local_constructor("new Date(2020, 0, 1)")]
#[case::local_date("new Date(0).getDate()")]
#[case::local_day("new Date(0).getDay()")]
#[case::local_year("new Date(0).getFullYear()")]
#[case::local_legacy_year("new Date(0).getYear()")]
#[case::local_month("new Date(0).getMonth()")]
#[case::local_hours("new Date(0).getHours()")]
#[case::local_minutes("new Date(0).getMinutes()")]
#[case::local_seconds("new Date(0).getSeconds()")]
#[case::local_milliseconds("new Date(0).getMilliseconds()")]
#[case::timezone("new Date(0).getTimezoneOffset()")]
#[case::stringify("new Date(0).toString()")]
#[case::date_string("new Date(0).toDateString()")]
#[case::time_string("new Date(0).toTimeString()")]
#[case::locale_string("new Date(0).toLocaleString()")]
#[case::locale_date("new Date(0).toLocaleDateString()")]
#[case::locale_time("new Date(0).toLocaleTimeString()")]
#[case::implicit_string("String(new Date(0))")]
#[case::primitive_default("new Date(0)[Symbol.toPrimitive]('default')")]
#[case::primitive_string("Date.prototype[Symbol.toPrimitive].call(new Date(0), 'string')")]
#[case::prototype_constructor("new (Object.getPrototypeOf(new Date(0)).constructor)()")]
#[case::constructor_descriptor(
    "new (Object.getOwnPropertyDescriptor(Date.prototype, 'constructor').value)()"
)]
#[case::global_descriptor("Object.getOwnPropertyDescriptor(globalThis, 'Date').value.now()")]
#[case::reflected_now("Reflect.getOwnPropertyDescriptor(Date, 'now')")]
#[case::reflected_parse("Object.getOwnPropertyDescriptors(Date)")]
#[case::reflected_local("Object.getOwnPropertyDescriptor(Date.prototype, 'getHours')")]
#[case::legacy_local("Date.prototype.__lookupGetter__('getHours')")]
#[case::borrowed_local("Date.prototype.getHours.call(new Date(0))")]
#[case::reflect_construct("Reflect.construct(Date, [])")]
#[case::bound_constructor("new (Date.bind(null))()")]
#[case::eval_clock("eval('Date.now()')")]
#[case::function_clock("Object.constructor('return Date.now()')()")]
#[case::object_constructor("new Date({ valueOf() { return 0; } })")]
#[case::undefined_constructor("new Date(undefined)")]
#[case::boolean_constructor("new Date(true)")]
#[case::null_constructor("new Date(null)")]
#[case::local_setter("new Date(0).setHours(1)")]
#[case::utc_mutation_outside_read_subset("new Date(0).setUTCFullYear(2020)")]
#[case::computed_constructor("new (Object.getPrototypeOf(new Date(0))['constructor'])()")]
#[case::computed_now("Object.getOwnPropertyDescriptor(globalThis, 'Date').value['now']()")]
#[case::borrowed_primitive("Date.prototype[Symbol.toPrimitive].call(new Date(0), 'default')")]
#[case::date_copy_outside_numeric_subset("new Date(new Date(0))")]
#[case::string_coercion_outside_numeric_subset(
    "new Date({[Symbol.toPrimitive]() { return '0'; }})"
)]
#[case::argument_effect("Date.UTC({valueOf() { return Date.now(); }}, 0, 1)")]
#[case::utc_reader_argument_effect("new Date(0).getUTCDate(Date.now())")]
#[case::numeric_constructor_argument_effect("new Date((Date.now(), 0))")]
#[case::explicit_receiver_call("Date.call(Date, 0)")]
#[case::explicit_receiver_apply("Reflect.apply(Date, Date, [0])")]
#[case::computed_clock_call("new (new Date(0)['constructor'])()")]
#[case::serialization_outside_read_subset("new Date(0).toJSON()")]
#[case::implicit_serialization("JSON.stringify(new Date(0))")]
#[serial]
fn forbidden_date_fails_when_entry_or_helper_catches_or_propagates(
    #[case] expression: &str,
    #[values(false, true)] imported: bool,
    #[values(false, true)] caught: bool,
) -> Result<(), String> {
    // Given
    let body = if caught {
        format!(
            "try {{ {expression}; }} catch (error) {{ error.message = 'changed'; error.name = 'changed'; delete error.stack; }}\nreturn 1;"
        )
    } else {
        format!("return {expression};")
    };
    // When
    let error = extract_value(&body, imported)
        .err()
        .ok_or("forbidden Date operation emitted CSS")?;
    // Then
    let path = if imported {
        "date-helper.ts:2:"
    } else {
        "date-entry.css.ts:3:"
    };
    assert!(error.starts_with(path), "{expression}: {error}");
    assert!(error.contains("`Date"), "{error}");
    assert!(!error.contains("<devup-sandbox>"), "{error}");
    assert!(!error.contains("__devup_read_site_"), "{error}");
    Ok(())
}
