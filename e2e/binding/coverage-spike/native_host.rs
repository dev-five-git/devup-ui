use std::{env, error::Error, path::Path, process::Command};

#[test]
fn scenarios_succeed_when_loaded_by_current_host() -> Result<(), Box<dyn Error>> {
    // Given: caller-selected Node/Bun, addon and control; no addon linkage here.
    let host = env::var_os("W74A_HOST").ok_or("W74A_HOST must select current Node or Bun")?;
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("host.cjs");
    // When: inherit the complete environment, including LLVM_PROFILE_FILE.
    let status = Command::new(host).arg(script).status()?;
    // Then: status() waits for natural exit and JS assertions must succeed.
    assert!(status.success(), "W74a JS host failed: {status}");
    Ok(())
}
