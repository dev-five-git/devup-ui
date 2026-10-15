use super::*;
use boa_engine::{Context, Source};
use oxc_codegen::Codegen;

#[test]
#[serial]
fn eager_computed_target_and_callback_ref_survive_when_captures_render_twice() {
    // Given: a computed getter target and per-render style captures with distinct refs.
    let source = format!(
        "{IMPORT}const trace=[];let reads=0;let tone='olive';const on=true;const targets={{get div(){{reads++;trace.push('target');return 'article';}}}};const name='div';const Card=styled(targets[name]).attrs({{id:'card'}})({{color:'red',...((trace.push('style'),on)?{{color:(trace.push('leaf'),tone)}}:{{}})}});"
    );
    let output = compile("test.ts", &source, true);
    let arena = Allocator::default();
    let mut program = parse(&arena, "test.ts", &output);
    let calls = renders(&program);
    assert_eq!(calls.len(), 1);
    let helper = &calls[0].helper;
    program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    let code = Codegen::new().build(&program).code;
    // When: two callbacks execute with a changed dynamic value and callback refs 7/8.
    let script = format!(
        "const __devupForwardRef=render=>({{render}});const {helper}=(target,props)=>({{target,props}});{code}\nconst before=reads;const first=Card.render({{ref:99,children:'first'}},7);tone='teal';const second=Card.render({{ref:99,children:'second'}},8);JSON.stringify([before,reads,first.target,second.target,first.props.ref,second.props.ref,first.props.children,second.props.children,Object.values(first.props.style),Object.values(second.props.style),trace]);"
    );
    let mut context = Context::default();
    let result = context.eval(Source::from_bytes(&script));
    assert!(result.is_ok(), "{result:?}");
    // Then: the target reads once eagerly, refs forward, and captures stay per-render.
    assert_eq!(result.ok().and_then(|value| value.as_string().map(|value| value.to_std_string_escaped())), Some(r#"[1,1,"article","article",7,8,"first","second",["olive"],["teal"],["target","style","leaf","style","leaf"]]"#.to_string()));
}

#[rstest]
#[case("cts")]
#[case("cjs")]
#[serial]
fn callback_ref_reaches_target_when_commonjs_renderer_executes(#[case] extension: &str) {
    // Given: an empty-style renderer, so both new React bindings must be executable CommonJS.
    let filename = format!("test.{extension}");
    let output = compile(
        &filename,
        &format!("{IMPORT}const Card=styled('article', {{}});"),
        true,
    );
    let arena = Allocator::default();
    let program = parse(&arena, &filename, &output);
    assert!(
        program
            .body
            .iter()
            .all(|statement| !matches!(statement, Statement::ImportDeclaration(_))),
        "{output}"
    );
    // When: the full emitted module runs with a callback ref distinct from the props ref.
    let script = format!(
        "function require(name){{if(name==='react')return {{forwardRef:render=>({{render}})}};if(name==='react/jsx-runtime')return {{jsx:(target,props)=>({{target,props}})}};throw new Error(name);}}{output}\nconst element=Card.render({{ref:99,children:'child',as:'section',forwardedAs:'nav'}},7);JSON.stringify([element.target,element.props.ref,element.props.children,element.props.as]);"
    );
    let mut context = Context::default();
    let result = context.eval(Source::from_bytes(&script));
    assert!(result.is_ok(), "{result:?}");
    // Then: dialect, callback ref, children and independent target contracts all survive.
    assert_eq!(
        result
            .ok()
            .and_then(|value| value.as_string().map(|value| value.to_std_string_escaped())),
        Some(r#"["section",7,"child","nav"]"#.to_string())
    );
}

#[rstest]
#[case("Link")]
#[case("motion.div")]
#[serial]
fn actual_target_and_captures_survive_when_typescript_retargets_then_extends(#[case] target: &str) {
    // Given: a prepared base, retained attrs, a retarget and a further extension.
    let source = format!(
        "{IMPORT}const trace=[];const Link=props=>props;const Override=props=>props;const motion={{div:Link}};const on=true;const Base=styled.div.attrs({{id:'retargeted'}})({{color:'red',...((trace.push('base'),on)?{{color:'blue'}}:{{}})}});const Mid=Base.withComponent({target});const Card=styled(Mid)({{p:1}});"
    );
    let output = compile("test.ts", &source, true);
    let arena = Allocator::default();
    let mut program = parse(&arena, "test.ts", &output);
    let calls = renders(&program);
    assert_eq!(calls.len(), 3);
    let helper = &calls[0].helper;
    program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    let code = Codegen::new().build(&program).code;
    // When: the emitted extension renders its default and an actual component override.
    let script = format!(
        "const __devupForwardRef=render=>({{render}});const {helper}=(target,props)=>({{target,props}});{code}\nconst first=Card.render({{className:'caller',forwardedAs:'nav'}},null);const second=Card.render({{as:Override,forwardedAs:'aside'}},null);JSON.stringify([first.target===Link,second.target===Override,first.props.as,second.props.as,first.props.id,second.props.id,first.props.className.includes('caller'),trace]);"
    );
    let mut context = Context::default();
    let result = context.eval(Source::from_bytes(&script));
    assert!(result.is_ok(), "{result:?}");
    // Then: identity, forwarded props, attrs and single ordered captures survive retargeting.
    assert_eq!(
        result.ok().and_then(|result| result
            .as_string()
            .map(|value| value.to_std_string_escaped())),
        Some(
            r#"[true,true,"nav","aside","retargeted","retargeted",true,["base","base"]]"#
                .to_string()
        )
    );
}

#[rstest]
#[case("styled(Component)({ color: 'red' })")]
#[case("styled(motion.div)({ color: 'red' })")]
#[case("styled(tag)({ color: 'red' })")]
#[serial]
fn inherited_target_is_preserved_when_typescript_component_renders(#[case] factory: &str) {
    // Given: two extensions of a component, member or runtime-bound base in non-JSX TS.
    let source = format!(
        "{IMPORT}const Component = (props) => props; const motion = {{ div: Component }}; const tag = Component; const Base = {factory}; const Middle = styled(Base)({{ background: 'blue' }}); const Card = styled(Middle)`padding: 4px;`;"
    );
    let output = compile("test.ts", &source, true);
    let arena = Allocator::default();
    let mut program = parse(&arena, "test.ts", &output);
    let calls = renders(&program);
    assert_eq!(calls.len(), 3);
    let helper = &calls[0].helper;
    program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    let code = Codegen::new().build(&program).code;
    // When: the emitted extension renders through the actual factory.
    let script = format!(
        "const __devupForwardRef=render=>({{render}});const {helper} = (target, props) => ({{target, props}}); {code}\nCard.render({{children: 'child'}},null).target === Component;"
    );
    let mut context = Context::default();
    let result = context
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("inherited-target renderer evaluation failed: {error}"));
    // Then: the extension renders the original component, never its string name.
    assert_eq!(result.as_boolean(), Some(true));
}

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
        "const __devupForwardRef=render=>({{render}});const {helper} = (target, props) => ({{target, props}}); {code}\nconst before = reads; const first = Card.render({{children: 'first'}},null); const second = Card.render({{children: 'second'}},null); JSON.stringify([before, reads, first.target, second.target, first.props.children, second.props.children]);"
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
        "const __devupForwardRef=render=>({{render}});const {helper} = (target, props) => ({{target, props}}); function getColor() {{ return 'red'; }} {code}\nconst first = Card.render({{className: 'caller', children: 'child', onClick: 7, ref: 99, as: 'section', forwardedAs: 'nav'}},8); const variable = Object.keys(first.props.style)[0]; const second = Card.render({{style: {{[variable]: 'blue'}}, role: 'link'}},null); JSON.stringify([first.target, first.props.className.includes('caller'), first.props.style[variable], first.props.children, first.props.onClick, first.props.ref, first.props.as, second.props.style[variable]]);"
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
        r#"["section",true,"red","child",7,8,"nav","blue"]"#,
    );
}
