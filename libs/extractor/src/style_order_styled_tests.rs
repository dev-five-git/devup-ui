use crate::{ExtractOption, extract};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("styled.div([{ color: 'red', styleOrder: '01' }, { color: 'blue' }])")]
#[case("styled('div', [{ color: 'red', styleOrder: '01' }, { color: 'blue' }])")]
#[case("styled('div')([flag && { color: 'red', styleOrder: '01' }, { color: 'blue' }])")]
#[case("styled.div([flag ? { color: 'red', styleOrder: '01' } : { color: 'blue' }])")]
#[serial]
fn invalid_order_is_rejected_when_styled_rules_compose(
    #[case] expression: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source =
        format!("import {{ styled }} from '@devup-ui/react';\nexport const Card = {expression};");
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    // When
    let result = extract("styled-order.tsx", &source, ExtractOption::default());
    // Then
    let error = result
        .err()
        .ok_or("invalid composed order must not compile")?
        .to_string();
    let column = "export const Card = ".len() + expression.find("'01'").ok_or("missing order")? + 1;
    assert!(
        error.contains(&format!("styled-order.tsx:2:{column}:")),
        "{error}"
    );
    assert!(
        error.contains("`styleOrder()` cannot use `'01'`"),
        "{error}"
    );
    assert!(error.contains("an integer from 1 to 254"), "{error}");
    Ok(())
}
