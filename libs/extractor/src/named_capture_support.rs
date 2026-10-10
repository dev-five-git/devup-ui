pub(super) fn evaluate(source: &str) -> String {
    let source = serde_json::to_string(source).unwrap_or_else(|error| panic!("{error}"));
    let script = format!(
        "Promise.resolve(eval({source})).then(value=>process.stdout.write(String(value)),error=>{{console.error(error);process.exitCode=1}})"
    );
    let output = std::process::Command::new("node")
        .args(["-e", &script])
        .output()
        .unwrap_or_else(|error| panic!("native Node observer: {error}"));
    assert!(
        output.status.success(),
        "{}\n{script}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap_or_else(|error| panic!("{error}"))
}
