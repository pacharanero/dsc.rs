mod common;
use common::*;
use tempfile::TempDir;

#[test]
fn list() {
    vprintln("e2e_list: listing discourses");
    let dir = TempDir::new().expect("tempdir");
    let config_path = write_temp_config(
        &dir,
        "[[discourse]]\nname = \"local\"\nbaseurl = \"https://example.com\"\n",
    );
    let output = run_dsc(&["list", "-f", "json"], &config_path);
    assert!(output.status.success(), "list failed");
}