use super::client::DiscourseClient;
use anyhow::{anyhow, Context, Result};
use serde_json::Value;

impl DiscourseClient {
    /// Trigger a backup on the Discourse instance.
    pub fn create_backup(&self) -> Result<()> {
        let payload = [("with_uploads", "true")];
        let response = self
            .post("/admin/backups.json")?
            .form(&payload)
            .send()
            .context("creating backup")?;
        let status = response.status();
        let text = response.text().context("reading backup create response")?;
        if !status.is_success() {
            return Err(anyhow!("create backup failed with {}: {}", status, text));
        }
        Ok(())
    }

    /// List backups available on the Discourse instance.
    pub fn list_backups(&self) -> Result<Value> {
        let response = self.get("/admin/backups.json")?;
        let status = response.status();
        let text = response.text().context("reading backups list response")?;
        if !status.is_success() {
            return Err(anyhow!("list backups failed with {}: {}", status, text));
        }
        let body: Value = serde_json::from_str(&text).context("parsing backups list json")?;
        Ok(body)
    }

    /// Restore a backup by filename/path.
    pub fn restore_backup(&self, backup_path: &str) -> Result<()> {
        let path = format!("/admin/backups/{}/restore", backup_path);
        let response = self.post(&path)?.send().context("restoring backup")?;
        let status = response.status();
        let text = response.text().context("reading backup restore response")?;
        if !status.is_success() {
            return Err(anyhow!("restore backup failed with {}: {}", status, text));
        }
        Ok(())
    }
}
