use rstest::rstest;

use super::{ValueType, inspect};

#[rstest]
#[case("const n:true=read(); useValue(n)", ValueType::Unproven)]
#[case("const n:false=read(); useValue(n)", ValueType::Unproven)]
#[case(
    "interface P<T> { size: number } function f(p:P<string>){useValue(p.size)}",
    ValueType::Unproven
)]
#[case(
    "interface P{[key:string]:number;size:string} function f(p:P){useValue(p.size)}",
    ValueType::String
)]
#[case(
    "interface P{():number;new():number;size:string} function f(p:P){useValue(p.size)}",
    ValueType::String
)]
#[case(
    "type P = { size: number } & { text: string } & { size: 2 };function f(p:P){useValue(p.text)}",
    ValueType::String
)]
#[case("const n:bigint|string=read();useValue(n*2)", ValueType::Unproven)]
#[case("const n:string|bigint=read();useValue(n*2)", ValueType::Unproven)]
fn annotations_keep_passthrough_boundaries_when_non_scalar_contracts_are_present(
    #[case] source: &str,
    #[case] expected: ValueType,
) {
    // Given: signatures and unknown branches must not manufacture scalar proofs.
    // When: source-only annotation resolution inspects the actual member or operand.
    let (actual, _) = inspect(source, None);
    // Then: only a declared string field proves passthrough.
    assert_eq!(actual, vec![expected]);
}

#[test]
fn angle_assertion_does_not_manufacture_a_proof_when_the_operand_is_unknown() {
    // Given: TS angle assertions are syntax only, just like `as` assertions.
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(
        &allocator,
        "useValue(<number>read(), <string>2)",
        oxc_span::SourceType::ts(),
    )
    .parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    // When: capture resolves the authored operands rather than trusting the assertion.
    let (_scope, _) = super::Scope::enter(&parsed.program, "/entry.ts", None, "@devup-ui/react");
    let actual = super::read(&parsed.program);
    // Then: the unknown call remains unproven and the literal remains a number.
    assert_eq!(actual, vec![ValueType::Unproven, ValueType::Number]);
}

#[rstest]
#[case("useValue('auto')", ValueType::NonNumericString)]
#[case("useValue('10px')", ValueType::NonNumericString)]
#[case("useValue(' 4 ')", ValueType::NonNumericString)]
#[case("useValue('4')", ValueType::String)]
#[case("useValue('-2')", ValueType::String)]
#[case("useValue('0.5')", ValueType::String)]
#[case("useValue('Infinity')", ValueType::String)]
#[case("useValue('NaN')", ValueType::String)]
#[case("useValue('\\x34')", ValueType::String)]
#[case("useValue(`\\x34`)", ValueType::String)]
#[case("useValue(`10px`)", ValueType::NonNumericString)]
#[case("useValue(`${read()}px`)", ValueType::NonNumericString)]
#[case("useValue(`${read()}PX`)", ValueType::NonNumericString)]
#[case("useValue(`${read()}%`)", ValueType::NonNumericString)]
#[case("useValue(`${read()}ms`)", ValueType::NonNumericString)]
#[case("useValue(`${read()}\\x70x`)", ValueType::NonNumericString)]
#[case("useValue(`${read()}`)", ValueType::String)]
#[case("useValue(`1${read()}2`)", ValueType::String)]
#[case(
    "const n:'auto'|'10px'=read(); useValue(n)",
    ValueType::NonNumericString
)]
#[case("const n:'auto'|'4'=read(); useValue(n)", ValueType::String)]
#[case("const n:'4'|'auto'=read(); useValue(n)", ValueType::String)]
#[case("const n:string|'10px'=read(); useValue(n)", ValueType::String)]
#[case("const n:'10px'|number=read(); useValue(n)", ValueType::Unproven)]
#[case(
    "const n:`${number}px`=read(); useValue(n)",
    ValueType::NonNumericString
)]
#[case(
    "const n:`${string}rem`=read(); useValue(n)",
    ValueType::NonNumericString
)]
#[case("const n:`${number}`=read(); useValue(n)", ValueType::String)]
#[case("const n:`10px`=read(); useValue(n)", ValueType::NonNumericString)]
#[case(
    "type N='auto'|'10px'; function f(n:N) {useValue(n)}",
    ValueType::NonNumericString
)]
#[case(
    "interface P {size:'auto'|'10px'} function f({size}:P) {useValue(size)}",
    ValueType::NonNumericString
)]
#[case(
    "declare function f(): 'auto'|'10px'; useValue(f())",
    ValueType::NonNumericString
)]
#[case("useValue(flag?'auto':`${read()}px`)", ValueType::NonNumericString)]
#[case("useValue(''+'4')", ValueType::String)]
fn nonnumeric_proof_when_source_excludes_shared_numeric_grammar(
    #[case] code: &str,
    #[case] expected: ValueType,
) {
    // Given: literals, annotations and templates on both sides of the numeric grammar boundary.
    let source = code;
    // When: source-only proof capture resolves the expression without evaluating it.
    let (actual, _) = inspect(source, None);
    // Then: passthrough is proven only when the string cannot need static numeric conversion.
    assert_eq!(actual, vec![expected]);
}
