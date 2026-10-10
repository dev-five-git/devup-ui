use css::utils::compile_regex;

/// Labels only generated offset-bearing bindings, retaining all executable syntax.
pub(crate) fn normalize_bindings(code: &str) -> String {
    let mut bindings = Vec::new();
    compile_regex(r"\b__devup(?:Assignment|Styled)\d+\b")
        .replace_all(code, |caps: &regex_lite::Captures| {
            let index = bindings
                .iter()
                .position(|binding| *binding == caps[0])
                .unwrap_or_else(|| {
                    bindings.push(caps[0].to_string());
                    bindings.len() - 1
                });
            format!("__capture{index}")
        })
        .into_owned()
}

/// Recognizes only literal JSX classes or the complete one-level static tuple flow.
/// Every JSX class occurrence must match; unsupported flows fail closed.
pub(crate) fn static_element_classes(
    code: &str,
    expected_tokens: &[Option<&str>],
) -> Vec<Vec<String>> {
    let elements = compile_regex(concat!(
        r#"<div className="([^"]+)" />|"#,
        r#"(?s)\(\((__devupAssignment\d+)\) => <div className=.*? />\)"#,
        r#"\(\(\(__devupLevel0\) => \[.*?\]\)\(\(\(__devupValue\) => \["#,
        r#"\s*"([^"]+)",.*?\]\)\("([^"]+)"\)\)\)"#,
    ));
    let classes: Vec<Vec<String>> = elements
        .captures_iter(code)
        .enumerate()
        .map(|(index, caps)| {
            let expected_token = expected_tokens
                .get(index)
                .unwrap_or_else(|| panic!("{code}"));
            let names = if let Some(literal) = caps.get(1) {
                assert_eq!(*expected_token, None, "{code}");
                literal.as_str()
            } else {
                let binding = &caps[2];
                let names = &caps[3];
                let token = &caps[4];
                assert_eq!(*expected_token, Some(token), "{code}");
                assert_eq!(
                    &caps[0],
                    format!(
                        "(({binding}) => <div className={{{binding}?.[0] ?? \"\"}} style={{{{ ...{binding}?.[1] }}}} />)(((__devupLevel0) => [\n\t__devupLevel0?.[0] ?? \"\",\n\t__devupLevel0?.[1],\n\t[__devupLevel0?.[2]]\n])(((__devupValue) => [\n\t\"{names}\",\n\t{{}},\n\t__devupValue\n])(\"{token}\")))"
                    ),
                    "static class/value projection changed: {code}"
                );
                names
            };
            names.split_whitespace().map(str::to_string).collect()
        })
        .collect();
    assert_eq!(classes.len(), expected_tokens.len(), "{code}");
    assert_eq!(classes.len(), code.matches("className=").count(), "{code}");
    classes
}

#[test]
fn generated_bindings_keep_distinct_references_when_labeled() {
    // Given
    let code = "((__devupStyled12, __devupStyled24) => [__devupStyled24, __devupStyled12])(w, v)";
    // When
    let normalized = normalize_bindings(code);
    // Then
    assert_eq!(
        normalized,
        "((__capture0, __capture1) => [__capture1, __capture0])(w, v)"
    );
}

const STATIC_ELEMENT: &str = concat!(
    "((__devupAssignment60) => <div className={__devupAssignment60?.[0] ?? \"\"} style={{ ...__devupAssignment60?.[1] }} />)(((__devupLevel0) => [\n",
    "\t__devupLevel0?.[0] ?? \"\",\n\t__devupLevel0?.[1],\n\t[__devupLevel0?.[2]]\n])(((__devupValue) => [\n",
    "\t\"literal-class\",\n\t{},\n\t__devupValue\n])(\"$space\")))",
);

#[test]
fn static_classes_resolve_when_literal_and_tuple_elements_are_adjacent() {
    // Given
    let code = format!("<div className=\"responsive-base responsive-level\" />{STATIC_ELEMENT}");
    // When
    let classes = static_element_classes(&code, &[None, Some("$space")]);
    // Then
    assert_eq!(
        classes,
        vec![
            vec!["responsive-base", "responsive-level"],
            vec!["literal-class"]
        ]
    );
}

#[test]
#[should_panic(expected = "static class/value projection changed")]
fn static_classes_reject_crossed_slots_when_style_uses_the_class_projection() {
    // Given
    let code = STATIC_ELEMENT.replace("...__devupAssignment60?.[1]", "...__devupAssignment60?.[0]");
    // When / Then
    static_element_classes(&code, &[Some("$space")]);
}

#[test]
#[should_panic(expected = "assertion `left == right` failed")]
fn static_classes_reject_unrecognized_projections_when_class_is_not_literal() {
    // Given
    let code = "<div className={other?.[0] ?? \"\"} />";
    // When / Then
    static_element_classes(code, &[Some("$space")]);
}
