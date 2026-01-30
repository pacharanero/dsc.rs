mod common;
use common::*;
use dsc::discourse::DiscourseClient;
use std::process::Command;
use std::time::Duration;
use tempfile::TempDir;
use uuid::Uuid;

#[test]
fn update() {
    let Some(test) = test_discourse() else {
        return;
    };
    if test.ssh_enabled != Some(true) {
        return;
    }
    vprintln("e2e_update: update + changelog post");
    let Some(topic_id) = test.changelog_topic_id else {
        return;
    };
    let marker = Uuid::new_v4().to_string();
    let ssh_host_line = test
        .ssh_host
        .as_ref()
        .map(|host| format!("ssh_host = \"{}\"\n", host))
        .unwrap_or_default();
    let dir = TempDir::new().expect("tempdir");
    let config_path = write_temp_config(
        &dir,
        &format!(
            "[[discourse]]\nname = \"{}\"\nbaseurl = \"{}\"\napikey = \"{}\"\napi_username = \"{}\"\n{}changelog_topic_id = {}\n",
            test.name, test.baseurl, test.apikey, test.api_username, ssh_host_line, topic_id
        ),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_dsc"))
        .arg("-c")
        .arg(&config_path)
        .arg("update")
        .arg(&test.name)
        .arg("-p")
        .env("DSC_TEST_MARKER", &marker)
        .env("DSC_SSH_UPDATE_CMD", "echo update-ok")
        .env("DSC_SSH_CLEANUP_CMD", "echo Total reclaimed space: 0B")
        .env("DSC_SSH_OS_UPDATE_CMD", "echo OS packages updated")
        .env("DSC_SSH_REBOOT_CMD", "echo Server rebooted")
        .env("DSC_SSH_OS_VERSION_CMD", "echo Ubuntu 22.04.3 LTS")
        .output()
        .expect("run update");
    if !output.status.success() {
        panic!(
            "update failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let client = DiscourseClient::new(&to_config(&test)).expect("client");
    let post_id = extract_test_post_id(&output);
    let found = if let Some(post_id) = post_id {
        wait_for_post_marker(&client, post_id, &marker)
    } else {
        wait_for_marker(&client, topic_id, &marker)
    };
    assert!(found, "marker not found on changelog");
}

#[test]
fn update_all() {
    let Some(test) = test_discourse() else {
        return;
    };
    if test.ssh_enabled != Some(true) {
        return;
    }
    vprintln("e2e_update_all: update all + changelog post");
    let Some(topic_id) = test.changelog_topic_id else {
        return;
    };
    let marker = Uuid::new_v4().to_string();
    let ssh_host_line = test
        .ssh_host
        .as_ref()
        .map(|host| format!("ssh_host = \"{}\"\n", host))
        .unwrap_or_default();
    let dir = TempDir::new().expect("tempdir");
    let config_path = write_temp_config(
        &dir,
        &format!(
            "[[discourse]]\nname = \"{}\"\nbaseurl = \"{}\"\napikey = \"{}\"\napi_username = \"{}\"\n{}changelog_topic_id = {}\n",
            test.name, test.baseurl, test.apikey, test.api_username, ssh_host_line, topic_id
        ),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_dsc"))
        .arg("-c")
        .arg(&config_path)
        .arg("update")
        .arg("all")
        .arg("-p")
        .env("DSC_TEST_MARKER", &marker)
        .env("DSC_SSH_UPDATE_CMD", "echo update-ok")
        .env("DSC_SSH_CLEANUP_CMD", "echo Total reclaimed space: 0B")
        .env("DSC_SSH_OS_UPDATE_CMD", "echo OS packages updated")
        .env("DSC_SSH_REBOOT_CMD", "echo Server rebooted")
        .env("DSC_SSH_OS_VERSION_CMD", "echo Ubuntu 22.04.3 LTS")
        .output()
        .expect("run update all");
    if !output.status.success() {
        panic!(
            "update all failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let client = DiscourseClient::new(&to_config(&test)).expect("client");
    let post_id = extract_test_post_id(&output);
    let found = if let Some(post_id) = post_id {
        wait_for_post_marker(&client, post_id, &marker)
    } else {
        wait_for_marker(&client, topic_id, &marker)
    };
    assert!(found, "marker not found on changelog");
}

fn wait_for_marker(client: &DiscourseClient, topic_id: u64, marker: &str) -> bool {
    let max_attempts = 10;
    for _ in 0..max_attempts {
        if let Ok(topic) = client.fetch_topic(topic_id, true) {
            let found = topic.post_stream.posts.iter().any(|post| {
                post.raw
                    .as_ref()
                    .map(|raw| raw.contains(marker))
                    .unwrap_or(false)
            });
            if found {
                return true;
            }
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    false
}

fn wait_for_post_marker(client: &DiscourseClient, post_id: u64, marker: &str) -> bool {
    let max_attempts = 10;
    for _ in 0..max_attempts {
        if let Ok(Some(raw)) = client.fetch_post_raw(post_id) {
            if raw.contains(marker) {
                return true;
            }
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    false
}

fn extract_test_post_id(output: &std::process::Output) -> Option<u64> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().find_map(|line| {
        line.strip_prefix("DSC_TEST_POST_ID=")
            .and_then(|value| value.parse::<u64>().ok())
    })
}
