mod common;
use common::*;
use dsc::api::DiscourseClient;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn theme_list() {
    let Some(test) = test_discourse() else {
        return;
    };
    vprintln("e2e_theme_list: listing themes");
    let dir = TempDir::new().expect("tempdir");
    let config_path = write_temp_config(
        &dir,
        &format!(
            "[[discourse]]\nname = \"{}\"\nbaseurl = \"{}\"\napikey = \"{}\"\napi_username = \"{}\"\n",
            test.name, test.baseurl, test.apikey, test.api_username
        ),
    );
    let output = run_dsc(&["theme", "list", &test.name], &config_path);
    assert!(output.status.success(), "theme list failed");
}

#[test]
fn theme_install_remove() {
    let Some(test) = test_discourse() else {
        return;
    };
    if test.ssh_enabled != Some(true) {
        return;
    }
    let Some(url) = test.test_theme_url.as_ref() else {
        return;
    };
    let Some(name) = test.test_theme_name.as_ref() else {
        return;
    };
    vprintln("e2e_theme_install_remove: install/remove theme");
    let ssh_host_line = test
        .ssh_host
        .as_ref()
        .map(|host| format!("ssh_host = \"{}\"\n", host))
        .unwrap_or_default();
    let dir = TempDir::new().expect("tempdir");
    let config_path = write_temp_config(
        &dir,
        &format!(
            "[[discourse]]\nname = \"{}\"\nbaseurl = \"{}\"\napikey = \"{}\"\napi_username = \"{}\"\n{}",
            test.name, test.baseurl, test.apikey, test.api_username, ssh_host_line
        ),
    );

    let output = Command::new(env!("CARGO_BIN_EXE_dsc"))
        .arg("-c")
        .arg(&config_path)
        .arg("theme")
        .arg("install")
        .arg(&test.name)
        .arg(url)
        .env("DSC_SSH_THEME_INSTALL_CMD", "echo theme install {url}")
        .output()
        .expect("run theme install");
    assert!(output.status.success(), "theme install failed");

    let output = Command::new(env!("CARGO_BIN_EXE_dsc"))
        .arg("-c")
        .arg(&config_path)
        .arg("theme")
        .arg("remove")
        .arg(&test.name)
        .arg(name)
        .env("DSC_SSH_THEME_REMOVE_CMD", "echo theme remove {name}")
        .output()
        .expect("run theme remove");
    assert!(output.status.success(), "theme remove failed");
}

#[test]
fn theme_pull_push() {
    let Some(test) = test_discourse() else {
        return;
    };
    let Some(theme_id) = test.test_theme_id else {
        return;
    };
    vprintln("e2e_theme_pull_push: pull theme then push back");
    let dir = TempDir::new().expect("tempdir");
    let config_path = write_temp_config(
        &dir,
        &format!(
            "[[discourse]]\nname = \"{}\"\nbaseurl = \"{}\"\napikey = \"{}\"\napi_username = \"{}\"\n",
            test.name, test.baseurl, test.apikey, test.api_username
        ),
    );

    // Pull the theme to a file
    let json_path = dir.path().join("pulled-theme.json");
    let output = run_dsc(
        &[
            "theme",
            "pull",
            &test.name,
            &theme_id.to_string(),
            json_path.to_str().unwrap(),
        ],
        &config_path,
    );
    assert!(
        output.status.success(),
        "theme pull failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(json_path.exists(), "pulled theme file not created");

    let raw = std::fs::read_to_string(&json_path).expect("read pulled theme");
    let parsed: serde_json::Value = serde_json::from_str(&raw).expect("parse pulled theme");
    assert!(
        parsed.get("name").is_some(),
        "pulled theme JSON missing 'name'"
    );

    // Push back to the same theme ID (round-trip update)
    let output = run_dsc(
        &[
            "theme",
            "push",
            &test.name,
            json_path.to_str().unwrap(),
            &theme_id.to_string(),
        ],
        &config_path,
    );
    assert!(
        output.status.success(),
        "theme push failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let returned_id: u64 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .expect("theme push should print numeric ID");
    assert_eq!(returned_id, theme_id, "push should return the updated theme ID");
}

#[test]
fn theme_duplicate() {
    let Some(test) = test_discourse() else {
        return;
    };
    let Some(theme_id) = test.test_theme_id else {
        return;
    };
    vprintln("e2e_theme_duplicate: duplicate a theme");
    let dir = TempDir::new().expect("tempdir");
    let config_path = write_temp_config(
        &dir,
        &format!(
            "[[discourse]]\nname = \"{}\"\nbaseurl = \"{}\"\napikey = \"{}\"\napi_username = \"{}\"\n",
            test.name, test.baseurl, test.apikey, test.api_username
        ),
    );

    let output = run_dsc(
        &["theme", "duplicate", &test.name, &theme_id.to_string()],
        &config_path,
    );
    assert!(
        output.status.success(),
        "theme duplicate failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let new_id_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let new_id: u64 = new_id_str
        .parse()
        .expect("theme duplicate should print numeric ID");
    vprintln(&format!("duplicated theme, new ID: {}", new_id));
    assert_ne!(new_id, theme_id, "duplicate should have a different ID");

    // Clean up the duplicated theme
    let client = DiscourseClient::new(&to_config(&test)).expect("client");
    client
        .delete_theme(new_id)
        .expect("failed to delete duplicate theme during cleanup");
    vprintln(&format!("cleaned up duplicate theme {}", new_id));
}
