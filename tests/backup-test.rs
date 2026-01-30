mod common;
use common::*;
use tempfile::TempDir;

#[test]
fn backup_list() {
    let Some(test) = test_discourse() else {
        return;
    };
    if test.backup_enabled != Some(true) {
        return;
    }
    vprintln("e2e_backup_list: listing backups");
    let dir = TempDir::new().expect("tempdir");
    let config_path = write_temp_config(
        &dir,
        &format!(
            "[[discourse]]\nname = \"{}\"\nbaseurl = \"{}\"\napikey = \"{}\"\napi_username = \"{}\"\n",
            test.name, test.baseurl, test.apikey, test.api_username
        ),
    );
    let output = run_dsc(&["backup", "list", "--discourse", &test.name], &config_path);
    assert!(output.status.success(), "backup list failed");
}

#[test]
fn backup_create() {
    let Some(test) = test_discourse() else {
        return;
    };
    if test.backup_enabled != Some(true) {
        return;
    }
    vprintln("e2e_backup_create: creating backup");
    let dir = TempDir::new().expect("tempdir");
    let config_path = write_temp_config(
        &dir,
        &format!(
            "[[discourse]]\nname = \"{}\"\nbaseurl = \"{}\"\napikey = \"{}\"\napi_username = \"{}\"\n",
            test.name, test.baseurl, test.apikey, test.api_username
        ),
    );
    let output = run_dsc(
        &["backup", "create", "--discourse", &test.name],
        &config_path,
    );
    assert!(output.status.success(), "backup create failed");
}

#[test]
fn backup_restore() {
    let Some(test) = test_discourse() else {
        return;
    };
    if test.backup_enabled != Some(true) {
        return;
    }
    vprintln("e2e_backup_restore: restoring backup");
    let Some(backup_path) = test.test_backup_path.as_ref() else {
        return;
    };
    let dir = TempDir::new().expect("tempdir");
    let config_path = write_temp_config(
        &dir,
        &format!(
            "[[discourse]]\nname = \"{}\"\nbaseurl = \"{}\"\napikey = \"{}\"\napi_username = \"{}\"\n",
            test.name, test.baseurl, test.apikey, test.api_username
        ),
    );
    let output = run_dsc(
        &["backup", "restore", "--discourse", &test.name, backup_path],
        &config_path,
    );
    assert!(output.status.success(), "backup restore failed");
}