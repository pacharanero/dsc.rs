use super::client::DiscourseClient;
use super::error::http_error;
use super::models::{
    GroupDetail, GroupDetailResponse, GroupMember, GroupMembersResponse, GroupSummary,
};
use anyhow::{Context, Result, anyhow};
use reqwest::StatusCode;
use serde_json::Value;
use std::collections::HashSet;

impl DiscourseClient {
    /// Fetch all groups.
    pub fn fetch_groups(&self) -> Result<Vec<GroupSummary>> {
        if let Some(groups) = self.fetch_groups_admin()? {
            return Ok(groups);
        }
        self.fetch_groups_paginated("/groups.json")
    }

    /// Fetch group details by ID (fallbacks to name lookup if needed).
    pub fn fetch_group_detail(
        &self,
        group_id: u64,
        group_name: Option<&str>,
    ) -> Result<GroupDetail> {
        let id_path = format!("/groups/{}.json", group_id);
        if let Some(detail) = self.fetch_group_detail_by_path(&id_path)? {
            return Ok(detail);
        }
        if let Some(name) = group_name {
            let name_path = format!("/groups/{}.json", name);
            if let Some(detail) = self.fetch_group_detail_by_path(&name_path)? {
                return Ok(detail);
            }
        }
        Err(anyhow!("group not found: {}", group_id))
    }

    pub fn fetch_group_members(
        &self,
        group_id: u64,
        group_name: Option<&str>,
    ) -> Result<Vec<GroupMember>> {
        let id_path = format!("/groups/{}/members.json", group_id);
        if let Some(members) = self.fetch_group_members_by_path(&id_path)? {
            return Ok(members);
        }
        if let Some(name) = group_name {
            let name_path = format!("/groups/{}/members.json", name);
            if let Some(members) = self.fetch_group_members_by_path(&name_path)? {
                return Ok(members);
            }
        }
        Err(anyhow!("group not found: {}", group_id))
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
            return Err(http_error("create group request", status, &text));
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

    fn fetch_group_detail_by_path(&self, path: &str) -> Result<Option<GroupDetail>> {
        let response = self.get(path)?;
        let status = response.status();
        let text = response.text().context("reading group detail body")?;
        if !status.is_success() {
            if status == StatusCode::NOT_FOUND {
                return Ok(None);
            }
            return Err(http_error("group detail request", status, &text));
        }
        let body: GroupDetailResponse =
            serde_json::from_str(&text).context("parsing group detail json")?;
        Ok(Some(body.group))
    }

    fn fetch_group_members_by_path(&self, path: &str) -> Result<Option<Vec<GroupMember>>> {
        let response = self.get(path)?;
        let status = response.status();
        let text = response.text().context("reading group members body")?;
        if !status.is_success() {
            if status == StatusCode::NOT_FOUND {
                return Ok(None);
            }
            return Err(http_error("group members request", status, &text));
        }
        let body: GroupMembersResponse =
            serde_json::from_str(&text).context("parsing group members json")?;
        Ok(Some(body.members))
    }

    fn fetch_groups_admin(&self) -> Result<Option<Vec<GroupSummary>>> {
        let response = self.get("/admin/groups.json")?;
        let status = response.status();
        let text = response.text().context("reading groups response body")?;
        if status.is_success() {
            if text.trim().is_empty() {
                return Ok(None);
            }
            let value: Value = serde_json::from_str(&text).context("parsing groups json")?;
            return Ok(Some(extract_groups_from_value(&value)?));
        }
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        Err(http_error("groups request", status, &text))
    }

    fn fetch_groups_paginated(&self, path: &str) -> Result<Vec<GroupSummary>> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        let mut next_path = Some(path.to_string());

        while let Some(path) = next_path.take() {
            let path = self.normalize_groups_path(&path);
            if !seen.insert(path.clone()) {
                return Err(anyhow!("groups request loop detected at {}", path));
            }
            let response = self.get(&path)?;
            let status = response.status();
            let text = response.text().context("reading groups response body")?;
            if !status.is_success() {
                return Err(http_error("groups request", status, &text));
            }
            if text.trim().is_empty() {
                return Err(anyhow!(
                    "groups request failed with {} (empty response)",
                    status
                ));
            }
            let value: Value = serde_json::from_str(&text).context("parsing groups json")?;
            let page_groups = extract_groups_from_value(&value)?;
            if page_groups.is_empty() {
                break;
            }
            out.extend(page_groups);
            next_path = extract_next_groups_path(&value);
        }

        Ok(out)
    }

    fn normalize_groups_path(&self, path: &str) -> String {
        let mut path = path.to_string();
        if let Some(stripped) = path.strip_prefix(self.baseurl()) {
            path = stripped.to_string();
        }
        if !path.starts_with('/') {
            path = format!("/{}", path);
        }
        if path.contains(".json") {
            return path;
        }
        if let Some((base, query)) = path.split_once('?') {
            format!("{}.json?{}", base, query)
        } else {
            format!("{}.json", path)
        }
    }
}

fn push_opt(payload: &mut Vec<(String, String)>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        payload.push((key.to_string(), value.to_string()));
    }
}

fn extract_groups_from_value(value: &Value) -> Result<Vec<GroupSummary>> {
    let groups = if let Some(arr) = value.as_array() {
        arr
    } else {
        value
            .get("groups")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("groups response missing groups array"))?
    };
    let mut out = Vec::with_capacity(groups.len());
    for group in groups {
        let parsed: GroupSummary =
            serde_json::from_value(group.clone()).context("parsing group summary")?;
        out.push(parsed);
    }
    Ok(out)
}

fn extract_next_groups_path(value: &Value) -> Option<String> {
    let direct = value
        .get("load_more_groups")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    if direct
        .as_deref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
    {
        return direct;
    }
    value
        .get("extras")
        .and_then(|extras| extras.get("load_more_groups"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty())
}
