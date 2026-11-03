use crate::ExtractStyleValue;
use crate::class_selection_support::Modules;
use crate::extract_style::style_property::StyleProperty;
use serde_json::{Value, json};

pub type RenderRecords = Vec<(Modules, Vec<Value>, bool)>;

/// Join selected atoms to concrete emitted values without evaluating detached expressions.
pub fn declarations(outputs: &Modules, element: &Value, single_css: bool) -> Vec<(String, Value)> {
    let tokens = element["props"]["className"]
        .as_str()
        .unwrap_or_else(|| panic!("class string"));
    assert_eq!(
        tokens
            .split_whitespace()
            .filter(|token| *token == "caller")
            .count(),
        1
    );
    let mut selected = Vec::new();
    let mut variables = std::collections::BTreeSet::new();
    for token in tokens.split_whitespace() {
        let mut matches = Vec::new();
        for (name, output) in outputs {
            for style in &output.styles {
                let filename = (!single_css).then_some(name.as_str());
                let (class, variable) = match style.extract(filename) {
                    Some(StyleProperty::ClassName(class)) => (class, None),
                    Some(StyleProperty::Variable {
                        class_name,
                        variable_name,
                        ..
                    }) => (class_name, Some(variable_name)),
                    None => continue,
                };
                if class != token {
                    continue;
                }
                let row = match style {
                    ExtractStyleValue::Static(style) => {
                        assert_eq!(
                            (style.level, &style.selector, &style.layer),
                            (0, &None, &None)
                        );
                        (style.property.clone(), json!(style.value))
                    }
                    ExtractStyleValue::Dynamic(style) => {
                        assert_eq!(
                            (style.level(), style.selector(), style.layer()),
                            (0, None, None)
                        );
                        let variable = variable.unwrap_or_else(|| panic!("dynamic variable"));
                        variables.insert(variable.clone());
                        let value = &element["props"]["style"][&variable];
                        assert!(
                            value.is_string() || value.is_number(),
                            "concrete CSS value: {element}"
                        );
                        (style.property().into(), value.clone())
                    }
                    other => panic!("unexpected selected style {other:?}"),
                };
                if !matches.contains(&row) {
                    matches.push(row);
                }
            }
        }
        assert!(
            matches.len() <= 1,
            "ambiguous selected atom {token}: {matches:?}"
        );
        selected.extend(matches);
    }
    let mut style = element["props"]["style"]
        .as_object()
        .unwrap_or_else(|| panic!("style object"))
        .clone();
    assert_eq!(style.remove("opacity"), Some(json!(0.5)));
    assert_eq!(
        style
            .keys()
            .cloned()
            .collect::<std::collections::BTreeSet<_>>(),
        variables
    );
    selected.sort_by(|a, b| a.0.cmp(&b.0));
    selected
}

pub fn props(element: &Value, expected: Value) {
    let mut actual = element["props"]
        .as_object()
        .unwrap_or_else(|| panic!("props object"))
        .clone();
    actual.remove("className");
    actual.remove("style");
    assert_eq!(Value::Object(actual), expected);
}
