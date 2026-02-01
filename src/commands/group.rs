use crate::cli::StructuredFormat;
use crate::commands::common::{ensure_api_credentials, select_discourse};
use crate::config::Config;
use crate::discourse::DiscourseClient;
use crate::utils::slugify;
use anyhow::{anyhow, Result};

pub fn group_list(config: &Config, discourse_name: &str) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;
    let groups = client.fetch_groups()?;
    for group in groups {
        let full_name = group.full_name.unwrap_or_else(|| "-".to_string());
        println!("{} - {} ({})", group.id, group.name, full_name);
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
    let groups = client.fetch_groups()?;
    let group_summary = groups
        .into_iter()
        .find(|item| item.id == group_id)
        .ok_or_else(|| anyhow!("group not found"))?;
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
    let groups = source_client.fetch_groups()?;
    let group_summary = groups
        .into_iter()
        .find(|item| item.id == group_id)
        .ok_or_else(|| anyhow!("group not found"))?;
    let mut group =
        source_client.fetch_group_detail(group_summary.id, Some(&group_summary.name))?;
    group.name = format!("{}-copy", slugify(&group.name));
    if let Some(full_name) = group.full_name.clone() {
        group.full_name = Some(format!("Copy of {}", full_name));
    }

    let target_client = DiscourseClient::new(target_discourse)?;
    let new_id = target_client.create_group(&group)?;
    println!("{}", new_id);
    Ok(())
}
