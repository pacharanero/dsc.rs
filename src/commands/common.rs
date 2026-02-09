use crate::config::{find_discourse, Config, DiscourseConfig};
use crate::api::DiscourseClient;
use anyhow::{anyhow, Result};

pub fn select_discourse<'a>(
    config: &'a Config,
    discourse_name: Option<&str>,
) -> Result<&'a DiscourseConfig> {
    if let Some(name) = discourse_name {
        return find_discourse(config, name).ok_or_else(|| anyhow!("unknown discourse {}", name));
    }
    Err(anyhow!("discourse name is required"))
}

pub fn ensure_api_credentials(discourse: &DiscourseConfig) -> Result<()> {
    let apikey = discourse.apikey.as_deref().unwrap_or("").trim();
    let api_username = discourse.api_username.as_deref().unwrap_or("").trim();
    if apikey.is_empty() || api_username.is_empty() {
        return Err(anyhow!(
            "missing api credentials for {}; please set apikey and api_username in dsc.toml",
            discourse.name
        ));
    }
    Ok(())
}

pub fn parse_tags(raw: &str) -> Vec<String> {
    raw.split(|ch| ch == ';' || ch == ',')
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect()
}

pub fn fetch_fullname_from_url(baseurl: &str) -> Option<String> {
    let temp = DiscourseConfig {
        name: "temp".to_string(),
        baseurl: baseurl.to_string(),
        ..DiscourseConfig::default()
    };
    let client = match DiscourseClient::new(&temp) {
        Ok(client) => client,
        Err(err) => {
            println!("Failed to query site title for {}: {}", baseurl, err);
            return None;
        }
    };
    match client.fetch_site_title() {
        Ok(title) => {
            let title = title.trim().to_string();
            if title.is_empty() {
                None
            } else {
                Some(title)
            }
        }
        Err(err) => {
            println!("Failed to query site title for {}: {}", baseurl, err);
            None
        }
    }
}
