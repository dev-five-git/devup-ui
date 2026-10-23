#[rstest]
#[case::direct(("function probe(){try{CAPTURE;return 'accepted';}catch(error){return error.name;}}", "eval('_s0')"))]
#[case::dynamic(("function probe(code){try{CAPTURE;return 'accepted';}catch(error){return error.name;}}", "eval(code)"))]
#[case::parenthesized(("function probe(code){try{CAPTURE;return 'accepted';}catch(error){return error.name;}}", "(eval)(code)"))]
#[case::closure(("function probe(code){const read=()=>eval(code);CAPTURE;return read();}", "true"))]
#[case::nested_closure(("function probe(code){CAPTURE;{const read=()=>()=>eval(code);return read()();}}", "true"))]
#[case::before(("function probe(code){eval(code);return CAPTURE;}", "true"))]
#[case::after(("function probe(code){CAPTURE;return eval(code);}", "true"))]
#[case::concise(("const probe=code=>(CAPTURE,eval(code));", "true"))]
#[case::switch_sibling(("function probe(code){switch(1){case 0:return eval(code);case 1:return CAPTURE;}}", "true"))]
#[case::switch_test(("function probe(code){switch(1){case eval(code):return CAPTURE;}}", "true"))]
#[case::closure_default(("function probe(code){function read(value=eval(code)){return value;}CAPTURE;return read();}", "true"))]
#[case::method_closure(("function probe(code){const read={get value(){return eval(code);}};CAPTURE;return read.value;}", "true"))]
#[case::module(("CAPTURE;function probe(code){return eval(code);}", "true"))]
#[case::loop_header(("function probe(code){const read=()=>eval(code);for(let i=0;i<1;i++){break;}for(let out=CAPTURE;false;){}return read();}", "true"))]
#[serial]
fn stylex_eval_guard_is_located_when_capture_bindings_are_visible(
    #[case] sample: (&str, &str),
    #[values("props", "attrs")] api: &str,
) {
    // Given: only the authored condition needs storage; direct eval may inspect its frame.
    reset_class_map();
    reset_file_map();
    let body = sample.0.replace("CAPTURE", &format!("sx.{api}({} && s.base)", sample.1));
    let source = format!("import * as sx from '@stylexjs/stylex';\nconst s=sx.create({{base:{{color:'red',backgroundColor:'blue'}}}});\n{body}");
    // When
    let result = extract("capture-eval.js", &source, ExtractOption::default());
    // Then: rejection points to the condition even when eval is in another nested closure/case.
    let error = match result {
        Err(error) => error.to_string(),
        Ok(output) => panic!("capture exposed to direct eval: observed={}\n{}", observe_eval(&output.code, "probe(String.fromCharCode(95,115,48))"), output.code),
    };
    let column = body.find(&format!("sx.{api}(")).unwrap() + format!("sx.{api}(").len() + 1;
    assert!(error.starts_with(&format!("capture-eval.js:3:{column}:")), "{error}");
    assert!(error.contains("visible to direct eval") && error.contains("move the value into a variable"), "{error}");
    eprintln!("EVAL ERROR {error}");
}

#[rstest]
#[case::sequence(("", "(0,eval)(code)", "ReferenceError"))]
#[case::optional(("", "eval?.(code)", "ReferenceError"))]
#[case::member(("", "globalThis.eval(code)", "ReferenceError"))]
#[case::computed_member(("", "globalThis['eval'](code)", "ReferenceError"))]
#[case::alias(("const indirect=eval;", "indirect(code)", "ReferenceError"))]
#[case::sibling(("function sibling(code){return eval(code);}", "sibling(code)", "ReferenceError"))]
#[case::outer_closure(("const read=code=>eval(code);", "read(code)", "ReferenceError"))]
#[serial]
fn stylex_eval_guard_preserves_indirect_and_sibling_calls_when_they_cannot_see_storage(
    #[case] sample: (&str, &str, &str),
    #[values("props", "attrs")] api: &str,
) {
    // Given: runtime lookup of an unknown name cannot see probe's injected local binding.
    reset_class_map();
    reset_file_map();
    let source = format!("import * as sx from '@stylexjs/stylex';const s=sx.create({{base:{{color:'red',backgroundColor:'blue'}}}});{}function probe(code){{try{{sx.{api}({} && s.base);return 'accepted';}}catch(error){{return error.name;}}}}", sample.0, sample.1);
    // When
    let output = extract("capture-eval-control.js", &source, ExtractOption::default()).unwrap();
    let observed = observe_eval(&output.code, "probe(String.fromCharCode(95,115,48))");
    // Then: capture is accepted, no callable was added, and the original ReferenceError remains.
    assert_eq!(observed, sample.2, "{}", output.code);
    assert_eq!(callable_count(&output.code), callable_count(&source));
}

#[rstest]
#[case::outer_block("function probe(code){const read=()=>eval(code);{sx.API(true && s.base);try{return read();}catch(error){return error.name;}}}")]
#[case::outer_function("function probe(code){function read(){return eval(code);}{sx.API(true && s.base);try{return read();}catch(error){return error.name;}}}")]
#[case::sibling_block("function probe(code){{sx.API(true && s.base);}{try{return eval(code);}catch(error){return error.name;}}}")]
#[case::switch_discriminant("function probe(code){try{switch(eval(code)){case 0:sx.API(true && s.base);}}catch(error){return error.name;}}")]
#[case::parameter_eval("function probe(code,value=eval(code)){sx.API(true && s.base);return value;}")]
#[case::parameter_closure("function probe(code,read=()=>eval(code)){sx.API(true && s.base);try{return read();}catch(error){return error.name;}}")]
#[case::parameter_concise("const probe=(code,read=()=>eval(code))=>(sx.API(true && s.base),read());")]
#[serial]
fn stylex_eval_guard_preserves_lexical_boundaries_when_eval_is_defined_outside_storage(
    #[case] body: &str,
    #[values("props", "attrs")] api: &str,
) {
    // Given: dynamic invocation inside an inner block does not relocate an eval closure's scope.
    reset_class_map();
    reset_file_map();
    let source = format!("import * as sx from '@stylexjs/stylex';const s=sx.create({{base:{{color:'red',backgroundColor:'blue'}}}});{}", body.replace("API", api));
    // When
    let output = extract("capture-eval-boundary.js", &source, ExtractOption::default()).unwrap();
    let observed = observe_eval(&output.code, "probe(String.fromCharCode(95,115,48))");
    // Then: direct eval still cannot find the new inner binding (including body/default scopes).
    assert_eq!(observed, "ReferenceError", "{}", output.code);
    assert_eq!(callable_count(&output.code), callable_count(&source));
}

#[rstest]
#[case::direct_once("eval(code) && s.one")]
#[case::empty("eval(code) ? null : null")]
#[case::equal("eval(code) ? s.base : s.base")]
#[case::overridden("eval(code) && s.base,s.base")]
#[serial]
fn stylex_eval_guard_preserves_authored_errors_when_no_binding_is_injected(
    #[case] arguments: &str,
    #[values("props", "attrs")] api: &str,
) {
    // Given: equal/empty/overridden/direct-once conditions never need a capture slot.
    reset_class_map();
    reset_file_map();
    let source = format!("import * as sx from '@stylexjs/stylex';const s=sx.create({{one:{{color:'red'}},base:{{color:'red',backgroundColor:'blue'}}}});function probe(code){{try{{sx.{api}({arguments});return 'accepted';}}catch(error){{return error.name;}}}}");
    // When
    let output = extract("capture-eval-no-storage.js", &source, ExtractOption::default()).unwrap();
    let observed = observe_eval(&output.code, "probe(String.fromCharCode(95,115,48))");
    // Then: compilation is accepted and the original authored error is neither dropped nor masked.
    assert_eq!(observed, "ReferenceError", "{}", output.code);
    assert_eq!(callable_count(&output.code), callable_count(&source));
}

fn observe_eval(code: &str, call: &str) -> String {
    let arena = Allocator::default();
    let mut parsed = Parser::new(&arena, code, SourceType::mjs()).parse();
    assert!(parsed.diagnostics.is_empty(), "{code}");
    parsed.program.body.retain(|s| !matches!(s, oxc_ast::ast::Statement::ImportDeclaration(_)));
    let script = format!("{};try{{String({call});}}catch(error){{error.name;}}", oxc_codegen::Codegen::new().build(&parsed.program).code);
    boa_engine::Context::default().eval(boa_engine::Source::from_bytes(&script)).unwrap().as_string().unwrap().to_std_string_escaped()
}
