use super::extract_value;
use crate::ExtractStyleValue;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::utc_coercion_order(
    "let order = ''; const year = {valueOf() {order += 'y'; return 2020;}}; const month = {valueOf() {order += 'm'; return 0;}}; const ignored = {valueOf() {throw new Error('extra argument coerced');}}; const n = Date.UTC(year, month, 1, 0, 0, 0, 0, (order += 'e', ignored)); return order === 'eym' && n === 1577836800000;"
)]
#[case::utc_abrupt_conversion(
    "let order = ''; const first = {valueOf() {order += 'a'; throw new TypeError('input');}}; const second = {valueOf() {order += 'b'; return 0;}}; try {Date.UTC(first, second);} catch (error) {return error instanceof TypeError && order === 'a';} return false;"
)]
#[case::reader_receivers(
    "try {Date.prototype.getTime.call({});} catch (error) {return error instanceof TypeError;} return false;"
)]
#[case::utc_reader_receivers(
    "try {Date.prototype.getUTCDate.call({});} catch (error) {return error instanceof TypeError;} return false;"
)]
#[case::proxy_receiver(
    "try {Date.prototype.valueOf.call(new Proxy(new Date(0), {}));} catch (error) {return error instanceof TypeError;} return false;"
)]
#[case::native_slots(
    "const d = new Date(9); return d instanceof Date && Object.getPrototypeOf(d) === Date.prototype && Date.prototype.getTime.call(d) === 9;"
)]
#[case::reader_argument_effects(
    "let effects = 0; const value = new Date(3).getTime(++effects); return effects === 1 && value === 3;"
)]
#[case::constructor_argument_effects(
    "let effects = 0; const value = new Date((++effects, 3)).getTime(); return effects === 1 && value === 3;"
)]
#[case::subclass(
    "class Epoch extends Date {} const date = new Epoch(3); return date instanceof Epoch && date instanceof Date && date.getTime() === 3;"
)]
#[case::reflect_new_target(
    "function Epoch() {} Epoch.prototype = Object.create(Date.prototype); const date = Reflect.construct(Date, [8], Epoch); return Object.getPrototypeOf(date) === Epoch.prototype && date.getTime() === 8;"
)]
#[case::fallback_prototype(
    "function Epoch() {} Epoch.prototype = null; const date = Reflect.construct(Date, [8], Epoch); return Object.getPrototypeOf(date) === Date.prototype && date.getTime() === 8;"
)]
#[case::primitive_number(
    "return Date.prototype[Symbol.toPrimitive].call(new Date(7), 'number') === 7;"
)]
#[case::primitive_invalid_hint(
    "try {Date.prototype[Symbol.toPrimitive].call(new Date(7), 'other');} catch (error) {return error instanceof TypeError;} return false;"
)]
#[case::borrowed_utc(
    "const read = new Date(0).getUTCFullYear; return read.call(new Date(Date.UTC(2020, 0, 1))) === 2020;"
)]
#[case::reflected_utc(
    "return Reflect.getOwnPropertyDescriptor(Date, 'UTC').value.call(null, 2020, 0, 1) === 1577836800000;"
)]
#[case::borrowed_iso(
    "return Date.prototype.toISOString.call(new Date(0)) === '1970-01-01T00:00:00.000Z';"
)]
#[case::pure_replacement(
    "Date.now = () => 7; Date.prototype.getHours = () => 9; return Date.now() === 7 && new Date(0).getHours() === 9;"
)]
#[case::global_replacement(
    "const D = Date; globalThis.Date = () => 5; return Date() === 5 && new D(2).getTime() === 2 && D.UTC(2020) === 1577836800000;"
)]
#[serial]
fn native_semantics_are_preserved_when_date_capability_runs(
    #[case] body: &str,
    #[values(false, true)] imported: bool,
) -> Result<(), String> {
    // Given / When
    let output = extract_value(body, imported)?;
    // Then
    assert!(
        output.styles.iter().any(|style| matches!(style,
            ExtractStyleValue::Static(style) if style.property == "opacity" && style.value == "true"
        )),
        "{:?}",
        output.styles
    );
    Ok(())
}

#[rstest]
#[case("date")]
#[case("{nested: [date]}")]
#[serial]
fn date_exports_are_rejected_at_creation_when_later_reads_change_the_active_site(
    #[case] value: &str,
) -> Result<(), String> {
    // Given
    let source = format!(
        "import {{style}} from '@devup-ui/react';\nconst date = new Date(0);\nconst later = new Date(1).getTime();\nexport const result = {value};"
    );
    let offset = source.find("Date(0)").ok_or("missing creation")?;
    // When
    let error = crate::extract(
        "date-export.css.ts",
        &source,
        crate::ExtractOption::default(),
    )
    .err()
    .ok_or("Date object escaped to runtime output")?
    .to_string();
    // Then
    assert!(
        error.starts_with(&format!(
            "{}:",
            crate::locate("date-export.css.ts", &source, offset)
        )),
        "{error}"
    );
    assert!(error.contains("ReferenceError: `Date`"), "{error}");
    Ok(())
}

#[rstest]
#[case("new Date(0).getTime()")]
#[case("Date.UTC(2020, 0, 1)")]
#[serial]
fn date_expression_stays_on_full_plan_when_written_directly_in_css(#[case] expression: &str) {
    // Given
    let source = format!(
        "import {{css}} from '@devup-ui/react'; export const card = css({{opacity: String({expression})}});"
    );
    // When
    let plan = crate::stylesheet_policy::plan(
        &source,
        "direct-date.css.ts",
        &crate::ExtractOption::default(),
        None,
        &|_| false,
    );
    // Then
    assert_eq!(plan, crate::stylesheet_policy::Plan::Run);
}

#[rstest]
#[case("new Date(0).getTime()", "0")]
#[case("Date.UTC(2020, 0, 1)", "1577836800000")]
#[serial]
fn direct_stylesheet_date_is_static_when_the_approved_style_api_runs(
    #[case] expression: &str,
    #[case] expected: &str,
) -> Result<(), String> {
    // Given
    let source = format!(
        "import {{style}} from '@devup-ui/react'; export const card = style({{opacity: String({expression})}});"
    );
    // When
    let output = crate::extract(
        "direct-date.css.ts",
        &source,
        crate::ExtractOption::default(),
    )
    .map_err(|error| error.to_string())?;
    // Then
    assert!(output.styles.iter().any(|style| matches!(style,
        ExtractStyleValue::Static(style) if style.property == "opacity" && style.value == expected
    )), "{:?}", output.styles);
    assert!(!output.code.contains("Date"), "{}", output.code);
    assert!(!output.code.contains("var(--"), "{}", output.code);
    Ok(())
}

#[rstest]
#[serial]
fn immutable_date_evidence_survives_when_multiple_errors_and_guards_are_mutated(
    #[values(false, true)] imported: bool,
) -> Result<(), String> {
    // Given
    let body = "try {Date.now;} catch (error) {error.message = 'other'; Object.setPrototypeOf(error, null);}\ntry {Date.parse;} catch (error) {delete error.message;}\nDate.now = () => 7; Date.parse = () => 8; return 1;";
    // When
    let error = extract_value(body, imported)
        .err()
        .ok_or("caught Date evidence was discarded")?;
    // Then
    let path = if imported {
        "date-helper.ts"
    } else {
        "date-entry.css.ts"
    };
    let row = if imported { 2 } else { 3 };
    assert!(error.contains(&format!("{path}:{row}:6:")), "{error}");
    assert!(error.contains(&format!("{path}:{}:6:", row + 1)), "{error}");
    assert!(error.contains("`Date.now`"), "{error}");
    assert!(error.contains("`Date.parse`"), "{error}");
    Ok(())
}
