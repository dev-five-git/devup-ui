use std::process::Command;

#[test]
fn production_addon_executes_in_real_node() -> Result<(), Box<dyn std::error::Error>> {
    let preparation =
        "native coverage preparation is missing; run `bun run test` from the repository root";
    let node = std::env::var_os("DEVUP_NATIVE_COVERAGE_NODE").ok_or(preparation)?;
    let entry = std::env::var_os("DEVUP_NATIVE_COVERAGE_ENTRY").ok_or(preparation)?;
    let evidence = std::env::var_os("DEVUP_NATIVE_COVERAGE_EVIDENCE").ok_or(preparation)?;
    let host = std::env::var_os("DEVUP_NATIVE_COVERAGE_HOST").ok_or(preparation)?;
    let result = Command::new(node)
        .args([host, entry, evidence])
        .env("NODE_OPTIONS", "")
        .env("BUN_OPTIONS", "")
        .status()?;
    assert!(result.success(), "real Node coverage host failed: {result}");
    Ok(())
}
