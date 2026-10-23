#[rstest]
#[case::default(("function f(out=sx.API(flag() && s.base)){}", "parameter default"))]
#[case::destructured(("function f({out=sx.API(flag() && s.base)}={}){}", "parameter default"))]
#[case::arrow_default(("const f=(out=sx.API(flag() && s.base))=>out;", "parameter default"))]
#[case::instance_field(("class C { out=sx.API(flag() && s.base); }", "class field initializer"))]
#[case::static_field(("class C { static out=sx.API(flag() && s.base); }", "class field initializer"))]
#[case::private_field(("class C { #out=sx.API(flag() && s.base); }", "class field initializer"))]
#[case::accessor_field(("class C { accessor out=sx.API(flag() && s.base); }", "class field initializer"))]
#[serial]
fn stylex_capture_is_located_when_storage_has_no_execution_local_statement(
    #[case] sample: (&str, &str),
    #[values("props", "attrs")] api: &str,
) {
    // Given: evaluating an expression-only initializer needs local condition storage.
    reset_class_map();
    reset_file_map();
    let body = sample.0.replace("API", api);
    let source = format!("import * as sx from '@stylexjs/stylex';\nconst s=sx.create({{base:{{color:'red',backgroundColor:'blue'}}}});\n{body}");
    // When
    let error = extract("capture-context.js", &source, ExtractOption::default()).unwrap_err().to_string();
    // Then: the diagnostic is at the condition, not at the call or enclosing declaration.
    let column = body.find("flag()").unwrap() + 1;
    assert!(error.starts_with(&format!("capture-context.js:3:{column}:")), "{error}");
    assert!(error.contains(sample.1) && error.contains("move the value into a variable"), "{error}");
    eprintln!("CONTEXT {error}");
}

#[rstest]
#[case::concise(("const render=()=>jsx('div',sx.API((trace.push('once'),true)&&s.base));", "render()", "[\"once\"]"))]
#[case::method(("class C { render(){return jsx('div',sx.API((trace.push('once'),true)&&s.base));}} function render(){return new C().render();}", "render()", "[\"once\"]"))]
#[case::getter(("const c={get out(){return sx.API((trace.push('once'),true)&&s.base);}}; function render(){return jsx('div',c.out);}", "render()", "[\"once\"]"))]
#[case::setter(("const c={set out(v){this.value=sx.API((trace.push(v),true)&&s.base);}}; function render(){c.out='once';return jsx('div',c.value);}", "render()", "[\"once\"]"))]
#[case::recursive(("function choose(n){const out=sx.API((trace.push(n),true)&&s.base,(n>0?(choose(n-1),false):false)&&s.later);return out;} function render(){return jsx('div',choose(1));}", "render()", "[1,0]"))]
#[case::deferred_tdz(("const value=false;function render(initial){const read=()=>{trace.push('read');return value;};try{sx.API(read()&&s.base,(trace.push('bad'),true)&&s.later);}catch(e){trace.push(e.name);}let value=initial;return jsx('div',sx.API(read()&&s.base));}", "render(true)", "[\"read\",\"ReferenceError\",\"read\"]"))]
#[case::shadow_collision(("const _s0=7; function render(){let _s1=9;{let _s2=11;trace.push(_s0,_s1,_s2);return jsx('div',sx.API((trace.push('once'),true)&&s.base));}}", "render()", "[7,9,11,\"once\"]"))]
#[case::closure_loop(("function render(){const fs=[];for(let i=0;i<2;i++){const out=sx.API((trace.push(i),true)&&s.base);fs.push(()=>out);}return jsx('div',fs[0]());}", "render()", "[0,1]"))]
#[case::short_circuit(("function render(){false&&sx.API((trace.push('bad'),true)&&s.base);return jsx('div',sx.API((trace.push('once'),true)&&s.base));}", "render()", "[\"once\"]"))]
#[case::generator(("function* gen(n){return sx.API((trace.push(n),true)&&s.base,(yield n)&&s.later);}function render(){const a=gen('a'),b=gen('b');a.next();b.next();const out=a.next(false).value;b.next(true);return jsx('div',out);}", "render()", "[\"a\",\"b\"]"))]
#[case::field_arrow(("class C { render=()=>jsx('div',sx.API((trace.push('once'),true)&&s.base));}function render(){return new C().render();}", "render()", "[\"once\"]"))]
#[case::default_arrow(("function f(render=()=>jsx('div',sx.API((trace.push('once'),true)&&s.base))){return render();}", "f()", "[\"once\"]"))]
#[case::reentrant_getter(("let entered=false;const state={get on(){trace.push(entered);if(!entered){entered=true;render();entered=false;}return !entered;}};function render(){return jsx('div',sx.API(state.on&&s.base));}", "render()", "[false,true]"))]
#[case::static_block(("class C {static {this.out=sx.API(true&&s.base);}} function render(){return jsx('div',C.out);}", "render()", "[]"))]
#[case::unbound_reference(("function render(){const out=sx.API(true&&s.base);trace.push(typeof _s0);return jsx('div',out);}", "render()", "[\"undefined\"]"))]
#[serial]
fn stylex_capture_preserves_execution_scope_when_bodies_reenter_or_suspend(
    #[case] sample: (&str, &str, &str),
    #[values("props", "attrs")] api: &str,
) {
    // Given: independent executions must retain their own earlier condition results.
    reset_class_map();
    reset_file_map();
    let body = sample.0.replace("API", api);
    let body = if api == "attrs" { body.replace("jsx('div',", "adapt('div',") } else { body };
    let source = format!("import * as sx from '@stylexjs/stylex';import {{jsx}} from 'react/jsx-runtime';const s=sx.create({{base:{{color:'red',backgroundColor:'blue'}},later:{{color:'green'}}}});function adapt(tag,p){{return jsx(tag,{{className:p.class}});}}{body}");
    // When
    let output = extract("capture-body.js", &source, ExtractOption::default()).unwrap();
    assert_eq!(callable_count(&output.code), callable_count(&source));
    let observation = render_observation(&output, sample.1, Some("capture-body.js"));
    let (trace, mut values): (serde_json::Value, Vec<String>) = serde_json::from_str(&observation).unwrap();
    values.sort();
    // Then
    assert_eq!(trace, serde_json::from_str::<serde_json::Value>(sample.2).unwrap(), "{}", output.code);
    assert_eq!(values, ["blue", "red"], "{}", output.code);
    eprintln!("BODY {api} {} bytes={}\n{}", sample.1, output.code.len(), output.code);
}

#[rstest]
#[serial]
fn stylex_immediate_tdz_is_located_before_lowering(#[values("props","attrs")] api: &str) {
    // Given: the original immediate-TDZ input, including its later initialized read.
    reset_class_map();
    reset_file_map();
    let body="function render(){try{sx.API(value&&s.base);}catch(e){trace.push(e.name);}let value=true;return jsx('div',sx.API(value&&s.base));}".replace("API",api);
    let body=if api=="attrs" {body.replace("jsx('div',","adapt('div',")}else{body};
    let source=format!("import * as sx from '@stylexjs/stylex';import {{jsx}} from 'react/jsx-runtime';const s=sx.create({{base:{{color:'red',backgroundColor:'blue'}},later:{{color:'green'}}}});function adapt(tag,p){{return jsx(tag,{{className:p.class}});}}{body}");
    // When: the known read is classified by the existing initialization boundary.
    let error=extract("capture-body.js",&source,ExtractOption::default()).unwrap_err().to_string();
    // Then: both APIs point to the first authored value, with the original ReferenceError cause.
    assert!(error.starts_with(&format!("capture-body.js:1:{}:",source.find("value&&").unwrap()+1)),"{error}");
    assert!(error.contains("read before its lexical declaration is initialized")&&error.contains("ReferenceError"),"{error}");
    eprintln!("IMMEDIATE {api} {error}");
}

#[test]
#[serial]
fn stylex_capture_stays_linear_when_many_conditions_overlap_both_sides() {
    // Given: every condition overwrites the same key on both sides (old trees must be dropped).
    let mut sizes = Vec::new();
    for count in [8, 16, 32, 64] {
        reset_class_map();
        reset_file_map();
        let arguments = (0..count).map(|i| format!("(trace.push({i}),{i}%2===0)?s.base:s.later")).collect::<Vec<_>>().join(",");
        let source = format!("import * as sx from '@stylexjs/stylex';import {{jsx}} from 'react/jsx-runtime';const s=sx.create({{base:{{color:'red'}},later:{{color:'green'}}}});function render(){{return jsx('div',sx.props({arguments}));}}");
        // When
        let output = extract("capture-growth.js", &source, ExtractOption::default()).unwrap();
        assert_eq!(callable_count(&output.code), callable_count(&source));
        let observation = render_observation(&output, "render()", Some("capture-growth.js"));
        let (trace, values): (Vec<usize>, Vec<String>) = serde_json::from_str(&observation).unwrap();
        // Then: every argument runs, the final odd branch wins, output is deterministic and bounded.
        assert_eq!(trace, (0..count).collect::<Vec<_>>());
        assert_eq!(values, ["green"]);
        assert!(output.code.len() < 200 * count + 1000, "{}", output.code.len());
        reset_class_map();
        reset_file_map();
        assert_eq!(output.code, extract("capture-growth.js", &source, ExtractOption::default()).unwrap().code);
        sizes.push((count, output.code.len()));
    }
    eprintln!("GROWTH {sizes:?}");
}

fn callable_count(source: &str) -> usize {
    use oxc_ast_visit::{Visit, walk};
    struct Count(usize);
    impl<'a> Visit<'a> for Count {
        fn visit_function(&mut self, it: &oxc_ast::ast::Function<'a>, flags: oxc_syntax::scope::ScopeFlags) {
            self.0 += 1;
            walk::walk_function(self, it, flags);
        }
        fn visit_arrow_function_expression(&mut self, it: &oxc_ast::ast::ArrowFunctionExpression<'a>) {
            self.0 += 1;
            walk::walk_arrow_function_expression(self, it);
        }
    }
    let arena = Allocator::default();
    let parsed = Parser::new(&arena, source, SourceType::mjs()).parse();
    assert!(parsed.diagnostics.is_empty(), "{source}");
    let mut count = Count(0);
    count.visit_program(&parsed.program);
    count.0
}

#[rstest]
#[serial]
fn stylex_capture_survives_await_when_activations_interleave(#[values("props", "attrs")] api: &str) {
    // Given: the first activation suspends after capturing true, while another captures false.
    reset_class_map();
    reset_file_map();
    let attribute = if api == "props" { "className" } else { "class" };
    let source = format!("import * as sx from '@stylexjs/stylex';const s=sx.create({{base:{{color:'red',backgroundColor:'blue'}},later:{{color:'green'}}}});async function render(on,wait){{return sx.{api}((trace.push(on),on)&&s.base,(await wait)&&s.later).{attribute};}}const reference=sx.{api}(s.base).{attribute};");
    // When: the emitted native JS runs its actual promise jobs.
    let output = extract("capture-await.js", &source, ExtractOption::default()).unwrap();
    assert_eq!(callable_count(&output.code), callable_count(&source));
    let arena = Allocator::default();
    let mut program = Parser::new(&arena, &output.code, SourceType::mjs()).parse().program;
    program.body.retain(|s| !matches!(s, oxc_ast::ast::Statement::ImportDeclaration(_)));
    let script = format!("{};const trace=[];let release,result;const pending=new Promise(resolve=>release=resolve);const first=render(true,pending);render(false,false).then(()=>release(false));first.then(value=>result=JSON.stringify([trace,value.split(/\\s+/).sort().join(' ')===reference.split(/\\s+/).sort().join(' ')]));", oxc_codegen::Codegen::new().build(&program).code);
    let mut context = boa_engine::Context::default();
    context.eval(boa_engine::Source::from_bytes(&script)).unwrap();
    context.run_jobs().unwrap();
    let result = context.eval(boa_engine::Source::from_bytes("result")).unwrap().as_string().unwrap().to_std_string_escaped();
    // Then
    assert_eq!(result, "[[true,false],true]", "{}", output.code);
    eprintln!("AWAIT {api} {result}\n{}", output.code);
}

#[test]
#[serial]
fn stylex_capture_accounts_for_output_when_multiple_keys_share_one_condition() {
    // Given: the independently executed public-WASM counterexample from the durable report.
    reset_class_map();
    reset_file_map();
    let source = "import * as sx from '@stylexjs/stylex';const s=sx.create({base:{color:'red',backgroundColor:'blue'}});export function probe(){let n=0;const out=sx.props(++n===1 && s.base);return [n,out.className];}";
    // When
    let output = extract("capture-account.js", source, ExtractOption {single_css:true,..ExtractOption::default()}).unwrap();
    // Then: no callable is added; byte and implementation-line accounting is durable test output.
    assert_eq!(callable_count(&output.code), callable_count(source));
    eprintln!("ACCOUNT bytes={}\n{}", output.code.len(), output.code);
    for (name, text) in [("visit.rs", include_str!("visit.rs")), ("visit/stylex_capture.rs", include_str!("visit/stylex_capture.rs")), ("stylex_capture_tests.rs", include_str!("stylex_capture_tests.rs")), ("stylex_no_storage_tests.rs", include_str!("stylex_no_storage_tests.rs")), ("stylex_exact_key_tests.rs", include_str!("stylex_exact_key_tests.rs"))] {
        let pure = text.lines().filter(|line| { let line=line.trim(); !line.is_empty() && !line.starts_with("//") && !line.starts_with('#') }).count();
        eprintln!("LINES {name} pure={pure} total={}", text.lines().count());
    }
}

#[rstest]
#[case::one_key("s.one, (trace.push('once'), true)&&s.one")]
#[case::empty("(trace.push('once'), true)?null:null")]
#[case::equal("(trace.push('once'), true)?s.base:s.base")]
#[case::overridden("(trace.push('once'), true)&&s.base, s.base")]
#[serial]
fn stylex_capture_accepts_defaults_when_no_storage_is_needed(
    #[case] arguments: &str,
    #[values(false, true)] field: bool,
) {
    // Given: a default can evaluate directly once or discard a condition before constant classes.
    reset_class_map();
    reset_file_map();
    let body = if field {
        format!("class C {{out=sx.props({arguments});}}function render(){{return jsx('div',new C().out);}}")
    } else {
        format!("function render(out=sx.props({arguments})){{return jsx('div',out);}}")
    };
    let source=format!("import * as sx from '@stylexjs/stylex';import {{jsx}} from 'react/jsx-runtime';const s=sx.create({{one:{{color:'red'}},base:{{color:'red',backgroundColor:'blue'}}}});{body}");
    // When
    let output=extract("capture-default-control.js", &source, ExtractOption::default()).unwrap();
    let observation=render_observation(&output,"render()",Some("capture-default-control.js"));
    let (trace,_): (Vec<String>,Vec<String>)=serde_json::from_str(&observation).unwrap();
    // Then: no error and the unpruned authored effect runs once despite equal rendered outcomes.
    assert_eq!(trace,["once"],"{}",output.code);
    assert_eq!(callable_count(&output.code), callable_count(&source));
}
