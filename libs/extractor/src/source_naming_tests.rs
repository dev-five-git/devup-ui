use super::*;
use oxc_ast::ast::Statement;
use rstest::rstest;

#[rstest]
#[case(
    "@devup-ui/react",
    concat!("import {", "value", "} from '@devup-ui/react-values';"),
    "value"
)]
#[case("@custom/ui", concat!("import {", "value", "} from '@custom/ui-values';"), "value")]
#[case(
    "@devup-ui/react",
    "import * as sdk from '@devup-ui/react-values';",
    "sdk.value"
)]
#[case(
    "@devup-ui/react",
    "import sdk from '@devup-ui/react-values';",
    "sdk.value"
)]
#[case(
    "@devup-ui/react",
    "const sdk=require('@devup-ui/react-values');",
    "sdk.value"
)]
#[case("@custom/ui", "const sdk=require('@custom/ui-values');", "sdk.value")]
#[case(
    "@devup-ui/react",
    "const sdk=import('@devup-ui/react-values');",
    "sdk.value"
)]
#[case(
    "@devup-ui/react",
    concat!("import {", "value", "} from '@devup-ui/react-values';const local=()=>value;"),
    "local()"
)]
fn result_is_risky_when_import_supplies_data(
    #[case] package: &str,
    #[case] declaration: &str,
    #[case] value: &str,
) {
    // Given: a data package resembles, but is not inside, the compiler package.
    let source = format!("{declaration}const result={value};");
    let allocator = oxc_allocator::Allocator::default();
    let program = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::ts())
        .parse()
        .program;
    let scoping = oxc_semantic::SemanticBuilder::new()
        .build(&program)
        .semantic
        .into_scoping();
    let option = ExtractOption {
        package: package.to_string(),
        ..ExtractOption::default()
    };
    let Some(Statement::VariableDeclaration(result)) = program.body.last() else {
        panic!("expected result binding")
    };
    let Some(symbol) = result.declarations[0]
        .id
        .get_binding_identifier()
        .and_then(|identifier| identifier.symbol_id.get())
    else {
        panic!("expected result symbol")
    };
    // When: source-only dependency analysis runs without a resolver.
    let names = symbols(&program, &scoping, &option);
    // Then: imported data and its local initializer chain remain risky.
    assert_eq!(names.get(&symbol), Some(&Naming::Risky));
    assert_eq!(stylesheet(&source, "test.css.ts", &option), Naming::Risky);
}

#[rstest]
#[case("@devup-ui/react", "@devup-ui/react", Naming::Own)]
#[case("@devup-ui/react", "@devup-ui/react/compat", Naming::Own)]
#[case("@devup-ui/react", "@devup-ui/react-values", Naming::Risky)]
#[case("@custom/ui", "@custom/ui", Naming::Own)]
#[case("@custom/ui", "@custom/ui/compat", Naming::Own)]
#[case("@custom/ui", "@custom/ui-values", Naming::Risky)]
fn stylesheet_preserves_package_boundary_policy(
    #[case] package: &str,
    #[case] module: &str,
    #[case] expected: Naming,
) {
    // Given: only the package boundary varies around a literal compiler call.
    let source = format!("import {{css}} from '{module}';const result=css({{color:'red'}});");
    // When: stylesheet eligibility is decided from source alone.
    let option = ExtractOption {
        package: package.to_string(),
        ..ExtractOption::default()
    };
    let naming = stylesheet(&source, "test.css.ts", &option);
    // Then: exact and slash-subpath package exemptions are unchanged.
    assert_eq!(naming, expected);
}

#[rstest]
#[case("@devup-ui/react", "@devup-ui/react", false)]
#[case("@devup-ui/react", "@devup-ui/react/compat", false)]
#[case("@devup-ui/react", "@devup-ui/react-values", true)]
#[case("@custom/ui", "@custom/ui", false)]
#[case("@custom/ui", "@custom/ui/compat", false)]
#[case("@custom/ui", "@custom/ui-values", true)]
fn require_preserves_package_boundary_policy(
    #[case] package: &str,
    #[case] module: &str,
    #[case] external: bool,
) {
    // Given: require uses the same boundary as static imports.
    let source = format!("const sdk=require('{module}');");
    let option = ExtractOption {
        package: package.to_string(),
        ..ExtractOption::default()
    };
    // When: source-only stylesheet eligibility visits the call.
    let naming = stylesheet(&source, "test.css.ts", &option);
    // Then: only package lookalikes become external.
    assert_eq!(naming, if external { Naming::Risky } else { Naming::Own });
}
