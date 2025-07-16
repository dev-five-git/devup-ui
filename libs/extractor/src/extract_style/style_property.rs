pub enum StyleProperty {
    ClassName(String),
    Variable {
        class_name: String,
        variable_name: String,
        identifier: String,
    },
}
impl StyleProperty {
    pub fn to_string(&self) -> String {
        match self {
            StyleProperty::ClassName(name) => name.clone(),
            StyleProperty::Variable { variable_name, .. } => format!("var({})", variable_name),
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
        let prop = StyleProperty::Variable {
            class_name: "cls".to_string(),
            variable_name: "--var-name".to_string(),
            identifier: "id".to_string(),
        };
        assert_eq!(prop.to_string(), "var(--var-name)".to_string());
    }
}
