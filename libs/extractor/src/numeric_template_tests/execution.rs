use super::*;
use oxc_ast::ast::{ImportDeclarationSpecifier, Statement};
use oxc_codegen::Codegen;

#[rstest]
#[case::scalar(Surface::Box)]
#[case::styled(Surface::Styled)]
#[case::responsive(Surface::Responsive)]
#[serial]
fn effectful_template_executes_once_when_generated_element_renders(
    #[case] surface: Surface,
    #[values(
        "`0.${(trace.push('value'),5)}`",
        "`0.${`${(trace.push('value'),5)}`}`"
    )]
    expression: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: non-JSX factories let Boa execute the real generated JS renderer.
    reset_class_map();
    reset_file_map();
    let statement = match surface {
        Surface::Box => format!("return jsx(Box, {{ opacity: {expression} }});"),
        Surface::Responsive => format!("return jsx(Box, {{ opacity: [1,null,{expression}] }});"),
        Surface::Styled => {
            format!("const Card = styled.div({{ opacity: {expression} }}); return Card({{}});")
        }
        Surface::Css => unreachable!("css has no runtime owning element"),
    };
    let source = format!(
        "import {{ Box, styled }} from '@devup-ui/react';\nimport {{ jsx }} from 'react/jsx-runtime';\nfunction render(value, trace) {{ {statement} }}"
    );
    let output = extract(
        "numeric-template.js",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("runtime fixture extraction failed: {error}"));
    let arena = Allocator::default();
    let parsed = Parser::new(&arena, &output.code, SourceType::mjs()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{}", output.code);
    let mut program = parsed.program;
    let mut factories = Vec::new();
    for statement in &program.body {
        if let Statement::ImportDeclaration(import) = statement
            && import.source.value == "react/jsx-runtime"
        {
            for specifier in import.specifiers.iter().flatten() {
                match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(named) => {
                        assert_eq!(named.imported.to_string(), "jsx");
                        factories.push(format!(
                            "const {} = (target, props) => ({{target, props}});",
                            named.local.name
                        ));
                    }
                    other => panic!("unexpected JSX factory import: {other:?}"),
                }
            }
        }
    }
    program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    let factories = factories.join("\n");
    let script = format!(
        "{factories}{}\nconst trace = []; const result = render(5, trace); JSON.stringify([trace,Object.values(result.props.style || {{}})]);",
        Codegen::new().build(&program).code
    );
    // When: execute one render, not a reconstructed interpolation expression.
    let result = Context::default()
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("generated renderer failed: {error}\n{script}"));
    // Then: the mutation happens once and the complete value reaches the style variable.
    assert_eq!(
        result
            .as_string()
            .unwrap_or_else(|| panic!("renderer must return JSON text"))
            .to_std_string_escaped(),
        r#"[["value"],["0.5"]]"#
    );
}
