use super::client::DiscourseClient;
use anyhow::{anyhow, Context, Result};

impl DiscourseClient {
    /// Update a site setting by name (admin only).
    pub fn update_site_setting(&self, setting: &str, value: &str) -> Result<()> {
        let setting = setting.trim();
        if setting.is_empty() {
            return Err(anyhow!("site setting name is required"));
        }
        if setting.chars().any(|ch| ch.is_whitespace() || ch == '/') {
            return Err(anyhow!(
                "site setting name contains invalid characters: {}",
                setting
            ));
        }
        let payload = [("value", value)];
        let response = self
            .put(&format!("/admin/site_settings/{}.json", setting))?
            .form(&payload)
            .send()
            .context("updating site setting")?;
        let status = response.status();
        let text = response
            .text()
            .context("reading site setting update response")?;
        if !status.is_success() {
            return Err(anyhow!(
                "update site setting failed with {}: {}",
                status,
                text
            ));
        }
        Ok(())
    }
}
