use std::{fs, process::Command};

// Compiler and clap diagnostics change between releases, so the snapshots only hold on the
// toolchain they were recorded with. The published crate has no `rust-toolchain.toml`, so the
// snapshots are skipped there too.
#[test]
fn derive_diagnostics() {
    let Ok(toolchain) =
        fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/rust-toolchain.toml"))
    else {
        eprintln!("skipping compile-fail tests: no rust-toolchain.toml");

        return;
    };
    let channel = toolchain
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
