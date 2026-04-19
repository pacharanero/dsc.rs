use super::client::DiscourseClient;
use super::error::http_error;
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One row from /admin/users/list/<type>.json.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserSummary {
    pub id: u64,
    pub username: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub trust_level: Option<u64>,
    #[serde(default)]
    pub admin: Option<bool>,
    #[serde(default)]
    pub moderator: Option<bool>,
    #[serde(default)]
    pub suspended: Option<bool>,
    #[serde(default)]
    pub silenced: Option<bool>,
    #[serde(default)]
    pub last_seen_at: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Distilled /users/<username>.json payload.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserDetail {
    pub id: u64,
    pub username: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub trust_level: Option<u64>,
    #[serde(default)]
    pub admin: Option<bool>,
    #[serde(default)]
    pub moderator: Option<bool>,
    #[serde(default)]
    pub suspended_till: Option<String>,
    #[serde(default)]
    pub silenced_till: Option<String>,
    #[serde(default)]
    pub last_seen_at: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub post_count: Option<u64>,
    #[serde(default)]
    pub groups: Vec<Value>,
}

impl DiscourseClient {
    /// List users via the admin users endpoint.
    ///
    /// `listing` is one of: `active` (default), `new`, `staff`, `suspended`,
    /// `silenced`, `staged`. Discourse paginates 100 per page.
    pub fn admin_list_users(&self, listing: &str, page: u32) -> Result<Vec<UserSummary>> {
        let path = format!(
            "/admin/users/list/{}.json?show_emails=true&page={}",
            listing, page
        );
        let response = self.get(&path)?;
        let status = response.status();
        let text = response.text().context("reading user list response")?;
        if !status.is_success() {
            return Err(http_error("admin user list request", status, &text));
        }
        let users: Vec<UserSummary> =
            serde_json::from_str(&text).context("parsing user list response")?;
        Ok(users)
    }

    /// Look up a user by username (public endpoint).
    pub fn fetch_user_detail(&self, username: &str) -> Result<UserDetail> {
        let path = format!("/u/{}.json", username);
        let response = self.get(&path)?;
        let status = response.status();
        let text = response.text().context("reading user detail response")?;
        if !status.is_success() {
            return Err(http_error("user detail request", status, &text));
        }
        let value: Value =
            serde_json::from_str(&text).context("parsing user detail response")?;
        let user = value
            .get("user")
            .ok_or_else(|| anyhow!("user detail response missing `user` field"))?;
        let detail: UserDetail =
            serde_json::from_value(user.clone()).context("deserialising user detail")?;
        Ok(detail)
    }

    /// Suspend a user by ID. `until` is an ISO-8601 timestamp (or any string
    /// Discourse accepts, like "forever"); `reason` is mandatory from the UI
    /// but Discourse accepts empty via the API.
    pub fn suspend_user(&self, user_id: u64, until: &str, reason: &str) -> Result<()> {
        let payload = [("suspend_until", until), ("reason", reason)];
        self.put_admin_user_action(user_id, "suspend", &payload, "suspend user request")
    }

    /// Unsuspend a user by ID.
    pub fn unsuspend_user(&self, user_id: u64) -> Result<()> {
        self.put_admin_user_action(user_id, "unsuspend", &[], "unsuspend user request")
    }

    /// Silence a user by ID. Optional `silenced_till` (Discourse-accepted
    /// timestamp string) and `reason`; both default to empty.
    pub fn silence_user(&self, user_id: u64, until: &str, reason: &str) -> Result<()> {
        let mut payload: Vec<(&str, &str)> = Vec::new();
        if !until.is_empty() {
            payload.push(("silenced_till", until));
        }
        if !reason.is_empty() {
            payload.push(("reason", reason));
        }
        self.put_admin_user_action(user_id, "silence", &payload, "silence user request")
    }

    /// Unsilence a user by ID.
    pub fn unsilence_user(&self, user_id: u64) -> Result<()> {
        self.put_admin_user_action(user_id, "unsilence", &[], "unsilence user request")
    }

    /// Grant admin to a user.
    pub fn grant_admin(&self, user_id: u64) -> Result<()> {
        self.put_admin_user_action(user_id, "grant_admin", &[], "grant admin request")
    }

    /// Revoke admin from a user.
    pub fn revoke_admin(&self, user_id: u64) -> Result<()> {
        self.put_admin_user_action(user_id, "revoke_admin", &[], "revoke admin request")
    }

    /// Grant moderator to a user.
    pub fn grant_moderation(&self, user_id: u64) -> Result<()> {
        self.put_admin_user_action(
            user_id,
            "grant_moderation",
            &[],
            "grant moderation request",
        )
    }

    /// Revoke moderator from a user.
    pub fn revoke_moderation(&self, user_id: u64) -> Result<()> {
        self.put_admin_user_action(
            user_id,
            "revoke_moderation",
            &[],
            "revoke moderation request",
        )
    }

    fn put_admin_user_action(
        &self,
        user_id: u64,
        action: &str,
        payload: &[(&str, &str)],
        action_label: &str,
    ) -> Result<()> {
        let path = format!("/admin/users/{}/{}.json", user_id, action);
        let response = self.send_retrying(|| {
            let rb = self.put(&path)?;
            Ok(if payload.is_empty() {
                rb
            } else {
                rb.form(payload)
            })
        })?;
        let status = response.status();
        if !status.is_success() {
            let text = response
                .text()
                .unwrap_or_else(|_| "<failed to read response body>".to_string());
            return Err(http_error(action_label, status, &text));
        }
        Ok(())
    }
}
