use dsc::config::DiscourseConfig;
use dsc::api::DiscourseClient;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

pub const DEFAULT_TEST_CONFIG: &str = "testdsc.toml";
pub const FALLBACK_TEST_CONFIG: &str = "test-dsc.toml";

pub fn verbose_enabled() -> bool {
    std::env::var("DSC_TEST_VERBOSE")
        .or_else(|_| std::env::var("TEST_VERBOSE"))
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or_else(|_| std::env::args().any(|arg| arg == "-v" || arg == "--verbose"))
}

pub fn vprintln(message: &str) {
    if verbose_enabled() {
        eprintln!("[e2e] {}", message);
    }
}

#[derive(Debug, Deserialize)]
struct TestConfig {
    #[serde(default)]
    discourse: Vec<TestDiscourse>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TestDiscourse {
    pub name: String,
    pub baseurl: String,
    pub apikey: String,
    pub api_username: String,
    pub changelog_topic_id: Option<u64>,
    pub ssh_host: Option<String>,
    pub test_topic_id: Option<u64>,
    pub test_category_id: Option<u64>,
    pub test_color_scheme_id: Option<u64>,
    pub test_group_id: Option<u64>,
    pub ssh_enabled: Option<bool>,
    pub emoji_path: Option<String>,
    pub emoji_name: Option<String>,
    pub test_plugin_url: Option<String>,
    pub test_plugin_name: Option<String>,
    pub test_theme_url: Option<String>,
    pub test_theme_name: Option<String>,
    pub backup_enabled: Option<bool>,
    pub test_backup_path: Option<String>,
}

fn load_test_config() -> Option<TestConfig> {
    let path = match std::env::var("TEST_DSC_CONFIG") {
        Ok(path) => path,
        Err(_) => {
            if Path::new(DEFAULT_TEST_CONFIG).exists() {
                DEFAULT_TEST_CONFIG.to_string()
            } else if Path::new(FALLBACK_TEST_CONFIG).exists() {
                FALLBACK_TEST_CONFIG.to_string()
            } else {
                return None;
            }
        }
    };
    let raw = fs::read_to_string(path).ok()?;
    toml::from_str(&raw).ok()
}

pub fn test_discourse() -> Option<TestDiscourse> {
    load_test_config()?.discourse.into_iter().next()
}

pub fn test_discourse_pair() -> Option<(TestDiscourse, TestDiscourse)> {
    let mut discourses = load_test_config()?.discourse.into_iter();
    let source = discourses.next()?;
    let target = discourses.next()?;
    Some((source, target))
}

pub fn to_config(d: &TestDiscourse) -> DiscourseConfig {
    DiscourseConfig {
        name: d.name.clone(),
        baseurl: d.baseurl.clone(),
        apikey: Some(d.apikey.clone()),
        api_username: Some(d.api_username.clone()),
        changelog_topic_id: d.changelog_topic_id,
        ssh_host: d.ssh_host.clone(),
        ..DiscourseConfig::default()
    }
}

pub fn post_and_verify(d: &TestDiscourse, topic_id: u64, marker: &str) {
    let config = to_config(d);
    let client = DiscourseClient::new(&config).expect("client");
    let body = format!("e2e marker: {}", marker);
    vprintln(&format!(
        "posting marker to topic {} on {}",
        topic_id, d.name
    ));
    client.create_post(topic_id, &body).expect("post");
    vprintln(&format!("verifying marker on topic {}", topic_id));
    let topic = client.fetch_topic(topic_id, true).expect("fetch topic");
    let found = topic.post_stream.posts.iter().any(|post| {
        post.raw
            .as_ref()
            .map(|raw| raw.contains(marker))
            .unwrap_or(false)
    });
    assert!(found, "marker not found on forum");
}

pub fn run_dsc(args: &[&str], config_path: &Path) -> std::process::Output {
    vprintln(&format!("running dsc {}", args.join(" ")));
    Command::new(env!("CARGO_BIN_EXE_dsc"))
        .arg("-c")
        .arg(config_path)
        .args(args)
        .output()
        .expect("run dsc")
}

pub fn write_temp_config(dir: &TempDir, content: &str) -> PathBuf {
    let path = dir.path().join("dsc.toml");
    fs::write(&path, content).expect("write config");
    vprintln(&format!("wrote temp config {}", path.display()));
    path
}
