use super::literal_w38n_source::TestResult;

pub(super) fn selected(classes: &str, flag: bool) -> TestResult<Vec<String>> {
    let script =
        format!("const flag={flag};JSON.stringify(({classes}).trim().split(/\\s+/).sort());");
    let mut context = boa_engine::Context::default();
    let value = context.eval(boa_engine::Source::from_bytes(script.as_bytes()))?;
    let json = value.to_string(&mut context)?.to_std_string_escaped();
    Ok(serde_json::from_str(&json)?)
}
