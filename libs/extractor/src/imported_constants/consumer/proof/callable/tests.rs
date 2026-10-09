use rstest::rstest;

use super::super::tests::input_of;

#[rstest]
#[case("function read(n){const local=n+value;return local}", true)]
#[case("const read=n=>{const local=n+value;return local}", true)]
#[case("const read=function(n){const local=n+value;return local}", true)]
#[case("function read(n=value){return n}", true)]
#[case("function read(n=window.name){return n}", false)]
#[case("function read(){return window.name}", false)]
#[case("declare function read(n:number):string", false)]
fn callable_when_parameters_locals_and_free_captures_are_classified_is_exact(
    #[case] declaration: &str,
    #[case] expected: bool,
) {
    // Given
    let source = format!("import {{value}} from './data';{declaration};read;");
    // When
    let actual = input_of(&source, "read");
    // Then
    assert_eq!(actual, expected, "{source}");
}

#[test]
fn callable_when_enclosing_runtime_parameter_shadows_module_input_is_open() {
    // Given
    let source = "import {value} from './data';function outer(value){function read(){return value}return read;}";
    super::super::super::tests::parsed(source, |program, semantic| {
        let style = super::super::StyleSymbols::from_program(
            program,
            semantic.scoping(),
            &crate::ExtractOption::default(),
        );
        let kind = semantic
            .nodes()
            .iter()
            .find_map(|node| match node.kind() {
                oxc_ast::AstKind::Function(function)
                    if function.id.as_ref().is_some_and(|id| id.name == "read") =>
                {
                    Some(node.kind())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing parsed read callable"));
        let known = |identifier: &oxc_ast::ast::IdentifierReference<'_>| {
            super::super::input(identifier, semantic, &style)
        };
        let mut proof = super::Closed {
            style: &style,
            known: &known,
            exact: true,
            input: false,
        };
        // When
        let actual = super::closed(kind, &mut proof, semantic);
        // Then
        assert!(!actual);
    });
}
