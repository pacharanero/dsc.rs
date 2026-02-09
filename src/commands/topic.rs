use crate::commands::common::{ensure_api_credentials, select_discourse};
use crate::config::Config;
use crate::api::DiscourseClient;
use crate::utils::{read_markdown, resolve_topic_path, write_markdown};
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

pub fn topic_pull(
    config: &Config,
    discourse_name: &str,
    topic_id: u64,
    local_path: Option<&Path>,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;
    let topic = client.fetch_topic(topic_id, true)?;
    let raw = topic
        .post_stream
        .posts
        .get(0)
        .and_then(|p| p.raw.clone())
        .ok_or_else(|| anyhow!("topic has no raw content"))?;
    let title = topic
        .title
        .as_deref()
        .filter(|t| !t.trim().is_empty())
        .map(|t| t.to_string())
        .or_else(|| {
            topic
                .slug
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| format!("topic-{}", topic_id));
    let target = resolve_topic_path(local_path, &title, &std::env::current_dir()?)?;
    write_markdown(&target, &raw)?;
    println!("{}", target.display());
    Ok(())
}

pub fn topic_push(
    config: &Config,
    discourse_name: &str,
    topic_id: u64,
    local_path: &Path,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;
    let topic = client.fetch_topic(topic_id, true)?;
    let post = topic
        .post_stream
        .posts
        .get(0)
        .ok_or_else(|| anyhow!("topic has no posts"))?;
    let raw = read_markdown(local_path)?;
    client.update_post(post.id, &raw)?;
    Ok(())
}

pub fn topic_sync(
    config: &Config,
    discourse_name: &str,
    topic_id: u64,
    local_path: &Path,
    assume_yes: bool,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;
    let topic = client.fetch_topic(topic_id, true)?;
    let post = topic
        .post_stream
        .posts
        .get(0)
        .ok_or_else(|| anyhow!("topic has no posts"))?;
    let local_meta =
        fs::metadata(local_path).with_context(|| format!("reading {}", local_path.display()))?;
    let local_mtime = local_meta.modified()?;

    let remote_ts = post
        .updated_at
        .as_deref()
        .or(post.created_at.as_deref())
        .ok_or_else(|| anyhow!("missing remote timestamps"))?;
    let remote_time = chrono::DateTime::parse_from_rfc3339(remote_ts)
        .context("parsing remote timestamp")?
        .with_timezone(&chrono::Utc);

    println!(
        "Local file:  {}",
        chrono::DateTime::<chrono::Utc>::from(local_mtime)
    );
    println!("Remote post: {}", remote_time);

    let pull = remote_time > chrono::DateTime::<chrono::Utc>::from(local_mtime);
    if !assume_yes && !confirm_sync(pull)? {
        return Ok(());
    }

    if pull {
        let raw = post
            .raw
            .clone()
            .ok_or_else(|| anyhow!("missing raw content"))?;
        write_markdown(local_path, &raw)?;
    } else {
        let raw = read_markdown(local_path)?;
        client.update_post(post.id, &raw)?;
    }

    Ok(())
}

fn confirm_sync(pull: bool) -> Result<bool> {
    let action = if pull {
        "pull from Discourse"
    } else {
        "push to Discourse"
    };
    print!("Proceed to {}? [y/N]: ", action);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim(), "y" | "Y" | "yes" | "YES"))
}
