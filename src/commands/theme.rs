use crate::api::DiscourseClient;
use crate::cli::ListFormat;
use crate::commands::common::{ensure_api_credentials, select_discourse};
use crate::commands::update::run_ssh_command;
use crate::config::{Config, DiscourseConfig};
use crate::utils::slugify;
use anyhow::{Context, Result, anyhow};
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Serialize)]
struct ThemeListEntry {
    id: u64,
    name: String,
    status: String,
}

pub fn theme_list(
    config: &Config,
    discourse_name: &str,
    format: ListFormat,
    verbose: bool,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;
    let response = client.list_themes()?;
    let themes = response
        .get("themes")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let entries: Vec<ThemeListEntry> = themes
        .into_iter()
        .map(|theme| {
            let id = theme.get("id").and_then(|v| v.as_u64()).unwrap_or_default();
            let name = theme
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let status = theme
                .get("enabled")
                .and_then(|v| v.as_bool())
                .map(|value| {
                    if value {
                        "enabled".to_string()
                    } else {
                        "disabled".to_string()
                    }
                })
                .unwrap_or_else(|| "unknown".to_string());
            ThemeListEntry { id, name, status }
        })
        .collect();

    match format {
        ListFormat::Text => {
            if entries.is_empty() && !verbose {
                println!("No themes found.");
                return Ok(());
            }
            for theme in entries {
                println!("{} - {} - {}", theme.id, theme.name, theme.status);
            }
        }
        ListFormat::Json => {
            let raw = serde_json::to_string_pretty(&entries)?;
            println!("{}", raw);
        }
        ListFormat::Yaml => {
            let raw = serde_yaml::to_string(&entries)?;
            println!("{}", raw);
        }
    }
    Ok(())
}

pub fn theme_install(config: &Config, discourse_name: &str, url: &str) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    let target = ssh_target(discourse);
    let template = std::env::var("DSC_SSH_THEME_INSTALL_CMD")
        .map_err(|_| {
            anyhow!(
                "missing DSC_SSH_THEME_INSTALL_CMD for theme install; set DSC_SSH_THEME_INSTALL_CMD to your install command"
            )
        })?;
    let command = render_template(&template, &[("url", url), ("name", url)]);
    let output = run_ssh_command(&target, &command)?;
    println!("Theme install completed: {}", url);
    if !output.trim().is_empty() {
        println!("{}", output.trim());
    }
    Ok(())
}

pub fn theme_remove(config: &Config, discourse_name: &str, name: &str) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    let target = ssh_target(discourse);
    let template = std::env::var("DSC_SSH_THEME_REMOVE_CMD")
        .map_err(|_| {
            anyhow!(
                "missing DSC_SSH_THEME_REMOVE_CMD for theme remove; set DSC_SSH_THEME_REMOVE_CMD to your remove command"
            )
        })?;
    let command = render_template(&template, &[("name", name), ("url", name)]);
    let output = run_ssh_command(&target, &command)?;
    println!("Theme removal completed: {}", name);
    if !output.trim().is_empty() {
        println!("{}", output.trim());
    }
    Ok(())
}

/// Pull a theme to a local JSON file.
pub fn theme_pull(
    config: &Config,
    discourse_name: &str,
    theme_id: u64,
    local_path: Option<&Path>,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;
    let response = client.fetch_theme(theme_id)?;

    // Unwrap {"theme": {...}} envelope if present
    let theme = response.get("theme").unwrap_or(&response);

    let path = match local_path {
        Some(p) => p.to_path_buf(),
        None => {
            let name_slug = theme
                .get("name")
                .and_then(|v| v.as_str())
                .map(slugify)
                .unwrap_or_else(|| format!("theme-{}", theme_id));
            let filename = format!("{}.json", name_slug);
            std::env::current_dir()
                .context("getting current directory")?
                .join(filename)
        }
    };

    let content =
        serde_json::to_string_pretty(theme).context("serializing theme to JSON")?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
    }
    std::fs::write(&path, content)
        .with_context(|| format!("writing {}", path.display()))?;
    println!("{}", path.display());
    Ok(())
}

/// Push a local JSON file to create or update a theme.
pub fn theme_push(
    config: &Config,
    discourse_name: &str,
    json_path: &Path,
    theme_id: Option<u64>,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;

    let raw = std::fs::read_to_string(json_path)
        .with_context(|| format!("reading {}", json_path.display()))?;
    let parsed: Value = serde_json::from_str(&raw)
        .with_context(|| format!("parsing JSON from {}", json_path.display()))?;

    // Unwrap {"theme": {...}} envelope if present
    let theme = if let Some(inner) = parsed.get("theme") {
        inner.clone()
    } else {
        parsed
    };

    let push_data = build_push_payload(&theme);

    let target_id = theme_id.or_else(|| theme.get("id").and_then(|v| v.as_u64()));

    if let Some(id) = target_id {
        client.update_theme(id, &push_data)?;
        println!("{}", id);
    } else {
        if push_data
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        {
            return Err(anyhow!(
                "missing name in theme file; set name or pass a theme ID to update"
            ));
        }
        let new_id = client.create_theme(&push_data)?;
        println!("{}", new_id);
    }

    Ok(())
}

/// Duplicate a theme and print the new theme ID.
pub fn theme_duplicate(
    config: &Config,
    discourse_name: &str,
    theme_id: u64,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;

    let response = client.fetch_theme(theme_id)?;
    let theme = response.get("theme").unwrap_or(&response);

    let original_name = theme
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");
    let new_name = format!("Copy of {}", original_name);

    let mut push_data = build_push_payload(theme);
    push_data["name"] = Value::String(new_name);
    // Never copy the default status to the duplicate
    push_data["default"] = Value::Bool(false);

    let new_id = client.create_theme(&push_data)?;
    println!("{}", new_id);
    Ok(())
}

/// Build a payload suitable for creating or updating a theme.
/// Strips server-generated and read-only fields.
fn build_push_payload(theme: &Value) -> Value {
    let mut map = serde_json::Map::new();
    for key in &[
        "name",
        "enabled",
        "user_selectable",
        "color_scheme_id",
        "theme_fields",
        "component",
    ] {
        if let Some(val) = theme.get(key) {
            map.insert(key.to_string(), val.clone());
        }
    }
    Value::Object(map)
}

fn ssh_target(discourse: &DiscourseConfig) -> String {
    discourse
        .ssh_host
        .clone()
        .unwrap_or_else(|| discourse.name.clone())
}

fn render_template(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (key, value) in replacements {
        out = out.replace(&format!("{{{}}}", key), value);
    }
    out
}
