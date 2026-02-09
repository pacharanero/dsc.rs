mod common;
use common::*;
use dsc::api::DiscourseClient;
use std::fs;
use tempfile::TempDir;
use uuid::Uuid;

#[test]
fn topic_pull() {
    let Some(test) = test_discourse() else {
        return;
    };
    let Some(topic_id) = test.test_topic_id else {
        return;
    };
    let marker = Uuid::new_v4().to_string();
    vprintln("e2e_topic_pull: post marker, then pull topic");
    post_and_verify(&test, topic_id, &marker);

    let dir = TempDir::new().expect("tempdir");
    let config_path = write_temp_config(
        &dir,
        &format!(
            "[[discourse]]\nname = \"{}\"\nbaseurl = \"{}\"\napikey = \"{}\"\napi_username = \"{}\"\n",
            test.name, test.baseurl, test.apikey, test.api_username
        ),
    );
    let output = run_dsc(
        &[
            "topic",
            "pull",
            &test.name,
            &topic_id.to_string(),
            dir.path().to_str().unwrap(),
        ],
        &config_path,
    );
    assert!(output.status.success(), "topic pull failed");
}

#[test]
fn topic_push() {
    let Some(test) = test_discourse() else {
        return;
    };
    let Some(topic_id) = test.test_topic_id else {
        return;
    };
    let marker = Uuid::new_v4().to_string();
    vprintln("e2e_topic_push: write file, then push topic");
    let dir = TempDir::new().expect("tempdir");
    let file_path = dir.path().join("push.md");
    fs::write(&file_path, format!("# E2E Push\n\n{}", marker)).expect("write file");

    let config_path = write_temp_config(
        &dir,
        &format!(
            "[[discourse]]\nname = \"{}\"\nbaseurl = \"{}\"\napikey = \"{}\"\napi_username = \"{}\"\n",
            test.name, test.baseurl, test.apikey, test.api_username
        ),
    );
    let output = run_dsc(
        &[
            "topic",
            "push",
            &test.name,
            file_path.to_str().unwrap(),
            &topic_id.to_string(),
        ],
        &config_path,
    );
    assert!(output.status.success(), "topic push failed");
    let config = to_config(&test);
    let client = DiscourseClient::new(&config).expect("client");
    let topic = client.fetch_topic(topic_id, true).expect("topic");
    let found = topic.post_stream.posts.iter().any(|post| {
        post.raw
            .as_ref()
            .map(|raw| raw.contains(&marker))
            .unwrap_or(false)
    });
    assert!(found, "marker not found after push");
}

#[test]
fn topic_sync() {
    let Some(test) = test_discourse() else {
        return;
    };
    let Some(topic_id) = test.test_topic_id else {
        return;
    };
    let marker = Uuid::new_v4().to_string();
    vprintln("e2e_topic_sync: write file, then sync");
    let dir = TempDir::new().expect("tempdir");
    let file_path = dir.path().join("sync.md");
    fs::write(&file_path, format!("# E2E Sync\n\n{}", marker)).expect("write file");

    let config_path = write_temp_config(
        &dir,
        &format!(
            "[[discourse]]\nname = \"{}\"\nbaseurl = \"{}\"\napikey = \"{}\"\napi_username = \"{}\"\n",
            test.name, test.baseurl, test.apikey, test.api_username
        ),
    );
    let output = run_dsc(
        &[
            "topic",
            "sync",
            &test.name,
            &topic_id.to_string(),
            file_path.to_str().unwrap(),
            "--yes",
        ],
        &config_path,
    );
    assert!(output.status.success(), "topic sync failed");
    let config = to_config(&test);
    let client = DiscourseClient::new(&config).expect("client");
    let topic = client.fetch_topic(topic_id, true).expect("topic");
    let found = topic.post_stream.posts.iter().any(|post| {
        post.raw
            .as_ref()
            .map(|raw| raw.contains(&marker))
            .unwrap_or(false)
    });
    assert!(found, "marker not found after sync");
}
