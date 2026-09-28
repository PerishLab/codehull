use std::process::Command;

#[test]
fn stamp() {
    let output = Command::new(env!("CARGO_BIN_EXE_codehull-api"))
        .arg("--version")
        .output()
        .expect("version process");
    assert!(output.status.success());
    let line = String::from_utf8(output.stdout).expect("utf8");
    if option_env!("CODEHULL_BUILD_CHANNEL") == Some("unbound") {
        assert_eq!(line.trim(), "codehull-api unbound");
        let refused = Command::new(env!("CARGO_BIN_EXE_codehull-api"))
            .arg("serve")
            .output()
            .expect("serve process");
        assert!(!refused.status.success());
        assert!(String::from_utf8_lossy(&refused.stderr).contains("unbound build"));
        return;
    }
    let marker = line
        .trim_end()
        .strip_prefix("codehull-api v")
        .expect("binary name");
    assert!(!marker.is_empty() && !marker.contains(' '), "{line:?}");
}
