use std::fmt::{Display, Error, Formatter};

pub enum StyleProperty {
    ClassName(String),
    Variable { class_name: String, variable_name: String, identifier: String },
}
impl Display for StyleProperty {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        match self {
            StyleProperty::ClassName(name) => write!(f, "{name}"),
            StyleProperty::Variable { variable_name, .. } => write!(f, "var({variable_name})"),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_string_class_name() {
        let prop = StyleProperty::ClassName("my-class".to_string());
        assert_eq!(prop.to_string(), "my-class".to_string());
    }

    #[test]
    fn test_to_string_variable() {
        let prop = StyleProperty::Variable { class_name: "cls".to_string(), variable_name: "--var-name".to_string(), identifier: "id".to_string() };
        assert_eq!(prop.to_string(), "var(--var-name)".to_string());
    }
}
