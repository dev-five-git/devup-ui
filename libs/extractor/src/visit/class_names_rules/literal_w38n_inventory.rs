use crate::extract_style::extract_static_style::ExtractStaticStyle;
use crate::{ExtractStyleProp, ExtractStyleValue};

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Inventory {
    Payload(String, Vec<ExtractStyleValue>),
    Conditional {
        test: String,
        yes: Option<Box<Inventory>>,
        no: Option<Box<Inventory>>,
    },
}

pub(super) fn inventory(props: &[ExtractStyleProp<'_>]) -> Vec<Inventory> {
    props.iter().map(project).collect()
}

fn project(prop: &ExtractStyleProp<'_>) -> Inventory {
    match prop {
        ExtractStyleProp::Conditional {
            condition,
            consequent,
            alternate,
        } => Inventory::Conditional {
            test: crate::utils::readable_code(condition),
            yes: consequent.as_deref().map(project).map(Box::new),
            no: alternate.as_deref().map(project).map(Box::new),
        },
        ExtractStyleProp::Static(_)
        | ExtractStyleProp::Diagnostic { .. }
        | ExtractStyleProp::StaticArray(_)
        | ExtractStyleProp::Enum { .. }
        | ExtractStyleProp::Expression { .. }
        | ExtractStyleProp::MemberExpression { .. }
        | ExtractStyleProp::Unreadable { .. } => {
            Inventory::Payload(format!("{prop:?}"), prop.extract())
        }
    }
}

pub(super) fn atom(property: &str, value: &str, order: Option<u8>) -> Inventory {
    let mut style = ExtractStaticStyle::new(property, value, 0, None);
    style.style_order = order;
    project(&ExtractStyleProp::Static(ExtractStyleValue::Static(style)))
}

pub(super) fn red_rules(order: Option<u8>) -> Vec<Inventory> {
    vec![
        atom("background", "blue", order),
        atom("color", "red", order),
    ]
}
