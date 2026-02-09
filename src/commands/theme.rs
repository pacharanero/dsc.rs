use crate::commands::common::{ensure_api_credentials, select_discourse};
use crate::commands::update::run_ssh_command;
use crate::config::{Config, DiscourseConfig};
use crate::api::DiscourseClient;
use anyhow::{anyhow, Result};

pub fn theme_list(config: &Config, discourse_name: &str) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;
    let response = client.list_themes()?;
    let themes = response
        .get("themes")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    for theme in themes {
        let id = theme.get("id").and_then(|v| v.as_u64()).unwrap_or_default();
        let name = theme
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let enabled = theme
            .get("enabled")
            .and_then(|v| v.as_bool())
            .map(|value| if value { "enabled" } else { "disabled" })
            .unwrap_or("unknown");
        println!("{} - {} - {}", id, name, enabled);
    }
    Ok(())
}

pub fn theme_install(config: &Config, discourse_name: &str, url: &str) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    let target = ssh_target(discourse);
    let template = std::env::var("DSC_SSH_THEME_INSTALL_CMD")
        .map_err(|_| anyhow!("DSC_SSH_THEME_INSTALL_CMD is required"))?;
    let command = render_template(&template, &[("url", url), ("name", url)]);
    let output = run_ssh_command(&target, &command)?;
    if !output.trim().is_empty() {
        println!("{}", output.trim());
    }
    Ok(())
}

pub fn theme_remove(config: &Config, discourse_name: &str, name: &str) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    let target = ssh_target(discourse);
    let template = std::env::var("DSC_SSH_THEME_REMOVE_CMD")
        .map_err(|_| anyhow!("DSC_SSH_THEME_REMOVE_CMD is required"))?;
    let command = render_template(&template, &[("name", name), ("url", name)]);
    let output = run_ssh_command(&target, &command)?;
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
