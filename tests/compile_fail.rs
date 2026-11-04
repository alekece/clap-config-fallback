use std::process::Command;

const TOOLCHAIN: &str = include_str!("../rust-toolchain.toml");

// Compiler and clap diagnostics change between releases, so the snapshots only hold on the
// toolchain they were recorded with.
#[test]
fn derive_diagnostics() {
    let channel = TOOLCHAIN
        .lines()
        .find_map(|line| line.strip_prefix("channel = "))
        .expect("rust-toolchain.toml sets a channel")
        .trim_matches('"');
    let output = Command::new("rustc")
        .arg("--version")
        .output()
        .expect("rustc is available");
    let version = String::from_utf8_lossy(&output.stdout);
    let version = version.trim();

    if !version.starts_with(&format!("rustc {channel} ")) {
        eprintln!("skipping compile-fail tests: {version} is not the pinned {channel}");

        return;
    }

    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}
