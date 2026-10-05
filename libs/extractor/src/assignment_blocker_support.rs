use crate::{
    ExtractStyleValue,
    assignment_test_support::{evaluate, extracted, jsx_js},
    extract_style::style_property::StyleProperty,
};

pub(super) fn selected(source: &str, result: &str) -> String {
    let output = extracted(source);
    let records = output
        .styles
        .iter()
        .filter_map(|value| {
            let style = value.extract(None)?;
            match (value, style) {
                (ExtractStyleValue::Static(value), StyleProperty::ClassName(class)) => Some((
                    class,
                    format!("{}:{}:{}", value.property, value.value, value.level),
                )),
                (
                    ExtractStyleValue::Dynamic(value),
                    StyleProperty::Variable {
                        class_name,
                        variable_name,
                        ..
                    },
                ) => Some((
                    class_name,
                    format!(
                        "{}:var({}):{}",
                        value.property(),
                        variable_name,
                        value.level()
                    ),
                )),
                _ => None,
            }
        })
        .collect::<Vec<_>>();
    let records = serde_json::to_string(&records).unwrap_or_else(|error| panic!("{error}"));
    evaluate(&format!(
        "{} const records={records}; const selected=node=>String(node.className??'').split(/\\s+/).filter(Boolean).map(name=>{{const entry=records.find(([key])=>key===name);if(!entry)throw Error('unbound class '+name);return entry[1]}}).sort(); const assigned=node=>selected(node).map(value=>value.replace(/var\\(([^)]+)\\)/g,(_,variable)=>String(node.style[variable]))); {result}",
        jsx_js(&output.code)
    ))
}
