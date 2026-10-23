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
    // When: execute one render, not a reconstructed interpolation expression.
    let result = observe(&output, "render(5, trace)");
    // Then: the mutation happens once and the complete value reaches the style variable.
    assert_eq!(result, r#"[["value"],["0.5"]]"#);
}

pub(super) fn observe(output: &crate::ExtractOutput, invocation: &str) -> String {
    let arena = Allocator::default();
    let parsed = Parser::new(&arena, &output.code, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{}", output.code);
    let mut program = parsed.program;
    let scoping = oxc_semantic::SemanticBuilder::new()
        .build(&program)
        .semantic
        .into_scoping();
    let mut options = oxc_transformer::TransformOptions::default();
    options.jsx.runtime = oxc_transformer::JsxRuntime::Classic;
    options.jsx.pragma = Some("__numericJsx".to_string());
    let transformed =
        oxc_transformer::Transformer::new(&arena, std::path::Path::new("numeric.tsx"), &options)
            .build_with_scoping(scoping, &mut program);
    assert_eq!(transformed.diagnostics.len(), 0);
    let mut factories = vec!["const __numericJsx=(target,props)=>({target,props});".to_string()];
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
        if let Statement::ImportDeclaration(import) = statement
            && import.source.value == "react"
        {
            for specifier in import.specifiers.iter().flatten() {
                if let ImportDeclarationSpecifier::ImportSpecifier(named) = specifier
                    && named.imported.name() == "forwardRef"
                {
                    factories.push(format!(
                        "const {} = (render) => (props) => render(props, null);",
                        named.local.name
                    ));
                }
            }
        }
    }
    program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    let factories = factories.join("\n");
    let script = format!(
        "{factories}{}\nconst trace = []; const result = {invocation}; JSON.stringify([trace,Object.values(result.props.style || {{}})]);",
        Codegen::new().build(&program).code
    );
    let result = Context::default()
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("generated renderer failed: {error}\n{script}"));
    result
        .as_string()
        .unwrap_or_else(|| panic!("renderer must return JSON text"))
        .to_std_string_escaped()
}

#[rstest]
#[serial]
fn coercion_and_throw_order_survive_when_complete_template_frames_execute(
    #[values(Surface::Box, Surface::Styled, Surface::Responsive)] surface: Surface,
    #[values(false, true)] throws: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: a coercible interpolation and a later effect that a throw must skip.
    reset_class_map();
    reset_file_map();
    let expression = "`0.${(trace.push('value'),value)}${(trace.push('later'),'')}`";
    let statement = match surface {
        Surface::Box => format!("return jsx(Box,{{opacity:{expression}}});"),
        Surface::Responsive => format!("return jsx(Box,{{opacity:[1,null,{expression}]}});"),
        Surface::Styled => {
            format!("const Card=styled.div({{opacity:{expression}}});return Card({{}});")
        }
        Surface::Css => unreachable!(),
    };
    let source = format!(
        "import {{Box,styled}} from '@devup-ui/react';import {{jsx}} from 'react/jsx-runtime';function render(value,trace){{{statement}}}"
    );
    let value = format!(
        "const value={{[Symbol.toPrimitive](hint){{trace.push(hint);if({throws})throw new Error('stop');return 5;}}}};"
    );
    let authored = format!(
        "const trace=[];{value}let values=[];try{{values=[{expression}];}}catch(e){{trace.push(e.message);}}JSON.stringify([trace,values]);"
    );
    let expected = Context::default()
        .eval(Source::from_bytes(&authored))
        .unwrap_or_else(|error| panic!("authored coercion failed: {error}"))
        .as_string()
        .unwrap_or_else(|| panic!("authored coercion must return JSON"))
        .to_std_string_escaped();
    // When: run generated declarations and their owning calls, including their catch boundary.
    let output = extract(
        "coercion.js",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("coercion extraction failed: {error}"));
    let invocation = format!(
        "(()=>{{{value}try{{return render(value,trace);}}catch(e){{trace.push(e.message);return {{props:{{}}}};}}}})()"
    );
    // Then: native interpolation coercion runs once, and only successful coercion reaches later.
    assert_eq!(observe(&output, &invocation), expected, "{}", output.code);
}
