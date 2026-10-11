use super::evaluate;
use super::literal_w38m_scalar_oracle::Expected;
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Rendered {
    pub(super) class_name: String,
    pub(super) variables: Option<BTreeMap<String, i32>>,
}

pub(super) fn observe(code: &str) -> (Vec<Rendered>, Vec<String>) {
    let observed = format!(
        "{code}\nfor(const rendered of [a,b]){{rendered.props.className=JSON.stringify([rendered.props.className,rendered.props.style??null]);}}"
    );
    let (encoded, trace) = evaluate(&observed);
    let renders = encoded
        .iter()
        .map(|json| {
            let (class_name, variables): (String, Option<BTreeMap<String, i32>>) =
                serde_json::from_str(json).unwrap_or_else(|error| panic!("{error}: {json}"));
            Rendered {
                class_name,
                variables,
            }
        })
        .collect();
    (renders, trace)
}

pub(super) fn assert_render(render: &Rendered, expected: &Expected) {
    let mut tokens = render.class_name.split_whitespace().collect::<Vec<_>>();
    tokens.sort_unstable();
    assert_eq!(tokens, expected.tokens);
    assert_eq!(render.variables.as_ref(), Some(&expected.variables));
}
