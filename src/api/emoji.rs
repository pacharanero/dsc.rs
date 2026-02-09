use super::client::DiscourseClient;
use super::models::CustomEmoji;
use anyhow::{anyhow, Context, Result};
use reqwest::StatusCode;
use serde_json::Value;
use std::path::Path;

impl DiscourseClient {
    /// Upload a custom emoji.
    pub fn upload_emoji(&self, emoji_path: &Path, emoji_name: &str) -> Result<()> {
        let make_form = || -> Result<reqwest::blocking::multipart::Form> {
            let file = std::fs::read(emoji_path)
                .with_context(|| format!("reading {}", emoji_path.display()))?;
            let part = reqwest::blocking::multipart::Part::bytes(file)
                .file_name(
                    emoji_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("emoji.png")
                        .to_string(),
                )
                .mime_str("image/png")
                .context("setting emoji mime")?;
            Ok(reqwest::blocking::multipart::Form::new()
                .part("emoji[image]", part)
                .text("emoji[name]", emoji_name.to_string()))
        };

        let mut response = self
            .post("/admin/customize/emojis.json")?
            .multipart(make_form()?)
            .send()
            .context("uploading emoji")?;
        if response.status() == StatusCode::NOT_FOUND {
            response = self
                .post("/admin/customize/emojis")?
                .multipart(make_form()?)
                .send()
                .context("uploading emoji")?;
        }
        if !response.status().is_success() {
            let status = response.status();
            let text = response
                .text()
                .unwrap_or_else(|_| "<failed to read response body>".to_string());
            let hint = if status == StatusCode::NOT_FOUND || status == StatusCode::FORBIDDEN {
                " (requires an admin API key)"
            } else {
                ""
            };
            return Err(anyhow!(
                "emoji upload failed with {}{}: {}",
                status,
                hint,
                text
            ));
        }
        Ok(())
    }

    /// List custom emojis.
    pub fn list_custom_emojis(&self) -> Result<Vec<CustomEmoji>> {
        if let Ok(emojis) = self.list_admin_emojis() {
            if !emojis.is_empty() {
                return Ok(emojis);
            }
        }
        if let Ok(emojis) = self.list_admin_config_emojis() {
            if !emojis.is_empty() {
                return Ok(emojis);
            }
        }
        self.list_public_emojis()
    }

    fn list_admin_emojis(&self) -> Result<Vec<CustomEmoji>> {
        let response = self.get("/admin/customize/emojis.json")?;
        let status = response.status();
        let text = response.text().context("reading emoji list response")?;
        if !status.is_success() {
            return Err(anyhow!("emoji list failed with {}: {}", status, text));
        }
        let value: Value = serde_json::from_str(&text).context("parsing emoji list json")?;
        let emojis = if let Some(arr) = value.as_array() {
            extract_emojis_from_array(arr, self.baseurl())
        } else if let Some(val) = value.get("emojis") {
            extract_emojis_from_value(val, self.baseurl())
        } else if let Some(val) = value.get("custom_emoji") {
            extract_emojis_from_value(val, self.baseurl())
        } else if let Some(val) = value.get("custom") {
            extract_emojis_from_value(val, self.baseurl())
        } else if let Some(map) = value.as_object() {
            let mut out = Vec::new();
            extract_emojis_from_map(map, self.baseurl(), &mut out);
            out
        } else {
            Vec::new()
        };
        Ok(emojis)
    }

    fn list_public_emojis(&self) -> Result<Vec<CustomEmoji>> {
        let response = self.get("/emoji.json")?;
        let status = response.status();
        let text = response.text().context("reading emoji.json response")?;
        if status == StatusCode::NOT_FOUND {
            return Ok(Vec::new());
        }
        if !status.is_success() {
            return Err(anyhow!(
                "emoji.json request failed with {}: {}",
                status,
                text
            ));
        }
        let value: Value = serde_json::from_str(&text).context("parsing emoji.json")?;
        let baseurl = self.baseurl().trim_end_matches('/');
        let mut out = Vec::new();
        if let Some(val) = value.get("custom_emoji") {
            out.extend(extract_emojis_from_value(val, baseurl));
        }
        if let Some(val) = value.get("custom") {
            out.extend(extract_emojis_from_value(val, baseurl));
        }
        if out.is_empty() {
            if let Some(val) = value.get("emoji") {
                out.extend(extract_emojis_from_value(val, baseurl));
            }
        }
        Ok(out)
    }

    fn list_admin_config_emojis(&self) -> Result<Vec<CustomEmoji>> {
        let response = self.get("/admin/config/emoji.json")?;
        let status = response.status();
        let text = response
            .text()
            .context("reading admin config emoji response")?;
        if status == StatusCode::NOT_FOUND {
            return Ok(Vec::new());
        }
        if !status.is_success() {
            return Err(anyhow!(
                "admin config emoji request failed with {}: {}",
                status,
                text
            ));
        }
        let value: Value =
            serde_json::from_str(&text).context("parsing admin config emoji json")?;
        if let Some(val) = value.get("emojis") {
            return Ok(extract_emojis_from_value(val, self.baseurl()));
        }
        Ok(extract_emojis_from_value(&value, self.baseurl()))
    }
}

fn extract_emojis_from_array(emojis: &[Value], baseurl: &str) -> Vec<CustomEmoji> {
    let mut out = Vec::new();
    for item in emojis.iter() {
        let name = item.get("name").and_then(|v| v.as_str());
        let url = item
            .get("url")
            .and_then(|v| v.as_str())
            .or_else(|| item.get("image_url").and_then(|v| v.as_str()));
        if let (Some(name), Some(url)) = (name, url) {
            out.push(CustomEmoji {
                name: name.to_string(),
                url: normalize_emoji_url(baseurl, url),
            });
        }
    }
    out
}

fn extract_emojis_from_map(
    map: &serde_json::Map<String, Value>,
    baseurl: &str,
    out: &mut Vec<CustomEmoji>,
) {
    for (name, value) in map.iter() {
        let url = value
            .as_str()
            .or_else(|| value.get("url").and_then(|v| v.as_str()))
            .or_else(|| value.get("image_url").and_then(|v| v.as_str()))
            .or_else(|| value.get("path").and_then(|v| v.as_str()));
        if let Some(url) = url {
            out.push(CustomEmoji {
                name: name.to_string(),
                url: normalize_emoji_url(baseurl, url),
            });
        }
    }
}

fn extract_emojis_from_value(value: &Value, baseurl: &str) -> Vec<CustomEmoji> {
    match value {
        Value::Array(arr) => extract_emojis_from_array(arr, baseurl),
        Value::Object(map) => {
            let name = map.get("name").and_then(|v| v.as_str());
            let url = map
                .get("url")
                .and_then(|v| v.as_str())
                .or_else(|| map.get("image_url").and_then(|v| v.as_str()))
                .or_else(|| map.get("path").and_then(|v| v.as_str()));
            if let (Some(name), Some(url)) = (name, url) {
                return vec![CustomEmoji {
                    name: name.to_string(),
                    url: normalize_emoji_url(baseurl, url),
                }];
            }
            let mut out = Vec::new();
            extract_emojis_from_map(map, baseurl, &mut out);
            out
        }
        _ => Vec::new(),
    }
}

fn normalize_emoji_url(baseurl: &str, url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else if url.starts_with("//") {
        let scheme = if baseurl.starts_with("http://") {
            "http:"
        } else {
            "https:"
        };
        format!("{}{}", scheme, url)
    } else if url.starts_with('/') {
        format!("{}{}", baseurl, url)
    } else {
        format!("{}/{}", baseurl, url)
    }
}
