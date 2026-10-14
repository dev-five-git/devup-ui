use std::{cell::RefCell, path::Path};

use boa_engine::{Context, JsValue, NativeFunction, Source, js_string};

#[derive(Debug)]
struct Frame {
    name: String,
    path: String,
    position: Option<(u32, u32)>,
}

thread_local! {
    static CALLS: RefCell<Vec<Vec<Frame>>> = const { RefCell::new(Vec::new()) };
}

fn style(args: &[JsValue], context: &Context) -> JsValue {
    let frames = context
        .stack_trace()
        .map(|frame| {
            let location = frame.position();
            Frame {
                name: location.function_name.to_std_string_escaped(),
                path: format!("{:?}", location.path),
                position: location
                    .position
                    .map(|p| (p.line_number(), p.column_number())),
            }
        })
        .collect();
    CALLS.with(|calls| calls.borrow_mut().push(frames));
    args.first().cloned().unwrap_or_else(JsValue::undefined)
}

fn coordinate(source: &str, offset: usize) -> (u32, u32) {
    let prefix = &source[..offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix
        .rsplit('\n')
        .next()
        .unwrap_or_else(|| panic!("prefix has a last line"))
        .len()
        + 1;
    (
        u32::try_from(line).unwrap_or_else(|error| panic!("fixture line fits: {error}")),
        u32::try_from(column).unwrap_or_else(|error| panic!("fixture column fits: {error}")),
    )
}

#[test]
fn public_positions_when_opaque_native_factory_is_called() {
    // Given: unmodified ASCII scripts and independently selected complete call expressions.
    let fixtures = [
        (
            "direct",
            "\nconst result = style({color: 'red'});\n",
            "style({color: 'red'})",
            None,
        ),
        (
            "higher-order",
            "function identity(f) { return f; }\nconst result = identity(style)({color: 'red'});\n",
            "identity(style)({color: 'red'})",
            None,
        ),
        (
            "bind",
            "const bound = style.bind(null);\nconst result = bound({color: 'red'});\n",
            "bound({color: 'red'})",
            None,
        ),
        (
            "call",
            "\nconst result = style.call(null, {color: 'red'});\n",
            "style.call(null, {color: 'red'})",
            None,
        ),
        (
            "apply",
            "\nconst result = style.apply(null, [{color: 'red'}]);\n",
            "style.apply(null, [{color: 'red'}])",
            None,
        ),
        (
            "builtinmap",
            "\nconst result = [{color: 'red'}].map(style);\n",
            "[{color: 'red'}].map(style)",
            None,
        ),
        (
            "nestedcallback",
            "function run(cb, f) { return cb(f); }\nconst result = run(function callback(f) { return f({color: 'red'}); }, style);\n",
            "f({color: 'red'})",
            None,
        ),
        (
            "eval",
            "const code = \"\\nconst result = style({color: 'red'});\\n\";\neval(code);\n",
            "style({color: 'red'})",
            Some("\nconst result = style({color: 'red'});\n"),
        ),
        (
            "nestedargument",
            "function identity(x) { return x; }\nconst result = identity(style({color: 'red'}));\n",
            "style({color: 'red'})",
            None,
        ),
        (
            "twoarguments",
            "function pair(a, b) { return [a, b]; }\nconst result = pair(style({color: 'red'}), style({color: 'blue'}));\n",
            "style({color: 'red'})",
            None,
        ),
    ];
    for (name, source, expression, generated) in fixtures {
        let input = generated.unwrap_or(source);
        let start = input
            .find(expression)
            .unwrap_or_else(|| panic!("independent fixture call exists"));
        let expected = coordinate(input, start);
        let end = coordinate(input, start + expression.len());
        let mut context = Context::default();
        context
            .register_global_builtin_callable(
                js_string!("style"),
                1,
                NativeFunction::from_fn_ptr(|_, args, context| Ok(style(args, context))),
            )
            .unwrap_or_else(|error| panic!("register native factory: {error}"));
        // When: the ordinary script calls the opaque native function.
        context
            .eval(Source::from_bytes(source).with_path(Path::new("caller-probe.js")))
            .unwrap_or_else(|error| panic!("evaluate fixture: {error}"));
        // Then: report every native invocation and compare to find(), never a nearby statement.
        let calls = CALLS.with(|calls| std::mem::take(&mut *calls.borrow_mut()));
        assert_eq!(
            calls.len(),
            if name == "twoarguments" { 2 } else { 1 },
            "{name}"
        );
        println!("FIXTURE {name} source={source:?} expected={expected:?} end={end:?}");
        for (index, frames) in calls.iter().enumerate() {
            let (call_start, call_end) = if index == 0 {
                (expected, end)
            } else {
                let second = "style({color: 'blue'})";
                let offset = input
                    .find(second)
                    .unwrap_or_else(|| panic!("second call exists"));
                (
                    coordinate(input, offset),
                    coordinate(input, offset + second.len()),
                )
            };
            println!("NATIVE {name}#{index} expected={call_start:?} end={call_end:?}");
            for (depth, frame) in frames.iter().enumerate() {
                let exact = frame.position == Some(call_start);
                let inside = frame
                    .position
                    .is_some_and(|p| call_start <= p && p < call_end);
                println!(
                    "FRAME {depth} name={:?} path={} position={:?} exact={exact} inside={inside}",
                    frame.name, frame.path, frame.position
                );
            }
        }
    }
}
