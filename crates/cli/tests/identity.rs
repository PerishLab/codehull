use std::process::Command;

#[test]
fn unbound() {
    let binary = env!("CARGO_BIN_EXE_codehull");
    let version = Command::new(binary).arg("--version").output().unwrap();
    assert!(version.status.success());
    let line = String::from_utf8(version.stdout).unwrap();
    if option_env!("CODEHULL_BUILD_CHANNEL") != Some("unbound") {
        assert!(line.starts_with("codehull v"), "{line:?}");
        return;
    }
    assert_eq!(line.trim(), "codehull unbound");
    let refused = Command::new(binary).arg("act").output().unwrap();
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("unbound build"));
}
