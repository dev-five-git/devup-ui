use super::*;
use boa_engine::{Context, Source};
use oxc_codegen::Codegen;

#[rstest]
#[case("styled(tag)({ color: 'red' })")]
#[case("styled(targets[name])({ color: 'red' })")]
#[case("styled(active ? Component : Other)({ color: 'red' })")]
#[case("styled(getThing().div)({ color: 'red' })")]
#[case("styled(getBase())({ color: 'red' })")]
#[serial]
fn bound_target_is_evaluated_once_when_component_renders(#[case] factory: &str) {
    // Given: runtime bases, including side effects and computed member reads.
    let source = format!(
        "{IMPORT}let reads = 0; const Component = 'article'; const Other = 'aside'; const targets = {{ get div() {{ reads++; return Component; }} }}; function getThing() {{ reads++; return {{ div: Component }}; }} function getBase() {{ reads++; return Component; }} function makeCard(tag, active, targets, name) {{ return {factory}; }} const Card = makeCard(Component, true, targets, 'div');"
    );
    let output = compile("test.js", &source, true);
    let arena = Allocator::default();
    let mut program = parse(&arena, "test.js", &output);
    let calls = renders(&program);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].target, Target::Component("DevupStyled".into()));
    let helper = &calls[0].helper;
    program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    let code = Codegen::new().build(&program).code;
    // When: the generated renderer executes twice after definition-time evaluation.
    let script = format!(
        "const {helper} = (target, props) => ({{target, props}}); {code}\nconst before = reads; const first = Card({{children: 'first'}}); const second = Card({{children: 'second'}}); JSON.stringify([before, reads, first.target, second.target, first.props.children, second.props.children]);"
    );
    let mut context = Context::default();
    let result = context
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("bound-target renderer evaluation failed: {error}"));
    // Then: renders keep the same target and never re-evaluate the runtime base.
    let expected_reads = match factory {
        "styled(tag)({ color: 'red' })"
        | "styled(active ? Component : Other)({ color: 'red' })" => 0,
        _ => 1,
    };
    assert_eq!(
        result
            .as_string()
            .unwrap_or_else(|| panic!("bound-target renderer JSON result must be a string"))
            .to_std_string_escaped(),
        format!("[{expected_reads},{expected_reads},\"article\",\"article\",\"first\",\"second\"]"),
    );
}

#[rstest]
#[case("styled('article', { color: runtimeColor })")]
#[case("styled('article').attrs({ role: 'button' })({ color: runtimeColor })")]
#[serial]
fn caller_props_win_when_dynamic_styles_are_rendered(#[case] factory: &str) {
    // Given: a dynamic CSS variable and caller-provided class, style and ordinary props.
    let source = format!("{IMPORT}const runtimeColor = getColor(); const Card = {factory};");
    let output = compile("test.js", &source, true);
    let arena = Allocator::default();
    let mut program = parse(&arena, "test.js", &output);
    let calls = renders(&program);
    assert_eq!(calls.len(), 1);
    let helper = &calls[0].helper;
    program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    let code = Codegen::new().build(&program).code;
    // When: the generated call renders with caller overrides.
    let script = format!(
        "const {helper} = (target, props) => ({{target, props}}); function getColor() {{ return 'red'; }} {code}\nconst first = Card({{className: 'caller', children: 'child', onClick: 7, ref: 8, as: 'section'}}); const variable = Object.keys(first.props.style)[0]; const second = Card({{style: {{[variable]: 'blue'}}, role: 'link'}}); JSON.stringify([first.target, first.props.className.includes('caller'), first.props.style[variable], first.props.children, first.props.onClick, first.props.ref, first.props.as, second.props.style[variable]]);"
    );
    let mut context = Context::default();
    let result = context
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("caller-props renderer evaluation failed: {error}"));
    // Then: generated variables survive, caller style wins and pass-through props remain.
    assert_eq!(
        result
            .as_string()
            .unwrap_or_else(|| panic!("caller-props renderer JSON result must be a string"))
            .to_std_string_escaped(),
        r#"["article",true,"red","child",7,8,"section","blue"]"#,
    );
}
