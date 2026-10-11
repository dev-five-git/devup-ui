use std::fmt::Debug;

pub(super) trait RequiredValue {
    type Value;
    fn required(self, context: &str) -> Self::Value;
}

impl<T> RequiredValue for Option<T> {
    type Value = T;
    fn required(self, context: &str) -> T {
        match self {
            Some(value) => value,
            None => panic!("{context}"),
        }
    }
}

impl<T, E: Debug> RequiredValue for Result<T, E> {
    type Value = T;
    fn required(self, context: &str) -> T {
        match self {
            Ok(value) => value,
            Err(error) => panic!("{context}: {error:?}"),
        }
    }
}

pub(super) trait RequiredError {
    type Error;
    fn required_err(self, context: &str) -> Self::Error;
}

impl<T: Debug, E> RequiredError for Result<T, E> {
    type Error = E;
    fn required_err(self, context: &str) -> E {
        match self {
            Err(error) => error,
            Ok(value) => panic!("{context}: unexpectedly succeeded with {value:?}"),
        }
    }
}

#[test]
#[should_panic(expected = "class result is missing")]
fn required_value_when_option_is_missing_fails_with_context() {
    let _: &str = None.required("class result is missing");
}

#[test]
#[should_panic(expected = "CSS compilation failed")]
fn required_value_when_result_is_error_fails_with_context() {
    let _: () = Err("invalid metadata").required("CSS compilation failed");
}

#[test]
#[should_panic(expected = "unexpectedly succeeded")]
fn required_error_when_result_succeeds_rejects_success() {
    let _: &str = Ok("class").required_err("metadata must fail");
}
