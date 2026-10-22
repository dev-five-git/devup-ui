#[rstest]
#[case::inherited(("s.base,tick('equal',on)&&s.base", "[\"equal\"]", "[\"blue\",\"red\"]", "[\"blue\",\"red\"]"))]
#[case::discarded(("tick('first',!on)&&s.one,tick('last',on)?s.one:s.other", "[\"first\",\"last\"]", "[\"red\"]", "[\"green\"]"))]
#[case::empty_before_live(("tick('empty',on)?s.empty:s.empty,tick('last',on)&&s.one", "[\"empty\",\"last\"]", "[\"red\"]", "[]"))]
#[case::equal_before_live(("tick('equal',on)?s.base:s.base,tick('last',on)?s.one:s.other", "[\"equal\",\"last\"]", "[\"blue\",\"red\"]", "[\"blue\",\"green\"]"))]
#[case::overridden_before_live(("tick('first',on)&&s.base,s.base,tick('last',on)?s.one:s.other", "[\"first\",\"last\"]", "[\"blue\",\"red\"]", "[\"blue\",\"green\"]"))]
#[case::inherited_reset(("s.reset,tick('equal',on)&&s.reset", "[\"equal\"]", "[]", "[]"))]
#[case::dead_tail(("[s.base,[tick('equal',on)&&s.base]],tick('empty',!on)?s.empty:s.empty", "[\"equal\",\"empty\"]", "[\"blue\",\"red\"]", "[\"blue\",\"red\"]"))]
#[serial]
fn stylex_no_storage_preserves_winners_when_final_dependencies_disappear(
    #[case] sample: (&str, &str, &str, &str),
    #[values("default", "field", "eval")] frame: &str,
    #[values("props", "attrs")] api: &str,
) {
    // Given: authored tests include inherited-equal and later-discarded choices in guarded frames.
    reset_class_map();
    reset_file_map();
    let source = no_storage_source(sample.0, frame, api);
    // When: the actual public extractor lowers the expression-only/eval-visible call.
    let output = extract("no-storage.js", &source, ExtractOption::default()).unwrap();
    // Then: both truth values retain ordered effects and the independently authored CSS winner.
    for (on, expected) in [(true, sample.2), (false, sample.3)] {
        let observed = render_observation(&output, &format!("render({on})"), Some("no-storage.js"));
        let (trace, mut values): (Vec<String>, Vec<String>) = serde_json::from_str(&observed).unwrap();
        values.sort();
        assert_eq!(trace, serde_json::from_str::<Vec<String>>(sample.1).unwrap(), "{}", output.code);
        assert_eq!(values, serde_json::from_str::<Vec<String>>(expected).unwrap(), "{}", output.code);
    }
    assert_eq!(callable_count(&output.code), callable_count(&source));
    assert!(!output.code.contains("let _s"), "{}", output.code);
    eprintln!("NO STORAGE {frame} {api} {} bytes={}\n{}", sample.0, output.code.len(), output.code);
}

fn no_storage_source(arguments: &str, frame: &str, api: &str) -> String {
    let attribute = if api == "props" { "className" } else { "class" };
    let call = format!("sx.{api}({arguments})");
    let body = match frame {
        "default" => format!("function choose(on,out={call}){{return out;}}function render(on){{return adapt(choose(on));}}"),
        "field" => format!("function render(on){{class C{{out={call};}}return adapt(new C().out);}}"),
        "eval" => format!("function render(on){{eval('');return adapt({call});}}"),
        "body" => format!("function render(on){{return adapt({call});}}"),
        _ => panic!("unknown fixture frame: {frame}"),
    };
    format!("import * as sx from '@stylexjs/stylex';import {{jsx}} from 'react/jsx-runtime';const s=sx.create({{base:{{color:'red',backgroundColor:'blue'}},one:{{color:'red'}},other:{{color:'green'}},empty:{{}},reset:{{color:null,backgroundColor:null}}}});function tick(label,value){{trace.push(label);return value;}}function adapt(out){{return jsx('div',{{className:out.{attribute}}});}}{body}")
}

#[rstest]
#[serial]
fn stylex_no_storage_uses_the_last_test_directly_when_an_earlier_result_is_captured(
    #[values("props", "attrs")] api: &str,
    #[values(false, true)] on: bool,
) {
    // Given: the final one-key test is direct, but its false branch needs an earlier live result.
    reset_class_map();
    reset_file_map();
    let source = no_storage_source("tick('first',on)&&s.one,tick('last',!on)&&s.other", "body", api);
    // When
    let output = extract("no-storage-last.js", &source, ExtractOption::default()).unwrap();
    let observed = render_observation(&output, &format!("render({on})"), Some("no-storage-last.js"));
    // Then: both tests execute once in order and either branch selects the authored winner.
    let winner = if on { "red" } else { "green" };
    assert_eq!(observed, format!("[[\"first\",\"last\"],[\"{winner}\"]]"), "{}", output.code);
    assert_eq!(output.code.matches("let _s0;").count(), 1, "{}", output.code);
    assert!(!output.code.contains("_s1"), "{}", output.code);
    assert_eq!(callable_count(&output.code), callable_count(&source));
    eprintln!("LAST DIRECT {api} {on} bytes={}\n{}", output.code.len(), output.code);
}

#[rstest]
#[case::inherited("s.base,state.on&&s.base,tick('later',true)?s.empty:s.empty")]
#[case::discarded("state.on&&s.one,tick('later',true)?s.one:s.other")]
#[serial]
fn stylex_no_storage_preserves_throws_when_the_throwing_choice_is_dead(
    #[case] arguments: &str,
    #[values("default", "field", "eval")] frame: &str,
    #[values("props", "attrs")] api: &str,
) {
    // Given: a throwing getter precedes a later authored test, even though its classes vanish.
    reset_class_map();
    reset_file_map();
    let source = format!("{}const trace=[];const state={{get on(){{trace.push('get');throw Error('stop');}}}};", no_storage_source(arguments, frame, api));
    // When: execute extracted code inside the original caller's catch boundary.
    let output = extract("no-storage-throw.js", &source, ExtractOption::default()).unwrap();
    let observed = observe_eval(&output.code, "(()=>{try{render(true);}catch(error){trace.push(error.message);}return JSON.stringify(trace);})()");
    // Then: no later test executes after the throw, and no callable is synthesized.
    assert_eq!(observed, "[\"get\",\"stop\"]", "{}", output.code);
    assert_eq!(callable_count(&output.code), callable_count(&source));
}

#[rstest]
#[serial]
fn stylex_no_storage_requires_capture_when_a_later_dead_test_follows_a_live_test(
    #[values("body", "default", "field", "eval")] frame: &str,
    #[values("props", "attrs")] api: &str,
) {
    // Given: only the earlier single-key condition survives; the later effect still must follow it.
    reset_class_map();
    reset_file_map();
    let source = no_storage_source("tick('live',on)&&s.one,tick('dead',false)?s.empty:s.empty", frame, api);
    // When
    let result = extract("no-storage-order.js", &source, ExtractOption::default());
    // Then: bodies capture, while expression-only/eval-visible frames retain genuine errors.
    match frame {
        "body" => {
            let output = result.unwrap();
            for (on, values) in [(true, "[\"red\"]"), (false, "[]")] {
                assert_eq!(render_observation(&output, &format!("render({on})"), Some("no-storage-order.js")), format!("[[\"live\",\"dead\"],{values}]"), "{}", output.code);
            }
            assert!(output.code.contains("let _s"), "{}", output.code);
            assert_eq!(callable_count(&output.code), callable_count(&source));
        }
        "default" | "field" | "eval" => {
            let error = result.unwrap_err().to_string();
            let column = source.find("tick('live',on)").unwrap() + 1;
            assert!(error.starts_with(&format!("no-storage-order.js:1:{column}:")), "{error}");
            let context = match frame {
                "default" => "parameter default",
                "field" => "class field initializer",
                "eval" => "visible to direct eval",
                _ => unreachable!(),
            };
            assert!(error.contains(context), "{error}");
        }
        _ => unreachable!(),
    }
}

#[rstest]
#[case::inherited("s.base,css&&s.base")]
#[case::discarded("css&&s.one,tick('last',on)?s.one:s.other")]
#[case::empty("css?s.empty:s.empty,tick('last',on)?s.one:s.other")]
#[case::overridden("css&&s.base,s.base,tick('last',on)?s.one:s.other")]
#[serial]
fn stylex_no_storage_keeps_semantic_diagnostics_when_a_compiled_test_is_discarded(
    #[case] arguments: &str,
    #[values("default", "field", "eval")] frame: &str,
    #[values("props", "attrs")] api: &str,
) {
    // Given: the discarded test still reads the real imported compile-time API.
    reset_class_map();
    reset_file_map();
    let source = format!("import {{css}} from '@devup-ui/react';\n{}", no_storage_source(arguments, frame, api));
    // When
    let error = extract("no-storage-binding.js", &source, ExtractOption::default()).unwrap_err().to_string();
    // Then: original semantic IDs locate the read, not a new storage-context diagnostic.
    let column = source.split_once('\n').unwrap().1.find("css").unwrap() + 1;
    assert!(error.starts_with(&format!("no-storage-binding.js:2:{column}:")), "{error}");
    assert!(error.contains("`css` is read at runtime"), "{error}");
    assert!(!error.contains("condition capture"), "{error}");
}

#[rstest]
#[serial]
fn stylex_no_storage_stays_bounded_and_deterministic_when_many_choices_are_discarded(
    #[values("default", "field", "eval")] frame: &str,
    #[values("props", "attrs")] api: &str,
) {
    // Given: every both-sided condition replaces the earlier single-key choice.
    let mut sizes = Vec::new();
    for count in [8, 16, 32, 64] {
        reset_class_map();
        reset_file_map();
        let arguments = (0..count).map(|i| format!("tick('{i}',{i}%2===0)?s.one:s.other")).collect::<Vec<_>>().join(",");
        let source = no_storage_source(&arguments, frame, api);
        // When
        let output = extract("no-storage-growth.js", &source, ExtractOption::default()).unwrap();
        let observed = render_observation(&output, "render(true)", Some("no-storage-growth.js"));
        let (trace, values): (Vec<String>, Vec<String>) = serde_json::from_str(&observed).unwrap();
        // Then: all effects survive in order, only the last winner survives, with linear JS growth.
        assert_eq!(trace, (0..count).map(|i| i.to_string()).collect::<Vec<_>>());
        assert_eq!(values, ["green"]);
        assert!(!output.code.contains("let _s"), "{}", output.code);
        assert!(output.code.len() < count * 100 + 1000, "{}", output.code.len());
        assert_eq!(callable_count(&output.code), callable_count(&source));
        reset_class_map();
        reset_file_map();
        assert_eq!(output.code, extract("no-storage-growth.js", &source, ExtractOption::default()).unwrap().code);
        sizes.push((count, output.code.len()));
    }
    eprintln!("NO STORAGE GROWTH {frame} {api} {sizes:?}");
}

#[test]
#[should_panic(expected = "unknown fixture frame: invalid")]
fn no_storage_source_rejects_unknown_fixture_frame() {
    // Given: the frame is not one of the fixture helper's supported execution contexts.
    // When: constructing that fixture must fail rather than silently use a different frame.
    // Then: the helper reports the unsupported frame in its existing panic.
    no_storage_source("", "invalid", "props");
}
