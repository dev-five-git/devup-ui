use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(r#"{"color":"red",":hover":null,"@starting-style":0,"selectors":{"button &":false}}"#)]
#[case(r#"{...{},[name]:{"color":"blue"},"color":"red"}"#)]
#[serial]
fn authored_serializer_output_when_rule_members_are_unwalkable_is_preserved(
    #[case] serialized: &str,
) -> Result<(), String> {
    css::file_map::reset_file_map();
    css::class_map::reset_class_map();
    let source = format!(
        "import {{globalStyle}} from '@devup-ui/react'; JSON.stringify=()=>{}; globalStyle('body',{{color:'red'}});",
        serde_json::to_string(serialized).map_err(|error| error.to_string())?
    );
    let (collected, _) = crate::vanilla_extract::execute_stylesheet(
        &source,
        "/serializer.css.ts",
        &crate::ExtractOption::default(),
        None,
    )?;
    assert_eq!(
        collected.global_styles,
        vec![("body".into(), serialized.into())]
    );
    Ok(())
}
