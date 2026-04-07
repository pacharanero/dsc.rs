use anyhow::{Context, Result, anyhow};
use serde_json::{Value, json};

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

    /// Fetch a single theme by ID.
    pub fn fetch_theme(&self, theme_id: u64) -> Result<Value> {
        let response = self.get(&format!("/admin/themes/{}.json", theme_id))?;
        let status = response.status();
        let text = response.text().context("reading theme response body")?;
        if !status.is_success() {
            return Err(http_error("theme request", status, &text));
        }
        let value: Value = serde_json::from_str(&text).context("parsing theme response")?;
        Ok(value)
    }

    /// Create a new theme and return its ID.
    pub fn create_theme(&self, theme: &Value) -> Result<u64> {
        let payload = json!({ "theme": theme });
        let response = self
            .post("/admin/themes.json")?
            .json(&payload)
            .send()
            .context("creating theme")?;
        let status = response.status();
        let text = response.text().context("reading create theme response")?;
        if !status.is_success() {
            return Err(http_error("create theme request", status, &text));
        }
        let value: Value =
            serde_json::from_str(&text).context("parsing create theme response")?;
        let id = value
            .get("theme")
            .and_then(|v| v.get("id"))
            .or_else(|| value.get("id"))
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("missing theme id in create response"))?;
        Ok(id)
    }

    /// Delete a theme by ID.
    pub fn delete_theme(&self, theme_id: u64) -> Result<()> {
        let response = self.delete(&format!("/admin/themes/{}.json", theme_id))?;
        let status = response.status();
        let text = response.text().context("reading delete theme response")?;
        if !status.is_success() {
            return Err(http_error("delete theme request", status, &text));
        }
        Ok(())
    }

    /// Update an existing theme.
    pub fn update_theme(&self, theme_id: u64, theme: &Value) -> Result<()> {
        let payload = json!({ "theme": theme });
        let response = self
            .put(&format!("/admin/themes/{}.json", theme_id))?
            .json(&payload)
            .send()
            .context("updating theme")?;
        let status = response.status();
        let text = response.text().context("reading update theme response")?;
        if !status.is_success() {
            return Err(http_error("update theme request", status, &text));
        }
        Ok(())
    }
}
