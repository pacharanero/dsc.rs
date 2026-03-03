use crate::api::DiscourseClient;
use crate::api::GroupSummary;
use crate::cli::{ListFormat, StructuredFormat};
use crate::commands::common::{ensure_api_credentials, select_discourse};
use crate::config::Config;
use crate::utils::slugify;
use anyhow::{Result, anyhow};

pub fn group_list(
    config: &Config,
    discourse_name: &str,
    format: ListFormat,
    verbose: bool,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;
    let groups = client.fetch_groups()?;
    match format {
        ListFormat::Text => {
            if groups.is_empty() && !verbose {
                println!("No groups found.");
                return Ok(());
            }
            for group in groups {
                let full_name = group.full_name.unwrap_or_else(|| "-".to_string());
                println!("{} - {} ({})", group.id, group.name, full_name);
            }
        }
        ListFormat::Json => {
            let raw = serde_json::to_string_pretty(&groups)?;
            println!("{}", raw);
        }
        ListFormat::Yaml => {
            let raw = serde_yaml::to_string(&groups)?;
            println!("{}", raw);
        }
    }
    Ok(())
}

pub fn group_info(
    config: &Config,
    discourse_name: &str,
    group_id: u64,
    format: StructuredFormat,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;
    let group_summary = find_group_summary(&client, group_id)?;
    let group = client.fetch_group_detail(group_summary.id, Some(&group_summary.name))?;
    match format {
        StructuredFormat::Json => {
            let raw = serde_json::to_string_pretty(&group)?;
            println!("{}", raw);
        }
        StructuredFormat::Yaml => {
            let raw = serde_yaml::to_string(&group)?;
            println!("{}", raw);
        }
    }
    Ok(())
}

pub fn group_members(
    config: &Config,
    discourse_name: &str,
    group_id: u64,
    format: ListFormat,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;
    let group_summary = find_group_summary(&client, group_id)?;
    let members = client.fetch_group_members(group_summary.id, Some(&group_summary.name))?;
    match format {
        ListFormat::Text => {
            if members.is_empty() {
                println!("No group members found.");
                return Ok(());
            }
            for member in members {
                let name = member.name.unwrap_or_else(|| "-".to_string());
                println!("{} - {} ({})", member.id, member.username, name);
            }
        }
        ListFormat::Json => {
            let raw = serde_json::to_string_pretty(&members)?;
            println!("{}", raw);
        }
        ListFormat::Yaml => {
            let raw = serde_yaml::to_string(&members)?;
            println!("{}", raw);
        }
    }
    Ok(())
}

pub fn group_copy(
    config: &Config,
    source: &str,
    target: Option<&str>,
    group_id: u64,
) -> Result<()> {
    let source_discourse = select_discourse(config, Some(source))?;
    let target_discourse_name = target.unwrap_or(source);
    let target_discourse = select_discourse(config, Some(target_discourse_name))?;

    ensure_api_credentials(source_discourse)?;
    ensure_api_credentials(target_discourse)?;

    let source_client = DiscourseClient::new(source_discourse)?;
    let group_summary = find_group_summary(&source_client, group_id)?;
    let mut group =
        source_client.fetch_group_detail(group_summary.id, Some(&group_summary.name))?;
    group.name = format!("{}-copy", slugify(&group.name));
    if let Some(full_name) = group.full_name.clone() {
        group.full_name = Some(format!("Copy of {}", full_name));
    }

    let target_client = DiscourseClient::new(target_discourse)?;
    let new_id = target_client.create_group(&group)?;
    println!("Group copied successfully with new ID: {}", new_id);
    Ok(())
}

fn find_group_summary(client: &DiscourseClient, group_id: u64) -> Result<GroupSummary> {
    let groups = client.fetch_groups()?;
    groups
        .into_iter()
        .find(|item| item.id == group_id)
        .ok_or_else(|| anyhow!("group not found: {}", group_id))
}
