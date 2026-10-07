#[path = "styled_creation_support.rs"]
mod support;

#[path = "styled_creation_argument_tests.rs"]
mod whole_arguments;

use crate::assignment_test_support::{compiled_jsx, evaluate};
use rstest::rstest;
use serial_test::serial;
use support::{RULES, compiled, getters, selected};

#[test]
#[serial]
fn literal_fields_are_read_at_creation_when_component_renders_twice() {
    // Given: independent authored-object evaluation with counting getters.
    let setup = getters(true, "b");
    let expected = evaluate(&format!(
        r"
{setup}
const rules = {RULES};
const created = trace.slice();
state.w = '99px'; state.typo = 'other';
JSON.stringify([created, trace.slice(), trace.slice()]);
"
    ));
    // When: extraction's real generated component is created and rendered twice.
    let actual = evaluate(&format!(
        r"
{setup}
{}
const created = trace.slice();
state.w = '99px'; state.typo = 'other';
A({{}}); const first = trace.slice(); A({{}});
JSON.stringify([created, first, trace.slice()]);
",
        compiled(RULES)
    ));
    // Then: all authored reads occur once, in order, before either render.
    assert_eq!(actual, expected);
    assert_eq!(
        expected,
        r#"[["cond","pos","key","typo","w","p0","p1"],["cond","pos","key","typo","w","p0","p1"],["cond","pos","key","typo","w","p0","p1"]]"#
    );
}

#[rstest]
#[case(true, "b", false)]
#[case(false, "a", false)]
#[case(true, "b", true)]
#[case(false, "a", true)]
#[serial]
fn emitted_classes_keep_creation_values_and_layers_when_inputs_change(
    #[case] condition: bool,
    #[case] key: &str,
    #[case] literal_padding: bool,
) {
    // Given: known source selections, responsive padding and a nested authored layer.
    let color = if condition { "red" } else { "blue" };
    let background = match key {
        "a" => "red",
        "b" => "blue",
        _ => panic!("fixture key"),
    };
    let rules = if literal_padding {
        RULES.replace("[p0, p1]", "[1, 2]")
    } else {
        RULES.to_string()
    };
    let expected = serde_json::json!([
        ["background", background, 0, "base"],
        ["bottom", "0", 0, "base"],
        ["color", color, 0, "base"],
        ["left", "0", 0, "base"],
        ["margin", "4px", 0, "base.inner"],
        ["padding", "4px", 0, "base"],
        ["padding", "8px", 1, "base"],
        ["typography", "heading", 0, null],
        ["width", "13px", 0, "base"]
    ]);
    // When: selected emitted classes are paired with their actual inline values twice.
    let actual: serde_json::Value =
        serde_json::from_str(&selected(&rules, &getters(condition, key)))
            .unwrap_or_else(|error| panic!("projected selections must be JSON: {error}"));
    // Then: no unselected class/value leaks, and each layer/breakpoint stays associated.
    assert_eq!(actual, serde_json::json!([expected.clone(), expected]));
}

#[test]
#[serial]
fn creation_error_stops_later_fields_when_width_getter_throws() {
    // Given: width throws after earlier literal fields have evaluated.
    let setup = format!(
        r"{}
Object.defineProperty(globalThis, 'w', {{ get() {{ trace.push('w'); throw Error('width'); }} }});
",
        getters(true, "b")
    );
    let expected = evaluate(&format!(
        r"{setup}
let error; try {{ const rules = {RULES}; }} catch (caught) {{ error = caught.message; }}
JSON.stringify([trace,error]);"
    ));
    // When: the generated declaration is evaluated without any render.
    let actual = evaluate(&format!(
        r"{setup}
let error; try {{ {} }} catch (caught) {{ error = caught.message; }}
JSON.stringify([trace,error]);",
        compiled(RULES)
    ));
    // Then: the creation error and prefix trace match source, including skipped padding.
    assert_eq!(actual, expected);
}

#[test]
#[serial]
fn repeated_scalar_reads_keep_distinct_values_when_getter_changes_each_read() {
    // Given: identical getter syntax at two distinct authored field sites.
    let rules = "{width: state.size, height: state.size}";
    let setup = "let count=0; const state={get size(){return `${++count}px`}};";
    // When: both selected variable assignments are observed after two renders.
    let result = selected(rules, setup);
    // Then: source reads remain distinct, with no render-time rereads.
    assert_eq!(
        result,
        r#"[[["height","2px",0,null],["width","1px",0,null]],[["height","2px",0,null],["width","1px",0,null]]]"#
    );
}

#[test]
#[serial]
fn callback_body_remains_deferred_when_template_mixin_reads_runtime_input() {
    // Given: a function interpolation whose source body runs on rendering, not creation.
    let source = "import { styled } from '@devup-ui/react'; const A = styled.div`${props => props.enabled && 'external'}`;";
    // When: two different render inputs invoke the callback.
    let result = evaluate(&format!(
        r"
const trace=[]; {}
const created=trace.slice();
const first=A({{get enabled(){{trace.push('first');return true}}}});
const second=A({{get enabled(){{trace.push('second');return false}}}});
JSON.stringify([created,trace,first.className,second.className]);
",
        compiled_jsx(source)
    ));
    // Then: the callback is not frozen at declaration and retains per-render selection.
    assert_eq!(result, r#"[[],["first","second"],"external",""]"#);
}

#[test]
#[serial]
fn authored_binding_survives_when_its_name_matches_a_creation_capture_candidate() {
    // Given: an authored binding deliberately collides with the width capture's source name.
    let prefix = "import { styled } from '@devup-ui/react'; const A = styled.div({width: ";
    let name = format!("__devupStyled{}", prefix.len());
    let source = format!("{prefix}state.size, height: {name}}});");
    // When: extraction and two renders use both the getter and that authored binding.
    let result = evaluate(&format!(
        r"
let count=0; const state={{get size(){{count++;return '13px'}}}}; const {name}='17px';
{}
const first=A({{}}); const second=A({{}});
JSON.stringify([count,Object.values(first.style).sort(),Object.values(second.style).sort()]);
",
        compiled_jsx(&source)
    ));
    // Then: capture renaming preserves the independent authored value without rereading.
    assert_eq!(result, r#"[1,["13px","17px"],["13px","17px"]]"#);
}

#[test]
#[serial]
fn unselected_branch_stays_lazy_when_literal_fields_are_captured_at_creation() {
    // Given: the unselected color getter throws, and the selected getter precedes width.
    let rules = "{color: choice ? values.red : values.blue, width: values.width}";
    let setup = r"
const trace=[];
Object.defineProperty(globalThis,'choice',{get(){trace.push('choice');return false}});
const values={get red(){throw Error('unselected')},
  get blue(){trace.push('blue');return 'blue'},get width(){trace.push('width');return '13px'}};
";
    let expected = evaluate(&format!(
        r"{setup}
const rules={rules}; const created=trace.slice();
JSON.stringify([created,trace.slice(),trace.slice(),Object.values(rules).sort()]);"
    ));
    // When: the generated literal component is created and then rendered twice.
    let actual = evaluate(&format!(
        r"{setup}
{} const created=trace.slice(); const first=A({{}}); const rendered=trace.slice(); A({{}});
JSON.stringify([created,rendered,trace.slice(),Object.values(first.style).sort()]);",
        compiled(rules)
    ));
    // Then: source laziness, source order and selected inline values survive creation capture.
    assert_eq!(actual, expected);
}
