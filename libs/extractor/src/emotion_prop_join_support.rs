use crate::class_selection_support::{self, Modules};
use crate::extract_style::{extract_style_value::ExtractStyleValue, style_property::StyleProperty};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug)]
pub enum Case {
    Values,
    WidthCall,
    Compiled,
}

pub const VALUES: &str = "export const App = ({ c, w }) => <div css={{ color: c, width: w }} style={{ opacity: 1 }} className=\"x\" />;";
pub const WIDTH_CALL: &str = "const SPACE = 4;\nconst base = { padding: 4, lineHeight: 1.5, '&:hover': { margin: 2 } };\nexport const App = () => <div css={[base, { margin: SPACE, width: f({ size: 4 }) }]} />;";
pub const COMPILED: &str = "import { jsx as _jsx } from '@emotion/react/jsx-runtime';\nconst base = { padding: 2 };\nconst SPACE = 3;\nexport const App = ({ c, s }) => [_jsx('div', { id: 1 }), _jsx('div', { style: s, css: [base, { margin: SPACE, color: c }] })];";

#[derive(Debug)]
pub struct Element {
    pub target: String,
    pub class_name: String,
    pub inline: BTreeMap<String, String>,
}

#[derive(Debug)]
pub struct Join {
    pub token: String,
    pub property: String,
    pub selector: Option<String>,
    pub variable: Option<String>,
    pub value: String,
}

pub struct Record {
    pub outputs: Modules,
    pub element: Element,
    pub joins: Vec<Join>,
}

fn element(value: &serde_json::Value) -> Element {
    let props = value["props"]
        .as_object()
        .unwrap_or_else(|| panic!("element props"));
    assert_eq!(props.len(), 2, "only className and style on styled div");
    let inline = props["style"]
        .as_object()
        .unwrap_or_else(|| panic!("inline style"))
        .iter()
        .map(|(key, value)| {
            let scalar = match value {
                serde_json::Value::String(value) => value.clone(),
                serde_json::Value::Number(value) => value.to_string(),
                serde_json::Value::Null
                | serde_json::Value::Bool(_)
                | serde_json::Value::Array(_)
                | serde_json::Value::Object(_) => panic!("unexpected inline value: {value}"),
            };
            (key.clone(), scalar)
        })
        .collect();
    Element {
        target: value["target"]
            .as_str()
            .unwrap_or_else(|| panic!("native target"))
            .into(),
        class_name: props["className"]
            .as_str()
            .unwrap_or_else(|| panic!("selected tokens"))
            .into(),
        inline,
    }
}

pub fn selected(outputs: &Modules, element: &Element, single_css: bool) -> Vec<Join> {
    let mut joins = Vec::new();
    for token in element.class_name.split_whitespace() {
        for (name, output) in outputs {
            for style in &output.styles {
                let extracted = style.extract((!single_css).then_some(name.as_str()));
                let join = match (style, extracted) {
                    (ExtractStyleValue::Static(value), Some(StyleProperty::ClassName(class)))
                        if class == token =>
                    {
                        assert_eq!(
                            (value.level, &value.layer, value.style_order),
                            (0, &None, None)
                        );
                        Some(Join {
                            token: token.into(),
                            property: value.property.clone(),
                            selector: value.selector.as_ref().map(ToString::to_string),
                            variable: None,
                            value: value.value.clone(),
                        })
                    }
                    (
                        ExtractStyleValue::Dynamic(value),
                        Some(StyleProperty::Variable {
                            class_name,
                            variable_name,
                            ..
                        }),
                    ) if class_name == token => {
                        assert_eq!(
                            (value.level(), value.layer(), value.style_order()),
                            (0, None, None)
                        );
                        assert!(!value.important());
                        Some(Join {
                            token: token.into(),
                            property: value.property().into(),
                            selector: value.selector().map(ToString::to_string),
                            value: element
                                .inline
                                .get(&variable_name)
                                .unwrap_or_else(|| panic!("selected variable on actual element"))
                                .clone(),
                            variable: Some(variable_name),
                        })
                    }
                    _ => None,
                };
                if let Some(join) = join {
                    joins.push(join);
                }
            }
        }
    }
    joins
}

pub fn run(case: Case, single_css: bool) -> Record {
    let (original, observer, globals) = match case {
        Case::Values => (VALUES, "globalThis.observed=App({c:'orange',w:17});", ""),
        Case::WidthCall => (
            WIDTH_CALL,
            "const element=App();globalThis.observed={element,calls:globalThis.calls};",
            "globalThis.calls=[];globalThis.f=argument=>{globalThis.calls.push(argument);return 19;};",
        ),
        Case::Compiled => (
            COMPILED,
            "globalThis.observed=App({c:'purple',s:{opacity: 0.75}});",
            "",
        ),
    };
    let source = format!("import './environment.js';\n{original}");
    let outputs = class_selection_support::compile(
        &[("/namespace/prop.tsx", &source)],
        class_selection_support::option(single_css, "@devup-ui/react"),
    );
    let observed = class_selection_support::observe(
        &outputs,
        &format!("import {{App}} from './prop.tsx';{observer}"),
        globals,
    );
    let native = match case {
        Case::Values => element(&observed),
        Case::WidthCall => {
            assert_eq!(observed["calls"], serde_json::json!([{ "size": 4 }]));
            element(&observed["element"])
        }
        Case::Compiled => {
            let elements = observed
                .as_array()
                .unwrap_or_else(|| panic!("original two elements"));
            assert_eq!(elements.len(), 2);
            assert_eq!(
                elements[0],
                serde_json::json!({"target":"div","props":{"id":1}})
            );
            element(&elements[1])
        }
    };
    assert_eq!(native.target, "div");
    let joins = selected(&outputs, &native, single_css);
    let mut actual: Vec<_> = joins
        .iter()
        .map(|join| {
            (
                join.property.as_str(),
                join.selector.as_deref(),
                join.value.as_str(),
                join.variable.is_some(),
            )
        })
        .collect();
    actual.sort();
    let mut expected = match case {
        Case::Values => vec![
            ("color", None, "orange", true),
            ("width", None, "17px", true),
        ],
        Case::WidthCall => vec![
            ("padding", None, "4px", false),
            ("line-height", None, "1.5", false),
            ("margin", Some("&:hover"), "2px", false),
            ("margin", None, "4px", false),
            ("width", None, "19px", true),
        ],
        Case::Compiled => vec![
            ("color", None, "purple", true),
            ("margin", None, "3px", false),
            ("padding", None, "2px", false),
        ],
    };
    expected.sort();
    assert_eq!(actual, expected);
    let opaque: Vec<_> = native
        .class_name
        .split_whitespace()
        .filter(|token| !joins.iter().any(|join| join.token == *token))
        .collect();
    let (opaque_expected, opacity) = match case {
        Case::Values => (vec!["x"], Some("1")),
        Case::WidthCall => (vec![], None),
        Case::Compiled => (vec![], Some("0.75")),
    };
    assert_eq!(opaque, opaque_expected);
    assert_eq!(native.inline.get("opacity").map(String::as_str), opacity);
    assert_eq!(
        native.inline.len(),
        joins.iter().filter(|join| join.variable.is_some()).count()
            + usize::from(opacity.is_some())
    );
    Record {
        outputs,
        element: native,
        joins,
    }
}
