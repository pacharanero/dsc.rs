use crate::api::DiscourseClient;
use crate::cli::ListFormat;
use crate::commands::common::{ensure_api_credentials, select_discourse};
use crate::config::Config;
use anyhow::Result;

pub fn user_groups_list(
    config: &Config,
    discourse_name: &str,
    username: &str,
    format: ListFormat,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;

    let mut groups = client.fetch_user_groups(username)?;
    groups.sort_by(|a, b| a.name.cmp(&b.name));

    match format {
        ListFormat::Text => {
            if groups.is_empty() {
                println!("{} is not in any groups.", username);
                return Ok(());
            }
            let name_width = groups
                .iter()
                .map(|g| g.name.len())
                .max()
                .unwrap_or(0)
                .max(4);
            for g in &groups {
                println!("{:<width$}  id:{}", g.name, g.id, width = name_width);
            }
        }
        ListFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&groups)?);
        }
        ListFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&groups)?);
        }
    }

    Ok(())
}

pub fn user_groups_add(
    config: &Config,
    discourse_name: &str,
    username: &str,
    group_id: u64,
    notify: bool,
    dry_run: bool,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;

    if dry_run {
        println!(
            "[dry-run] {}: would add {} to group {} (notify={})",
            discourse.name, username, group_id, notify
        );
        return Ok(());
    }

    let usernames = vec![username.to_string()];
    let outcome = client.add_group_members_by_username(group_id, &usernames, notify)?;
    if outcome.added_usernames.is_empty() {
        println!(
            "{} was already a member of group {} (or Discourse reported no change)",
            username, group_id
        );
    } else {
        println!("Added {} to group {}", username, group_id);
    }
    if !outcome.errors.is_empty() {
        eprintln!("Server notes:");
        for msg in &outcome.errors {
            eprintln!("  - {}", msg);
        }
    }
    Ok(())
}

pub fn user_groups_remove(
    config: &Config,
    discourse_name: &str,
    username: &str,
    group_id: u64,
    dry_run: bool,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;

    if dry_run {
        println!(
            "[dry-run] {}: would remove {} from group {}",
            discourse.name, username, group_id
        );
        return Ok(());
    }

    let usernames = vec![username.to_string()];
    client.remove_group_members_by_username(group_id, &usernames)?;
    println!("Removed {} from group {}", username, group_id);
    Ok(())
}
