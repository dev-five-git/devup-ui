use rstest::rstest;

use super::{ValueType, inspect};
use crate::ResolvedModule;

#[path = "source_value_type_package_tests.rs"]
mod packages;

const MODULES: &[(&str, &str)] = &[
    (
        "./numbers",
        "export type Size = number | 1 | 2; export interface Props {size:Size;text:string} export const size:number = runtime(); export const mixed:any = 2; export function width():number {return runtime()} export default width;",
    ),
    (
        "./named",
        "export {size as value, width, type Size, type Props} from './numbers';",
    ),
    ("./star", "export * from './named';"),
    ("./namespace", "export * as tokens from './numbers';"),
    (
        "./default",
        "import type {Props} from './numbers'; const p:Props=runtime(); export default p;",
    ),
    ("./local", "const value:string=runtime(); export {value};"),
    ("./literal", "export default 2;"),
    (
        "./interface",
        "export default interface Props {size:number}",
    ),
    ("./object", "export default {size:2};"),
    (
        "./a",
        "export type Size = importMissing; export {size} from './b';",
    ),
    ("./b", "export {size} from './a';"),
    (
        "./ambiguous",
        "export * from './numbers'; export * from './other';",
    ),
    (
        "./override",
        "export * from './numbers'; export * from './other'; export {size} from './numbers';",
    ),
    ("./other", "export const size:string = runtime();"),
    ("./bad", "export const ="),
    (
        "./default-barrel",
        "export {default as width} from './numbers';",
    ),
];

fn resolver(specifier: &str, _: &str) -> Option<ResolvedModule> {
    MODULES
        .iter()
        .find(|(name, _)| *name == specifier)
        .map(|(_, code)| ResolvedModule {
            path: format!("/{}.ts", specifier.trim_start_matches("./")),
            code: (*code).to_string(),
        })
}

#[rstest]
#[case("import {size} from './numbers'; useValue(size)", ValueType::Number)]
#[case(
    "import {mixed} from './numbers'; useValue(mixed)",
    ValueType::Unproven
)]
#[case(
    "import {width} from './numbers'; useValue(width())",
    ValueType::Number
)]
#[case("import width from './numbers'; useValue(width())", ValueType::Number)]
#[case(
    "import {width} from './default-barrel'; useValue(width())",
    ValueType::Number
)]
#[case(
    "import type {Size} from './numbers'; const n:Size=runtime(); useValue(n)",
    ValueType::Number
)]
#[case(
    "import {type Props} from './numbers'; function f(p:Props) {useValue(p.text)}",
    ValueType::String
)]
#[case(
    "import * as numbers from './numbers'; useValue(numbers.size)",
    ValueType::Number
)]
#[case(
    "import type * as numbers from './numbers'; const n:numbers.Size=runtime(); useValue(n)",
    ValueType::Number
)]
#[case("import {value as n} from './named'; useValue(n)", ValueType::Number)]
#[case(
    "import type {Props} from './named'; function f(p:Props) {useValue(p.size)}",
    ValueType::Number
)]
#[case("import {value} from './star'; useValue(value)", ValueType::Number)]
#[case("import width from './star'; useValue(width())", ValueType::Unproven)]
#[case(
    "import {tokens} from './namespace'; useValue(tokens.size)",
    ValueType::Number
)]
#[case("import p from './default'; useValue(p.size)", ValueType::Number)]
#[case(
    "import type Props from './interface'; function f(p:Props) {useValue(p.size)}",
    ValueType::Number
)]
#[case("import {value} from './local'; useValue(value)", ValueType::String)]
#[case("import value from './literal'; useValue(value)", ValueType::Number)]
#[case(
    "import value from './object'; useValue(value.size)",
    ValueType::Unproven
)]
#[case("import {size} from './a'; useValue(size)", ValueType::Unproven)]
#[case(
    "import {size} from './ambiguous'; useValue(size)",
    ValueType::Unproven
)]
#[case("import {size} from './override'; useValue(size)", ValueType::Number)]
#[case("import {size} from './bad'; useValue(size)", ValueType::Unproven)]
#[case("import {size} from './missing'; useValue(size)", ValueType::Unproven)]
#[case(
    "import {absent} from './numbers'; useValue(absent)",
    ValueType::Unproven
)]
#[case(
    "import * as numbers from './numbers'; useValue(numbers.absent)",
    ValueType::Unproven
)]
#[case(
    "import {size} from './numbers'; function f(size:string) {useValue(size)}",
    ValueType::String
)]
fn imported_proof_when_annotation_graph_resolves(#[case] code: &str, #[case] expected: ValueType) {
    // Given: source modules with runtime initializers that must never execute.
    let source = code;
    // When: resolver-backed source annotation lookup captures the entry.
    let (actual, _) = inspect(source, Some(&resolver));
    // Then: resolved annotations prove types; missing/ambiguous/cyclic paths do not.
    assert_eq!(actual, vec![expected]);
}

#[test]
fn dependencies_include_type_only_reexports_when_the_alias_is_read() {
    // Given: a runtime value whose type is imported through a barrel.
    let source = "import type {Props} from './named'; function f(p:Props) {useValue(p.size)}";
    // When: source type facts follow the annotation graph.
    let (_, dependencies) = inspect(source, Some(&resolver));
    // Then: both files invalidate extraction even though no runtime import reads them.
    assert_eq!(
        dependencies,
        vec!["/named.ts".to_string(), "/numbers.ts".to_string()]
    );
}

#[test]
fn proof_is_unproven_when_module_resolver_is_absent() {
    // Given: an annotation declared only in another module.
    let source = "import type {Size} from './numbers'; const n:Size=runtime(); useValue(n)";
    // When: extraction runs without module access.
    let (actual, dependencies) = inspect(source, None);
    // Then: neither a type proof nor a phantom dependency is invented.
    assert_eq!((actual, dependencies), (vec![ValueType::Unproven], vec![]));
}

#[test]
fn dependencies_reach_output_when_extraction_reads_only_a_type() {
    // Given: a Box value whose only external reference is a type annotation.
    let source = "import {Box} from '@devup-ui/react'; import type {Size} from './numbers'; export function f(n:Size) {return <Box p={n}/>}";
    // When: the actual extraction entry follows the type module.
    let output = crate::extract_with_modules(
        "/entry.tsx",
        source,
        crate::ExtractOption::default(),
        false,
        &resolver,
    );
    // Then: the build plugin receives the type module as an invalidation dependency.
    assert_eq!(
        output
            .map(|output| output.dependencies)
            .map_err(|error| error.to_string()),
        Ok(vec!["/numbers.ts".to_string()])
    );
}

#[test]
fn module_source_is_read_once_when_multiple_values_share_its_annotations() {
    // Given: repeated reads of two exports from the same resolved source.
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let counted = calls.clone();
    let resolver = move |specifier: &str, importer: &str| {
        counted.set(counted.get() + 1);
        super::modules::resolver(specifier, importer)
    };
    // When: one capture resolves all expression facts.
    let (actual, _) = inspect(
        "import {size,width} from './numbers'; useValue(size,width(),size)",
        Some(&resolver),
    );
    // Then: the graph reuses the source while preserving each expression's proof.
    assert_eq!((actual, calls.get()), (vec![ValueType::Number; 3], 1));
}
