use crate::api::DiscourseClient;
use crate::cli::ListFormat;
use crate::commands::common::{ensure_api_credentials, select_discourse};
use crate::config::Config;
use anyhow::Result;

pub fn user_list(
    config: &Config,
    discourse_name: &str,
    listing: &str,
    page: u32,
    format: ListFormat,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;
    let users = client.admin_list_users(listing, page)?;

    match format {
        ListFormat::Text => {
            if users.is_empty() {
                println!("No users found in listing '{}'.", listing);
                return Ok(());
            }
            let name_width = users
                .iter()
                .map(|u| u.username.len())
                .max()
                .unwrap_or(0)
                .max(8);
            for u in &users {
                let flag = if u.admin.unwrap_or(false) {
                    "admin"
                } else if u.moderator.unwrap_or(false) {
                    "mod"
                } else if u.suspended.unwrap_or(false) {
                    "suspended"
                } else if u.silenced.unwrap_or(false) {
                    "silenced"
                } else {
                    "-"
                };
                let tl = u
                    .trust_level
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "?".to_string());
                println!(
                    "{:<width$}  id:{}  tl:{}  {}",
                    u.username,
                    u.id,
                    tl,
                    flag,
                    width = name_width
                );
            }
        }
        ListFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&users)?);
        }
        ListFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&users)?);
        }
    }

    Ok(())
}

pub fn user_info(
    config: &Config,
    discourse_name: &str,
    username: &str,
    format: ListFormat,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;
    let detail = client.fetch_user_detail(username)?;

    match format {
        ListFormat::Text => {
            println!("id:          {}", detail.id);
            println!("username:    {}", detail.username);
            if let Some(name) = &detail.name {
                println!("name:        {}", name);
            }
            if let Some(email) = &detail.email {
                println!("email:       {}", email);
            }
            if let Some(tl) = detail.trust_level {
                println!("trust_level: {}", tl);
            }
            if detail.admin.unwrap_or(false) {
                println!("role:        admin");
            } else if detail.moderator.unwrap_or(false) {
                println!("role:        moderator");
            }
            if let Some(until) = &detail.suspended_till {
                println!("suspended:   until {}", until);
            }
            if let Some(until) = &detail.silenced_till {
                println!("silenced:    until {}", until);
            }
            if let Some(last) = &detail.last_seen_at {
                println!("last_seen:   {}", last);
            }
            if let Some(created) = &detail.created_at {
                println!("created:     {}", created);
            }
            if let Some(posts) = detail.post_count {
                println!("posts:       {}", posts);
            }
            if !detail.groups.is_empty() {
                println!("groups:      {}", detail.groups.len());
            }
        }
        ListFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&detail)?);
        }
        ListFormat::Yaml => {
            println!("{}", serde_yaml::to_string(&detail)?);
        }
    }
    Ok(())
}

pub fn user_suspend(
    config: &Config,
    discourse_name: &str,
    username: &str,
    until: &str,
    reason: &str,
    dry_run: bool,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;

    if dry_run {
        println!(
            "[dry-run] {}: would suspend {} until {} (reason: {})",
            discourse.name,
            username,
            until,
            if reason.is_empty() { "<none>" } else { reason }
        );
        return Ok(());
    }

    let detail = client.fetch_user_detail(username)?;
    client.suspend_user(detail.id, until, reason)?;
    println!("Suspended {} (id:{}) until {}", detail.username, detail.id, until);
    Ok(())
}

pub fn user_unsuspend(
    config: &Config,
    discourse_name: &str,
    username: &str,
    dry_run: bool,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;

    if dry_run {
        println!("[dry-run] {}: would unsuspend {}", discourse.name, username);
        return Ok(());
    }

    let detail = client.fetch_user_detail(username)?;
    client.unsuspend_user(detail.id)?;
    println!("Unsuspended {} (id:{})", detail.username, detail.id);
    Ok(())
}

pub fn user_silence(
    config: &Config,
    discourse_name: &str,
    username: &str,
    until: &str,
    reason: &str,
    dry_run: bool,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;

    if dry_run {
        println!(
            "[dry-run] {}: would silence {}{}{}",
            discourse.name,
            username,
            if until.is_empty() {
                String::new()
            } else {
                format!(" until {}", until)
            },
            if reason.is_empty() {
                String::new()
            } else {
                format!(" (reason: {})", reason)
            },
        );
        return Ok(());
    }

    let detail = client.fetch_user_detail(username)?;
    client.silence_user(detail.id, until, reason)?;
    println!("Silenced {} (id:{})", detail.username, detail.id);
    Ok(())
}

pub fn user_unsilence(
    config: &Config,
    discourse_name: &str,
    username: &str,
    dry_run: bool,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;

    if dry_run {
        println!("[dry-run] {}: would unsilence {}", discourse.name, username);
        return Ok(());
    }

    let detail = client.fetch_user_detail(username)?;
    client.unsilence_user(detail.id)?;
    println!("Unsilenced {} (id:{})", detail.username, detail.id);
    Ok(())
}

#[derive(Clone, Copy)]
pub enum Role {
    Admin,
    Moderator,
}

pub fn user_promote(
    config: &Config,
    discourse_name: &str,
    username: &str,
    role: Role,
    dry_run: bool,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;

    let role_label = match role {
        Role::Admin => "admin",
        Role::Moderator => "moderator",
    };

    if dry_run {
        println!(
            "[dry-run] {}: would grant {} to {}",
            discourse.name, role_label, username
        );
        return Ok(());
    }

    let detail = client.fetch_user_detail(username)?;
    match role {
        Role::Admin => client.grant_admin(detail.id)?,
        Role::Moderator => client.grant_moderation(detail.id)?,
    }
    println!("Granted {} to {} (id:{})", role_label, detail.username, detail.id);
    Ok(())
}

pub fn user_demote(
    config: &Config,
    discourse_name: &str,
    username: &str,
    role: Role,
    dry_run: bool,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;

    let role_label = match role {
        Role::Admin => "admin",
        Role::Moderator => "moderator",
    };

    if dry_run {
        println!(
            "[dry-run] {}: would revoke {} from {}",
            discourse.name, role_label, username
        );
        return Ok(());
    }

    let detail = client.fetch_user_detail(username)?;
    match role {
        Role::Admin => client.revoke_admin(detail.id)?,
        Role::Moderator => client.revoke_moderation(detail.id)?,
    }
    println!(
        "Revoked {} from {} (id:{})",
        role_label, detail.username, detail.id
    );
    Ok(())
}

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
