use crate::commands::common::{ensure_api_credentials, select_discourse};
use crate::commands::update::run_ssh_command;
use crate::config::{Config, DiscourseConfig};
use crate::discourse::DiscourseClient;
use anyhow::{anyhow, Result};

pub fn plugin_list(config: &Config, discourse_name: &str) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;
    let response = client.list_plugins()?;
    let plugins = response
        .get("plugins")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    for plugin in plugins {
        let name = plugin
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let version = plugin
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let enabled = plugin
            .get("enabled")
            .and_then(|v| v.as_bool())
            .or_else(|| plugin.get("active").and_then(|v| v.as_bool()))
            .map(|value| if value { "enabled" } else { "disabled" })
            .unwrap_or("unknown");
        println!("{} - {} - {}", name, version, enabled);
    }
    Ok(())
}

pub fn plugin_install(config: &Config, discourse_name: &str, url: &str) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    let target = ssh_target(discourse);
    let template = std::env::var("DSC_SSH_PLUGIN_INSTALL_CMD")
        .map_err(|_| anyhow!("DSC_SSH_PLUGIN_INSTALL_CMD is required"))?;
    let command = render_template(&template, &[("url", url), ("name", url)]);
    let output = run_ssh_command(&target, &command)?;
    if !output.trim().is_empty() {
        println!("{}", output.trim());
    }
    Ok(())
}

pub fn plugin_remove(config: &Config, discourse_name: &str, name: &str) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    let target = ssh_target(discourse);
    let template = std::env::var("DSC_SSH_PLUGIN_REMOVE_CMD")
        .map_err(|_| anyhow!("DSC_SSH_PLUGIN_REMOVE_CMD is required"))?;
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
