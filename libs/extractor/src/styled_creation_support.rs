use crate::{
    ExtractStyleValue,
    assignment_test_support::{evaluate, extracted, jsx_js},
    extract_style::style_property::StyleProperty,
};

#[test]
#[serial_test::serial]
fn literal_typography_remains_selected_beside_static_css_after_repeated_renders() {
    // Given
    let setup = "const state={};";
    // When
    let actual = selected("{typography:'heading',color:'red'}", setup);
    // Then
    assert_eq!(
        actual,
        "[[[\"color\",\"red\",0,null],[\"typography\",\"heading\",0,null]],[[\"color\",\"red\",0,null],[\"typography\",\"heading\",0,null]]]"
    );
}

pub(super) const RULES: &str = r"{ '@layer': { base: {
  color: cond ? 'red' : 'blue',
  positioning: pos,
  bg: { a: 'red', b: 'blue' }[key],
  typography: typo,
  width: w,
  p: [p0, p1],
  '@layer': { inner: { m: 1 } },
} } }";

pub(super) fn getters(condition: bool, key: &str) -> String {
    format!(
        r"
const trace = [];
const state = {{ cond: {condition}, pos: 'bottom-left', key: '{key}',
  typo: 'heading', w: '13px', p0: '4px', p1: '8px' }};
for (const name of Object.keys(state)) {{
  Object.defineProperty(globalThis, name, {{ configurable: true,
    get() {{ trace.push(name); return state[name]; }} }});
}}
"
    )
}

pub(super) fn compiled(rules: &str) -> String {
    jsx_js(
        &extracted(&format!(
            "import {{ styled }} from '@devup-ui/react'; const A = styled.div({rules});"
        ))
        .code,
    )
}

pub(super) fn selected(rules: &str, setup: &str) -> String {
    let output = extracted(&format!(
        "import {{ styled }} from '@devup-ui/react'; const A = styled.div({rules});"
    ));
    let records = output
        .styles
        .iter()
        .filter_map(|record| match (record, record.extract(None)?) {
            (ExtractStyleValue::Static(style), StyleProperty::ClassName(class)) => Some((
                class,
                style.property.clone(),
                style.value.clone(),
                style.level,
                style.layer.clone(),
                None,
            )),
            (
                ExtractStyleValue::Dynamic(style),
                StyleProperty::Variable {
                    class_name,
                    variable_name,
                    ..
                },
            ) => Some((
                class_name,
                style.property().to_string(),
                String::new(),
                style.level(),
                style.layer.clone(),
                Some(variable_name),
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    let records = serde_json::to_string(&records)
        .unwrap_or_else(|error| panic!("style records must serialize: {error}"));
    evaluate(&format!(
        r"
{setup}
{}
const records = {records};
const project = node => String(node.className ?? '').split(/\s+/).filter(Boolean).map(name => {{
  if (name.startsWith('typo-')) return ['typography', name.slice(5), 0, null];
  const record = records.find(record => record[0] === name);
  if (!record) throw Error('unbound emitted class ' + name);
  const [, property, value, level, layer, variable] = record;
  if (variable && !Object.prototype.hasOwnProperty.call(node.style, variable))
    throw Error('selected class has no inline assignment ' + variable);
  return [property, variable ? node.style[variable] : value, level, layer];
}}).sort((a,b) => JSON.stringify(a).localeCompare(JSON.stringify(b)));
const first = project(A({{}}));
state.cond = !state.cond; state.pos = 'top-right'; state.key = 'missing';
state.typo = 'other'; state.w = '99px'; state.p0 = '40px'; state.p1 = '80px';
JSON.stringify([first, project(A({{}}))]);
",
        jsx_js(&output.code)
    ))
}
