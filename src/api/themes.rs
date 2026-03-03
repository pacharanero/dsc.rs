use anyhow::{Context, Result};
use serde_json::Value;

use super::client::DiscourseClient;
use super::error::http_error;

impl DiscourseClient {
    /// List installed themes on the Discourse instance.
    pub fn list_themes(&self) -> Result<Value> {
        let response = self.get("/admin/themes.json")?;
        let status = response.status();
        let text = response.text().context("reading themes response body")?;
        if !status.is_success() {
            return Err(http_error("themes request", status, &text));
        }
        let value: Value = serde_json::from_str(&text).context("parsing themes response")?;
        Ok(value)
    }
}
