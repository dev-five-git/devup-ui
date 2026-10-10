use oxc_allocator::Allocator;
use oxc_ast::ast::{CallExpression, Expression, Program};
use oxc_ast_visit::{Visit, VisitMut, walk};
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use std::fmt::Write;

use super::{Scope, ValueType, classify};
use crate::ModuleResolver;

#[path = "source_value_type_module_tests.rs"]
mod modules;
#[path = "source_value_type_string_tests.rs"]
mod strings;

struct ReadProofs(Vec<ValueType>);

impl<'a> Visit<'a> for ReadProofs {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        if matches!(&call.callee, Expression::Identifier(name) if name.name == "useValue") {
            self.0.extend(
                call.arguments
                    .iter()
                    .filter_map(|argument| argument.as_expression().map(classify)),
            );
        }
        walk::walk_call_expression(self, call);
    }
}

fn read(program: &Program<'_>) -> Vec<ValueType> {
    let mut proofs = ReadProofs(Vec::new());
    proofs.visit_program(program);
    proofs.0
}

fn inspect(code: &str, resolver: Option<&ModuleResolver>) -> (Vec<ValueType>, Vec<String>) {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, SourceType::tsx()).parse();
    assert_eq!(
        parsed.diagnostics.len(),
        0,
        "{code}: {:?}",
        parsed.diagnostics
    );
    let (_scope, dependencies) =
        Scope::enter(&parsed.program, "/entry.tsx", resolver, "@devup-ui/react");
    (read(&parsed.program), dependencies.into_iter().collect())
}

#[rstest]
#[case("useValue(2)", ValueType::Number)]
#[case("useValue('2')", ValueType::String)]
#[case("useValue(`1${read()}2`)", ValueType::String)]
#[case("useValue(10n)", ValueType::Unproven)]
#[case("useValue(true)", ValueType::Unproven)]
#[case("useValue(null)", ValueType::Unproven)]
#[case("useValue(read() * other())", ValueType::Number)]
#[case("useValue(read() - other())", ValueType::Number)]
#[case("useValue(read() / other())", ValueType::Number)]
#[case("useValue(read() % other())", ValueType::Number)]
#[case("useValue(+read())", ValueType::Number)]
#[case("useValue(-read())", ValueType::Number)]
#[case("useValue(1n * 2n)", ValueType::Unproven)]
#[case("useValue(1n - 2n)", ValueType::Unproven)]
#[case("useValue(1n / 2n)", ValueType::Unproven)]
#[case("useValue(1n % 2n)", ValueType::Unproven)]
#[case("useValue(+1n)", ValueType::Unproven)]
#[case("useValue(-1n)", ValueType::Unproven)]
#[case("useValue(1 + 2)", ValueType::Number)]
#[case("useValue(read() + other())", ValueType::Unproven)]
#[case("useValue(read() + 'px')", ValueType::String)]
#[case("useValue('px' + read())", ValueType::String)]
#[case("useValue(1n + 2n)", ValueType::Unproven)]
#[case("useValue(1 > 2)", ValueType::Unproven)]
#[case("useValue(!read())", ValueType::Unproven)]
#[case("useValue(typeof read())", ValueType::NonNumericString)]
#[case("useValue(flag ? 1 : 2)", ValueType::Number)]
#[case("useValue(flag ? 'a' : 'b')", ValueType::NonNumericString)]
#[case("useValue(flag ? 1 : '2')", ValueType::Unproven)]
#[case("useValue((flag ? 1n : 1) * 2)", ValueType::Unproven)]
#[case("useValue(1 && 2)", ValueType::Number)]
#[case("useValue(1 || 2)", ValueType::Number)]
#[case("useValue(1 ?? 2)", ValueType::Number)]
#[case("useValue(flag && 2)", ValueType::Unproven)]
#[case("useValue((read(), 2))", ValueType::Number)]
#[case("useValue((read(), '2'))", ValueType::String)]
#[case("useValue((read() as number))", ValueType::Unproven)]
#[case("useValue((2 satisfies number))", ValueType::Number)]
#[case("useValue(read()!)", ValueType::Unproven)]
#[case("useValue(read<number>)", ValueType::Unproven)]
#[case("const n = 2; const alias = n; useValue(alias)", ValueType::Number)]
#[case("let n = 2; useValue(n)", ValueType::Unproven)]
#[case("var n = 2; useValue(n)", ValueType::Unproven)]
#[case("const n: number = read(); useValue(n)", ValueType::Number)]
#[case("let n: string = read(); useValue(n)", ValueType::String)]
#[case("const n: any = 2; useValue(n)", ValueType::Unproven)]
#[case("const n: unknown = 2; useValue(n)", ValueType::Unproven)]
#[case("const n: number | 1 | 2 = read(); useValue(n)", ValueType::Number)]
#[case(
    "const n: 'a' | 'b' = read(); useValue(n)",
    ValueType::NonNumericString
)]
#[case("const n: 1 | -2 = read(); useValue(n)", ValueType::Number)]
#[case(
    "const n: bigint | number = read(); useValue(n * 2)",
    ValueType::Unproven
)]
#[case("const n: 1n | 2n = read(); useValue(n * 2n)", ValueType::Unproven)]
#[case("const n: `size-${number}` = read(); useValue(n)", ValueType::String)]
#[case("const n: number | string = read(); useValue(n)", ValueType::Unproven)]
#[case(
    "type N = (number); const n: N = read(); useValue(n)",
    ValueType::Number
)]
#[case(
    "type N=number; function f() {type N=string; const n:N=read(); useValue(n)}",
    ValueType::String
)]
#[case("type N = N; const n: N = read(); useValue(n)", ValueType::Unproven)]
#[case(
    "type N = T; type T = N; const n: N = read(); useValue(n)",
    ValueType::Unproven
)]
#[case(
    "type N<T> = number; const n: N<string> = read(); useValue(n)",
    ValueType::Unproven
)]
#[case(
    "type N = keyof { p: number }; const n: N = read(); useValue(n)",
    ValueType::Unproven
)]
#[case(
    "type N = readonly number[]; const n: N = read(); useValue(n[0])",
    ValueType::Unproven
)]
#[case(
    "const n: number = read(); function f(n: string) {useValue(n)}",
    ValueType::String
)]
#[case(
    "const n: number = read(); function f(n) {useValue(n)}",
    ValueType::Unproven
)]
#[case("function f(n: number) {useValue(n)}", ValueType::Number)]
#[case("function f(n?: number) {useValue(n)}", ValueType::Unproven)]
#[case("function f(n = 2) {useValue(n)}", ValueType::Unproven)]
#[case(
    "function f(): number {return read()} useValue(f())",
    ValueType::Number
)]
#[case("function f(): any {return 2} useValue(f())", ValueType::Unproven)]
#[case("function f() {return 2} useValue(f())", ValueType::Unproven)]
#[case(
    "async function f(): number {return 2} useValue(f())",
    ValueType::Unproven
)]
#[case("function* f(): number {yield 2} useValue(f())", ValueType::Unproven)]
#[case(
    "function f<T>(): number {return 2} useValue(f())",
    ValueType::Unproven
)]
#[case("const f = (): string => read(); useValue(f())", ValueType::String)]
#[case(
    "const f = async (): string => read(); useValue(f())",
    ValueType::Unproven
)]
#[case("const f = <T,>(): number => 2; useValue(f())", ValueType::Unproven)]
#[case("const f: () => number = read(); useValue(f())", ValueType::Number)]
#[case(
    "const f: <T>() => number = read(); useValue(f())",
    ValueType::Unproven
)]
#[case(
    "declare function f(): number; declare function f(): string; useValue(f())",
    ValueType::Unproven
)]
#[case(
    "function f(): number {return 2} f = read(); useValue(f())",
    ValueType::Unproven
)]
fn scalar_proof_when_source_contract_is_known(#[case] code: &str, #[case] expected: ValueType) {
    // Given: source whose observable assigned value has the stated source contract.
    let source = code;
    // When: proof capture parses and resolves it without evaluating it.
    let (actual, _) = inspect(source, None);
    // Then: only the contract's scalar kind is proven.
    assert_eq!(actual, vec![expected]);
}

#[rstest]
#[case(
    "function f(p: { size: number }) { useValue(p.size) }",
    ValueType::Number
)]
#[case(
    "interface P { size: number } function f(p:P) {useValue(p['size'])}",
    ValueType::Number
)]
#[case(
    "interface B { size: number } interface P extends B { text: string } function f(p:P) {useValue(p.size)}",
    ValueType::Number
)]
#[case(
    "interface P { size: number } interface P { text: string } function f(p:P) {useValue(p.text)}",
    ValueType::String
)]
#[case(
    "type P = { size: number } & { text: string }; function f(p:P) {useValue(p.size)}",
    ValueType::Number
)]
#[case(
    "type P = { size: number } | { size: 2 }; function f(p:P) {useValue(p.size)}",
    ValueType::Number
)]
#[case(
    "type P = { size: number } | { text: string }; function f(p:P) {useValue(p.size)}",
    ValueType::Unproven
)]
#[case(
    "type P = { size: number } | string; function f(p:P) {useValue(p.size)}",
    ValueType::Unproven
)]
#[case(
    "type P = {size?:number}; function f(p:P) {useValue(p.size)}",
    ValueType::Unproven
)]
#[case(
    "interface P {get size():number} function f(p:P) {useValue(p.size)}",
    ValueType::Number
)]
#[case(
    "interface P {set size(n:number)} function f(p:P) {useValue(p.size)}",
    ValueType::Unproven
)]
#[case(
    "interface P {size():number} function f(p:P) {useValue(p.size())}",
    ValueType::Number
)]
#[case(
    "interface P {size?():number} function f(p:P) {useValue(p.size())}",
    ValueType::Unproven
)]
#[case(
    "function f({ size: n }: { size: number }) {useValue(n)}",
    ValueType::Number
)]
#[case(
    "function f({nested:{size:n}}: {nested:{size:string}}) {useValue(n)}",
    ValueType::String
)]
#[case("const { size: n } = { size: 2 }; useValue(n)", ValueType::Number)]
#[case("const [n] = [2]; useValue(n)", ValueType::Number)]
#[case("const {n=2}: {n:number} = read(); useValue(n)", ValueType::Number)]
#[case("const {n=2} = read(); useValue(n)", ValueType::Unproven)]
#[case(
    "const {n,...rest}: {n:number} = read(); useValue(rest.n)",
    ValueType::Unproven
)]
#[case("const [n,...rest] = [2,3]; useValue(rest[0])", ValueType::Unproven)]
#[case(
    "const object = { size: 2 }; object.size='4px'; useValue(object.size)",
    ValueType::Unproven
)]
#[case("useValue(({ size: 2 }).size)", ValueType::Number)]
#[case("useValue(({size:2,size:'4px'}).size)", ValueType::NonNumericString)]
#[case("useValue(({size:2,...read()}).size)", ValueType::Unproven)]
#[case("useValue(({[read()]:2}).size)", ValueType::Unproven)]
#[case(
    "useValue(({get size():number {return read()}}).size)",
    ValueType::Number
)]
#[case("useValue(({get size() {return 2}}).size)", ValueType::Unproven)]
#[case("useValue(({set size(n) {}}).size)", ValueType::Unproven)]
#[case("useValue([2,'4px'][0])", ValueType::Number)]
#[case("useValue([2,'4px'][1])", ValueType::NonNumericString)]
#[case("useValue([2][9])", ValueType::Unproven)]
#[case("useValue([,2][0])", ValueType::Unproven)]
#[case("useValue([2,...read()][0])", ValueType::Unproven)]
#[case("useValue([2][index])", ValueType::Unproven)]
#[case(
    "function f(p: { size: number }) {useValue(p?.size)}",
    ValueType::Unproven
)]
#[case(
    "function f(p: {size:()=>number}) {useValue(p.size?.())}",
    ValueType::Unproven
)]
fn member_proof_when_annotations_or_eager_literals_define_fields(
    #[case] code: &str,
    #[case] expected: ValueType,
) {
    // Given: member reads, including mutable and absent-field boundaries.
    let source = code;
    // When: the source proof subsystem resolves the selected value.
    let (actual, _) = inspect(source, None);
    // Then: unsafe initializer/member assumptions do not prove a scalar.
    assert_eq!(actual, vec![expected]);
}

#[test]
fn outer_facts_return_when_nested_scope_drops() {
    // Given: two extracted programs with overlapping source positions.
    let outer_allocator = Allocator::default();
    let inner_allocator = Allocator::default();
    let outer = Parser::new(
        &outer_allocator,
        "const n: number=read(); useValue(n)",
        SourceType::tsx(),
    )
    .parse();
    let inner = Parser::new(
        &inner_allocator,
        "const n: string=read(); useValue(n)",
        SourceType::tsx(),
    )
    .parse();
    let (_outer, _) = Scope::enter(&outer.program, "/outer.tsx", None, "@devup-ui/react");
    {
        let (_inner, _) = Scope::enter(&inner.program, "/inner.tsx", None, "@devup-ui/react");
        assert_eq!(read(&inner.program), vec![ValueType::String]);
    }
    // When: the nested extraction has left its scope.
    let actual = read(&outer.program);
    // Then: the original source proofs are restored rather than stale inner facts.
    assert_eq!(actual, vec![ValueType::Number]);
}

#[test]
fn source_proof_survives_when_provenance_marks_the_expression() {
    // Given: an annotated runtime read captured before provenance changes spans.
    let allocator = Allocator::default();
    let mut parsed = Parser::new(
        &allocator,
        "const n: number=read(); useValue(n)",
        SourceType::tsx(),
    )
    .parse();
    let (_scope, _) = Scope::enter(&parsed.program, "/entry.tsx", None, "@devup-ui/react");
    // When: the existing risky-range marking runs.
    crate::provenance::MarkRanges(&[(0, 100)]).visit_program(&mut parsed.program);
    // Then: the original source-position proof remains available.
    assert_eq!(read(&parsed.program), vec![ValueType::Number]);
}

#[test]
fn facts_are_absent_when_extraction_scope_has_finished() {
    // Given: an annotated read captured only inside its extraction scope.
    let allocator = Allocator::default();
    let parsed = Parser::new(
        &allocator,
        "const n: number=read(); useValue(n)",
        SourceType::tsx(),
    )
    .parse();
    {
        let (_scope, _) = Scope::enter(&parsed.program, "/entry.tsx", None, "@devup-ui/react");
    }
    // When: the expression is queried after the guard has dropped.
    let actual = read(&parsed.program);
    // Then: no stale proof leaks into another extraction.
    assert_eq!(actual, vec![ValueType::Unproven]);
}

#[test]
fn deeply_recursive_aliases_are_unproven_when_resolution_budget_is_exhausted() {
    // Given: a source alias chain too deep for bounded source-only resolution.
    let mut code = String::new();
    for index in 0..80 {
        write!(code, "type N{index} = N{};", index + 1).unwrap_or_else(|error| panic!("{error}"));
    }
    code.push_str("type N80 = number; const n:N0=read(); useValue(n)");
    // When: the bounded resolver captures its expression proofs.
    let (actual, _) = inspect(&code, None);
    // Then: it falls back instead of recursing without a limit or claiming a number.
    assert_eq!(actual, vec![ValueType::Unproven]);
}
