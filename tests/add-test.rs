mod common;
use common::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn add() {
    vprintln("e2e_add: adding discourse");
    let dir = TempDir::new().expect("tempdir");
    let config_path = write_temp_config(&dir, "");
    let output = run_dsc(&["add", "newforum"], &config_path);
    assert!(output.status.success(), "add failed");
    let raw = fs::read_to_string(config_path).expect("read config");
    assert!(raw.contains("newforum"));
}