use crate::commands::common::{ensure_api_credentials, parse_tags};
use crate::config::{Config, DiscourseConfig};
use crate::discourse::DiscourseClient;
use anyhow::{anyhow, Result};

pub fn set_site_setting(
    config: &Config,
    setting: &str,
    value: &str,
    tags: Option<&str>,
) -> Result<()> {
    let filter = tags.map(parse_tags).unwrap_or_default();
    let matches_filter = |disc: &DiscourseConfig| {
        if filter.is_empty() {
            return true;
        }
        let disc_tags = disc.tags.as_ref().map(|t| {
            t.iter()
                .map(|tag| tag.to_ascii_lowercase())
                .collect::<Vec<_>>()
        });
        let Some(disc_tags) = disc_tags else {
            return false;
        };
        filter.iter().any(|tag| {
            let tag = tag.to_ascii_lowercase();
            disc_tags.iter().any(|t| t == &tag)
        })
    };

    let mut matched = 0;
    for discourse in config.discourse.iter().filter(|d| matches_filter(d)) {
        matched += 1;
        ensure_api_credentials(discourse)?;
        let client = DiscourseClient::new(discourse)?;
        client.update_site_setting(setting, value)?;
        println!("{}: updated {}", discourse.name, setting);
    }

    if matched == 0 {
        return Err(anyhow!("no discourses matched the tag filter"));
    }

    Ok(())
}
