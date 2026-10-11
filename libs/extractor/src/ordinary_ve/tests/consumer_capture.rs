use super::demand_support::TestResult;
use crate::extract_style::style_property::StyleProperty;
use crate::jsx_semantics_tests::whole::evaluate_compiled;
use crate::{ExtractOutput, ExtractStyleValue};

fn variable(output: &ExtractOutput, property: &str) -> Result<String, Box<dyn std::error::Error>> {
    let matching: Vec<_> = output
        .styles
        .iter()
        .filter_map(|value| match value {
            ExtractStyleValue::Dynamic(style) if style.property() == property => {
                match value.extract(None) {
                    Some(StyleProperty::Variable { variable_name, .. }) => Some(variable_name),
                    _ => panic!("dynamic {property} has no variable"),
                }
            }
            _ => None,
        })
        .collect();
    assert_eq!(matching.len(), 1, "{property}: {:?}", output.styles);
    matching
        .into_iter()
        .next()
        .ok_or_else(|| "one dynamic variable missing".into())
}

pub(super) fn box_values(output: &ExtractOutput, margin: &str) -> TestResult {
    let background = serde_json::to_string(&variable(output, "background")?)?;
    let width = serde_json::to_string(&variable(output, "width")?)?;
    let expected_width = serde_json::to_string(&format!("{margin}__width_suffix_731__"))?;
    let setup = concat!(
        "let bgReads=0,suffixReads=0,nameReads=0,handlerCalls=0;",
        "const props={get bg(){bgReads++;return '__background_419__';},",
        "get suffix(){suffixReads++;return '__width_suffix_731__';}};",
        "const window={get name(){nameReads++;return '__child_853__';}};",
        "const document={get title(){handlerCalls++;return '__handler_967__';}};"
    );
    let probe = format!(
        concat!(
            "(()=>{{const element=view(props);const style=element.props.style;",
            "const valid=()=>style[{background}]==='__background_419__'&&style[{width}]==={expected_width};",
            "const positive=valid();const saved=style[{background}];",
            "style[{background}]=style[{width}];const wrongSlot=valid();",
            "style[{background}]=saved;const restored=valid();",
            "return [positive,wrongSlot,restored,bgReads,suffixReads,nameReads,handlerCalls,",
            "typeof element.props.onClick,element.children];}})()"
        ),
        background = background,
        width = width,
        expected_width = expected_width
    );
    let actual = evaluate_compiled(&output.code, setup, &probe);
    assert_eq!(
        actual.element,
        serde_json::json!([true, false, true, 1, 1, 1, 0, "function", ["__child_853__"]])
    );
    Ok(())
}

pub(super) fn helper_value(output: &ExtractOutput) -> TestResult {
    let color = serde_json::to_string(&variable(output, "color")?)?;
    // Count emitted calls only in the execution harness; extraction sees the original fixture.
    let calls: Vec<_> = output
        .code
        .match_indices("color()")
        .filter(|(offset, _)| !output.code[..*offset].ends_with("function "))
        .map(|(offset, _)| offset)
        .collect();
    let mut compiled = output.code.clone();
    for offset in calls.into_iter().rev() {
        compiled.replace_range(offset..offset + "color()".len(), "(colorCalls++, color())");
    }
    let setup = concat!(
        "let colorCalls=0,nameReads=0;",
        "const window={get name(){nameReads++;return '__color_613__';}};"
    );
    let probe = format!(
        concat!(
            "(()=>{{const style=view.props.style;const valid=()=>style[{color}]==='__color_613__';",
            "const positive=valid();const saved=style[{color}];",
            "style[{color}]='__wrong_slot_827__';const wrongSlot=valid();",
            "style[{color}]=saved;const restored=valid();",
            "return [positive,wrongSlot,restored,colorCalls,nameReads];}})()"
        ),
        color = color
    );
    let actual = evaluate_compiled(&compiled, setup, &probe);
    assert_eq!(actual.element, serde_json::json!([true, false, true, 1, 1]));
    Ok(())
}
