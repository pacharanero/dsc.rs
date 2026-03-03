use crate::api::DiscourseClient;
use crate::cli::ListFormat;
use crate::commands::common::{ensure_api_credentials, select_discourse};
use crate::commands::update::run_ssh_command;
use crate::config::{Config, DiscourseConfig};
use anyhow::{Result, anyhow};
use serde::Serialize;

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
