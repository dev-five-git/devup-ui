use std::fmt::{Display, Formatter};

use crate::constant::DOUBLE_SEPARATOR;

pub enum SelectorSeparator {
    Single,
    Double,
    None,
}
impl Display for SelectorSeparator {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SelectorSeparator::Single => ":",
                SelectorSeparator::Double => "::",
                SelectorSeparator::None => "",
            }
        )
    }
}
impl From<&str> for SelectorSeparator {
    fn from(value: &str) -> Self {
        if value.starts_with(":") || value.is_empty() || value.starts_with("[") {
            SelectorSeparator::None
        } else if DOUBLE_SEPARATOR.contains(value) {
            SelectorSeparator::Double
        } else {
            SelectorSeparator::Single
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from() {
        assert!(matches!("placeholder".into(), SelectorSeparator::Double));
        assert!(matches!("before".into(), SelectorSeparator::Double));
        assert!(matches!("after".into(), SelectorSeparator::Double));

        assert!(matches!("hover".into(), SelectorSeparator::Single));

        assert!(matches!(":hover".into(), SelectorSeparator::None));

        assert!(matches!("::placeholder".into(), SelectorSeparator::None));

        assert!(matches!(
            "[aria-disabled='true']".into(),
            SelectorSeparator::None
        ));
    }

    #[test]
    fn test_display() {
        assert_eq!(SelectorSeparator::Double.to_string(), "::");
        assert_eq!(SelectorSeparator::Single.to_string(), ":");
        assert_eq!(SelectorSeparator::None.to_string(), "");
    }
}
