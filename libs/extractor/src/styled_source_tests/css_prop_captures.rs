use super::*;
use boa_engine::{Context, Source};
use oxc_codegen::Codegen;

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn authored_captures_survive_when_css_prop_renders_fixed_prepared_styled(
    #[case] css_prop: bool,
    #[values(false, true)] selected: bool,
) {
    // Given: retained controller/leaf effects whose style is overwritten by a literal.
    let css = if css_prop { "css:{padding:4}," } else { "" };
    let source = format!(
        "import {{styled}} from '@devup-ui/react';import {{jsx}} from '@emotion/react';const trace=[];const selected={selected};const Card=styled.article({{...((trace.push('controller'),selected)?{{color:(trace.push('leaf'),'red')}}:{{}}),color:'green'}});const App=ref=>jsx(Card,{{{css}ref,children:'child'}});"
    );
    reset_class_map();
    reset_file_map();
    let output = extract(
        "test.ts",
        &source,
        ExtractOption {
            single_css: true,
            import_aliases: std::collections::HashMap::from([(
                "@emotion/react".to_string(),
                crate::ImportAlias::NamedToNamed,
            )]),
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("capture extraction failed: {error}"));
    let arena = Allocator::default();
    let mut program = parse(&arena, "test.ts", &output.code);
    let calls = renders(&program);
    let helper = &calls[0].helper;
    program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    let code = Codegen::new().build(&program).code;

    // When: emitted CSS-prop calls render twice through the real generated callback.
    let script = format!(
        "const __devupForwardRef=render=>({{render}});function jsx(target,props){{return typeof target==='string'?{{target,props}}:target.render(props,props.ref);}}const {helper}=jsx;{code}\nconst before=trace.slice();const first=App(7);const second=App(8);JSON.stringify([before,first.target,second.target,first.props.ref,second.props.ref,first.props.children,second.props.children,trace]);"
    );
    let mut context = Context::default();
    let result = context
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("CSS-prop capture execution failed: {error}\n{code}"));
    let result = result
        .as_string()
        .unwrap_or_else(|| panic!("capture observation must be JSON"))
        .to_std_string_escaped();

    // Then: unused style captures still execute once per render in authored order.
    let trace = if selected {
        vec!["controller", "leaf", "controller", "leaf"]
    } else {
        vec!["controller", "controller"]
    };
    let expected = serde_json::json!([[], "article", "article", 7, 8, "child", "child", trace]);
    assert_eq!(result, expected.to_string(), "css_prop={css_prop}\n{code}");
}
