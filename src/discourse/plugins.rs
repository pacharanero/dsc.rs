use anyhow::{anyhow, Context, Result};
use serde_json::Value;

use super::client::DiscourseClient;

impl DiscourseClient {
    /// List installed plugins on the Discourse instance.
    pub fn list_plugins(&self) -> Result<Value> {
        let response = self.get("/admin/plugins.json")?;
        let status = response.status();
        let text = response.text().context("reading plugins response body")?;
        if !status.is_success() {
            return Err(anyhow!("plugins request failed with {}: {}", status, text));
        }
        let value: Value = serde_json::from_str(&text).context("parsing plugins response")?;
        Ok(value)
    }
}
