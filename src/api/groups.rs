use super::client::DiscourseClient;
use super::models::{GroupDetail, GroupDetailResponse, GroupSummary, GroupsResponse};
use anyhow::{anyhow, Context, Result};
use serde_json::Value;

impl DiscourseClient {
    /// Fetch all groups.
    pub fn fetch_groups(&self) -> Result<Vec<GroupSummary>> {
        let response = self.get("/groups.json")?;
        let status = response.status();
        let text = response.text().context("reading groups response body")?;
        if !status.is_success() {
            return Err(anyhow!("groups request failed with {}: {}", status, text));
        }
        let body: GroupsResponse = serde_json::from_str(&text).context("parsing groups json")?;
        Ok(body.groups)
    }

    /// Fetch group details by ID (fallbacks to name lookup if needed).
    pub fn fetch_group_detail(
        &self,
        group_id: u64,
        group_name: Option<&str>,
    ) -> Result<GroupDetail> {
        let id_path = format!("/groups/{}.json", group_id);
        if let Ok(detail) = self.fetch_group_detail_by_path(&id_path) {
            return Ok(detail);
        }
        if let Some(name) = group_name {
            let name_path = format!("/groups/{}.json", name);
            return self.fetch_group_detail_by_path(&name_path);
        }
        Err(anyhow!("group detail not found"))
    }

    /// Create a group with detailed settings copied from a source group.
    pub fn create_group(&self, group: &GroupDetail) -> Result<u64> {
        let mut payload: Vec<(String, String)> = Vec::new();
        payload.push(("group[name]".to_string(), group.name.clone()));
        if let Some(full_name) = group.full_name.clone() {
            payload.push(("group[full_name]".to_string(), full_name));
        }
        push_opt(&mut payload, "group[title]", group.title.as_deref());
        push_opt(
            &mut payload,
            "group[grant_trust_level]",
            group
                .grant_trust_level
                .as_ref()
                .map(|v| v.to_string())
                .as_deref(),
        );
        push_opt(
            &mut payload,
            "group[visibility_level]",
            group
                .visibility_level
                .as_ref()
                .map(|v| v.to_string())
                .as_deref(),
        );
        push_opt(
            &mut payload,
            "group[mentionable_level]",
            group
                .mentionable_level
                .as_ref()
                .map(|v| v.to_string())
                .as_deref(),
        );
        push_opt(
            &mut payload,
            "group[messageable_level]",
            group
                .messageable_level
                .as_ref()
                .map(|v| v.to_string())
                .as_deref(),
        );
        push_opt(
            &mut payload,
            "group[default_notification_level]",
            group
                .default_notification_level
                .as_ref()
                .map(|v| v.to_string())
                .as_deref(),
        );
        push_opt(
            &mut payload,
            "group[members_visibility_level]",
            group
                .members_visibility_level
                .as_ref()
                .map(|v| v.to_string())
                .as_deref(),
        );
        push_opt(
            &mut payload,
            "group[primary_group]",
            group
                .primary_group
                .as_ref()
                .map(|v| v.to_string())
                .as_deref(),
        );
        push_opt(
            &mut payload,
            "group[public_admission]",
            group
                .public_admission
                .as_ref()
                .map(|v| v.to_string())
                .as_deref(),
        );
        push_opt(
            &mut payload,
            "group[public_exit]",
            group.public_exit.as_ref().map(|v| v.to_string()).as_deref(),
        );
        push_opt(
            &mut payload,
            "group[allow_membership_requests]",
            group
                .allow_membership_requests
                .as_ref()
                .map(|v| v.to_string())
                .as_deref(),
        );
        push_opt(
            &mut payload,
            "group[automatic_membership_email_domains]",
            group.automatic_membership_email_domains.as_deref(),
        );
        push_opt(
            &mut payload,
            "group[automatic_membership_retroactive]",
            group
                .automatic_membership_retroactive
                .as_ref()
                .map(|v| v.to_string())
                .as_deref(),
        );
        push_opt(
            &mut payload,
            "group[membership_request_template]",
            group.membership_request_template.as_deref(),
        );
        push_opt(
            &mut payload,
            "group[flair_icon]",
            group.flair_icon.as_deref(),
        );
        push_opt(
            &mut payload,
            "group[flair_upload_id]",
            group
                .flair_upload_id
                .as_ref()
                .map(|v| v.to_string())
                .as_deref(),
        );
        push_opt(
            &mut payload,
            "group[flair_color]",
            group.flair_color.as_deref(),
        );
        push_opt(
            &mut payload,
            "group[flair_background_color]",
            group.flair_background_color.as_deref(),
        );
        push_opt(&mut payload, "group[bio_raw]", group.bio_raw.as_deref());
        let response = self
            .post("/admin/groups")?
            .form(&payload)
            .send()
            .context("creating group")?;
        let status = response.status();
        let text = response.text().context("reading group response body")?;
        if !status.is_success() {
            return Err(anyhow!("create group failed with {}: {}", status, text));
        }
        let value: Value = serde_json::from_str(&text).context("parsing group response json")?;
        let id = value
            .get("group")
            .and_then(|group| group.get("id"))
            .and_then(|id| id.as_u64())
            .or_else(|| {
                value
                    .get("basic_group")
                    .and_then(|g| g.get("id"))
                    .and_then(|id| id.as_u64())
            })
            .or_else(|| value.get("id").and_then(|id| id.as_u64()))
            .ok_or_else(|| anyhow!("missing group id in response: {}", text))?;
        Ok(id)
    }

    fn fetch_group_detail_by_path(&self, path: &str) -> Result<GroupDetail> {
        let response = self.get(path)?;
        let status = response.status();
        let text = response.text().context("reading group detail body")?;
        if !status.is_success() {
            return Err(anyhow!("group detail failed with {}: {}", status, text));
        }
        let body: GroupDetailResponse =
            serde_json::from_str(&text).context("parsing group detail json")?;
        Ok(body.group)
    }
}

fn push_opt(payload: &mut Vec<(String, String)>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        payload.push((key.to_string(), value.to_string()));
    }
}
