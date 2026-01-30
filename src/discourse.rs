use crate::config::DiscourseConfig;
use crate::utils::normalize_baseurl;
use anyhow::{anyhow, Context, Result};
use reqwest::blocking::{Client, Response};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

/// HTTP client for the Discourse API.
#[derive(Clone)]
pub struct DiscourseClient {
    baseurl: String,
    client: Client,
}

impl DiscourseClient {
    /// Create a new Discourse API client.
    pub fn new(config: &DiscourseConfig) -> Result<Self> {
        let baseurl = normalize_baseurl(&config.baseurl);
        if baseurl.is_empty() {
            return Err(anyhow!("baseurl is required"));
        }

        let mut headers = HeaderMap::new();
        if let (Some(apikey), Some(api_username)) =
            (config.apikey.as_ref(), config.api_username.as_ref())
        {
            headers.insert(
                "Api-Key",
                HeaderValue::from_str(apikey).context("invalid api key")?,
            );
            headers.insert(
                "Api-Username",
                HeaderValue::from_str(api_username).context("invalid api username")?,
            );
        }

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .context("building http client")?;

        Ok(Self { baseurl, client })
    }

    /// Return the configured base URL.
    pub fn baseurl(&self) -> &str {
        &self.baseurl
    }

    fn get(&self, path: &str) -> Result<Response> {
        let url = format!("{}{}", self.baseurl, path);
        self.client.get(url).send().context("sending request")
    }

    fn post(&self, path: &str) -> Result<reqwest::blocking::RequestBuilder> {
        let url = format!("{}{}", self.baseurl, path);
        Ok(self.client.post(url))
    }

    fn put(&self, path: &str) -> Result<reqwest::blocking::RequestBuilder> {
        let url = format!("{}{}", self.baseurl, path);
        Ok(self.client.put(url))
    }

    /// Fetch the Discourse site title.
    pub fn fetch_site_title(&self) -> Result<String> {
        let site_json_error = match self.get("/site.json") {
            Ok(response) => {
                let status = response.status();
                let text = response.text().context("reading site.json response body")?;
                if status.is_success() {
                    let body: SiteResponse =
                        serde_json::from_str(&text).context("parsing site.json")?;
                    return Ok(body.site.title);
                }
                anyhow!("site.json request failed with {}", status)
            }
            Err(err) => err,
        };

        let response = self.get("/")?;
        let status = response.status();
        let html = response.text().context("reading site HTML")?;
        if !status.is_success() {
            return Err(anyhow!(
                "site title lookup failed (site.json error: {}; HTML request failed with {})",
                site_json_error,
                status
            ));
        }
        if let Some(title) = extract_html_title(&html) {
            return Ok(title);
        }
        Err(anyhow!(
            "site title lookup failed (site.json error: {}; HTML missing <title>)",
            site_json_error
        ))
    }

    /// Fetch the current Discourse version if exposed via the API.
    pub fn fetch_version(&self) -> Result<Option<String>> {
        let response = self.get("/about.json")?;
        let status = response.status();
        let body: AboutResponse = response.json().context("reading about.json")?;
        if !status.is_success() {
            return Err(anyhow!("about.json request failed with {}", status));
        }
        Ok(body.about.version.or(body.about.installed_version))
    }

    /// Fetch a topic by ID.
    pub fn fetch_topic(&self, topic_id: u64, include_raw: bool) -> Result<TopicResponse> {
        let path = if include_raw {
            format!("/t/{}.json?include_raw=1", topic_id)
        } else {
            format!("/t/{}.json", topic_id)
        };
        let response = self.get(&path)?;
        let status = response.status();
        if !status.is_success() {
            let text = response.text().context("reading topic response body")?;
            return Err(anyhow!("topic request failed with {}: {}", status, text));
        }
        let text = response.text().context("reading topic response body")?;
        let body: TopicResponse = serde_json::from_str(&text).context("parsing topic json")?;
        Ok(body)
    }

    /// Fetch a post by ID and return its raw content.
    pub fn fetch_post_raw(&self, post_id: u64) -> Result<Option<String>> {
        let path = format!("/posts/{}.json?include_raw=1", post_id);
        let response = self.get(&path)?;
        let status = response.status();
        let text = response.text().context("reading post response body")?;
        if !status.is_success() {
            return Err(anyhow!("post request failed with {}: {}", status, text));
        }
        let value: Value = serde_json::from_str(&text).context("parsing post response")?;
        Ok(value
            .get("raw")
            .and_then(|raw| raw.as_str())
            .map(|raw| raw.to_string()))
    }

    /// Fetch a category by ID (topics list included).
    pub fn fetch_category(&self, category_id: u64) -> Result<CategoryResponse> {
        let path = format!("/c/{}.json", category_id);
        let response = self.get(&path)?;
        let status = response.status();
        let body: CategoryResponse = response.json().context("reading category json")?;
        if !status.is_success() {
            return Err(anyhow!("category request failed with {}", status));
        }
        Ok(body)
    }

    /// Fetch all categories.
    pub fn fetch_categories(&self) -> Result<Vec<CategoryInfo>> {
        let response = self.get("/categories.json?include_subcategories=true")?;
        let status = response.status();
        let body: CategoriesResponse = response.json().context("reading categories json")?;
        if !status.is_success() {
            return Err(anyhow!("categories request failed with {}", status));
        }
        let mut categories = body.category_list.categories;
        if let Ok(site_categories) = self.fetch_site_categories() {
            let mut seen = HashMap::new();
            for (idx, cat) in categories.iter().enumerate() {
                if let Some(id) = cat.id {
                    seen.insert(id, idx);
                }
            }
            for cat in site_categories {
                if let Some(id) = cat.id {
                    if !seen.contains_key(&id) {
                        categories.push(cat);
                    }
                }
            }
        }
        Ok(categories)
    }

    fn fetch_site_categories(&self) -> Result<Vec<CategoryInfo>> {
        let response = self.get("/site.json")?;
        let status = response.status();
        let text = response.text().context("reading site.json response body")?;
        if !status.is_success() {
            return Err(anyhow!(
                "site.json request failed with {}: {}",
                status,
                text
            ));
        }
        let value: Value = serde_json::from_str(&text).context("parsing site.json")?;
        let array = value
            .get("categories")
            .and_then(|v| v.as_array())
            .or_else(|| {
                value
                    .get("site")
                    .and_then(|v| v.get("categories"))
                    .and_then(|v| v.as_array())
            })
            .ok_or_else(|| anyhow!("site.json missing categories list"))?;
        let mut categories = Vec::new();
        for item in array {
            if let Ok(cat) = serde_json::from_value::<CategoryInfo>(item.clone()) {
                categories.push(cat);
            }
        }
        Ok(categories)
    }

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

    /// Update a post by ID.
    pub fn update_post(&self, post_id: u64, raw: &str) -> Result<()> {
        let payload = [("post[raw]", raw)];
        let response = self
            .put(&format!("/posts/{}.json", post_id))?
            .form(&payload)
            .send()
            .context("updating post")?;
        if !response.status().is_success() {
            return Err(anyhow!("update post failed with {}", response.status()));
        }
        Ok(())
    }

    /// Create a new topic in a category.
    pub fn create_topic(&self, category_id: u64, title: &str, raw: &str) -> Result<u64> {
        let payload = [
            ("title", title),
            ("raw", raw),
            ("category", &category_id.to_string()),
        ];
        let response = self
            .post("/posts.json")?
            .form(&payload)
            .send()
            .context("creating topic")?;
        let status = response.status();
        let text = response.text().context("reading create response body")?;
        if !status.is_success() {
            return Err(anyhow!("create topic failed with {}: {}", status, text));
        }
        let body: CreatePostResponse =
            serde_json::from_str(&text).context("parsing create topic response")?;
        Ok(body.topic_id)
    }

    /// Create a reply post in a topic.
    pub fn create_post(&self, topic_id: u64, raw: &str) -> Result<u64> {
        let payload = [("topic_id", topic_id.to_string()), ("raw", raw.to_string())];
        let response = self
            .post("/posts.json")?
            .form(&payload)
            .send()
            .context("creating post")?;
        let status = response.status();
        let text = response.text().context("reading create response body")?;
        if !status.is_success() {
            return Err(anyhow!("create post failed with {}: {}", status, text));
        }
        let body: CreatePostResponse =
            serde_json::from_str(&text).context("parsing create post response")?;
        Ok(body.id)
    }

    /// Upload a custom emoji.
    pub fn upload_emoji(&self, emoji_path: &Path, emoji_name: &str) -> Result<()> {
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
        let form = reqwest::blocking::multipart::Form::new()
            .part("emoji[image]", part)
            .text("emoji[name]", emoji_name.to_string());

        let response = self
            .post("/admin/customize/emojis")?
            .multipart(form)
            .send()
            .context("uploading emoji")?;
        if !response.status().is_success() {
            return Err(anyhow!("emoji upload failed with {}", response.status()));
        }
        Ok(())
    }

    /// List custom emojis.
    pub fn list_custom_emojis(&self) -> Result<Vec<CustomEmoji>> {
        if let Ok(emojis) = self.list_admin_emojis() {
            return Ok(emojis);
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
            arr
        } else {
            match value
                .get("emojis")
                .and_then(|v| v.as_array())
                .or_else(|| value.get("custom_emojis").and_then(|v| v.as_array()))
            {
                Some(arr) => arr,
                None => return Ok(Vec::new()),
            }
        };
        Ok(extract_emojis_from_array(
            emojis,
            self.baseurl.trim_end_matches('/'),
        ))
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
        let baseurl = self.baseurl.trim_end_matches('/');
        let mut out = Vec::new();
        if let Some(map) = value.get("custom_emoji").and_then(|v| v.as_object()) {
            extract_emojis_from_map(map, baseurl, &mut out);
        } else if let Some(map) = value.get("custom").and_then(|v| v.as_object()) {
            extract_emojis_from_map(map, baseurl, &mut out);
        } else if let Some(map) = value.get("emoji").and_then(|v| v.as_object()) {
            extract_emojis_from_map(map, baseurl, &mut out);
        }
        Ok(out)
    }

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

    /// Create a category with basic fields copied from a source category.
    pub fn create_category(&self, category: &CategoryInfo) -> Result<u64> {
        let mut payload = vec![("name", category.name.clone())];
        if !category.slug.is_empty() {
            payload.push(("slug", category.slug.clone()));
        }
        if let Some(color) = category.color.clone() {
            payload.push(("color", color));
        }
        if let Some(text_color) = category.text_color.clone() {
            payload.push(("text_color", text_color));
        }
        let response = self
            .post("/categories")?
            .form(&payload)
            .send()
            .context("creating category")?;
        let status = response.status();
        let body: CreateCategoryResponse = response.json().context("reading category response")?;
        if !status.is_success() {
            return Err(anyhow!("create category failed with {}", status));
        }
        Ok(body.category.id)
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
        let value: serde_json::Value =
            serde_json::from_str(&text).context("parsing group response json")?;
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
}

fn extract_html_title(html: &str) -> Option<String> {
    let haystack = html.as_bytes();
    let mut lower = Vec::with_capacity(haystack.len());
    for &byte in haystack {
        lower.push(byte.to_ascii_lowercase());
    }
    let open_tag = b"<title>";
    let close_tag = b"</title>";
    let start = find_subslice(&lower, open_tag)? + open_tag.len();
    let end = find_subslice(&lower[start..], close_tag)? + start;
    let title = String::from_utf8_lossy(&haystack[start..end])
        .trim()
        .to_string();
    if title.is_empty() {
        None
    } else {
        Some(title)
    }
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn push_opt(payload: &mut Vec<(String, String)>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        payload.push((key.to_string(), value.to_string()));
    }
}

/// Response payload for site.json.
#[derive(Debug, Deserialize)]
pub struct SiteResponse {
    pub site: SiteInfo,
}

/// Site metadata.
#[derive(Debug, Deserialize)]
pub struct SiteInfo {
    pub title: String,
}

/// Response payload for about.json.
#[derive(Debug, Deserialize)]
pub struct AboutResponse {
    pub about: AboutInfo,
}

/// About metadata.
#[derive(Debug, Deserialize)]
pub struct AboutInfo {
    pub version: Option<String>,
    pub installed_version: Option<String>,
}

/// Response payload for topic JSON.
#[derive(Debug, Deserialize)]
pub struct TopicResponse {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub slug: Option<String>,
    pub post_stream: PostStream,
}

/// Topic post stream.
#[derive(Debug, Deserialize)]
pub struct PostStream {
    pub posts: Vec<Post>,
}

/// Topic post.
#[derive(Debug, Deserialize)]
pub struct Post {
    pub id: u64,
    #[serde(default)]
    pub raw: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CustomEmoji {
    pub name: String,
    pub url: String,
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
        if let Some(url) = value.as_str() {
            out.push(CustomEmoji {
                name: name.to_string(),
                url: normalize_emoji_url(baseurl, url),
            });
        }
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

/// Response payload for category JSON.
#[derive(Debug, Deserialize)]
pub struct CategoryResponse {
    #[serde(default)]
    pub category: Option<CategoryInfo>,
    pub topic_list: TopicList,
}

/// Category metadata.
#[derive(Debug, Deserialize, Clone)]
pub struct CategoryInfo {
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub text_color: Option<String>,
    pub id: Option<u64>,
    #[serde(default)]
    pub subcategory_list: Vec<CategoryInfo>,
    #[serde(default)]
    pub parent_category_id: Option<u64>,
}

/// Response payload for categories.json.
#[derive(Debug, Deserialize)]
pub struct CategoriesResponse {
    pub category_list: CategoryList,
}

/// Category listing.
#[derive(Debug, Deserialize)]
pub struct CategoryList {
    pub categories: Vec<CategoryInfo>,
}

/// Topic list for a category.
#[derive(Debug, Deserialize)]
pub struct TopicList {
    pub topics: Vec<TopicSummary>,
}

/// Topic summary.
#[derive(Debug, Deserialize)]
pub struct TopicSummary {
    pub id: u64,
    pub title: String,
    pub slug: String,
}

/// Group summary.
#[derive(Debug, Deserialize, Clone)]
pub struct GroupSummary {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub full_name: Option<String>,
}

/// Response payload for groups.json.
#[derive(Debug, Deserialize)]
pub struct GroupsResponse {
    pub groups: Vec<GroupSummary>,
}

/// Response payload for group detail.
#[derive(Debug, Deserialize)]
pub struct GroupDetailResponse {
    pub group: GroupDetail,
}

/// Group details with settings used for deep-copy.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GroupDetail {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub full_name: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub grant_trust_level: Option<u64>,
    #[serde(default)]
    pub visibility_level: Option<u64>,
    #[serde(default)]
    pub mentionable_level: Option<u64>,
    #[serde(default)]
    pub messageable_level: Option<u64>,
    #[serde(default)]
    pub default_notification_level: Option<u64>,
    #[serde(default)]
    pub members_visibility_level: Option<u64>,
    #[serde(default)]
    pub primary_group: Option<bool>,
    #[serde(default)]
    pub public_admission: Option<bool>,
    #[serde(default)]
    pub public_exit: Option<bool>,
    #[serde(default)]
    pub allow_membership_requests: Option<bool>,
    #[serde(default)]
    pub automatic_membership_email_domains: Option<String>,
    #[serde(default)]
    pub automatic_membership_retroactive: Option<bool>,
    #[serde(default)]
    pub membership_request_template: Option<String>,
    #[serde(default)]
    pub flair_icon: Option<String>,
    #[serde(default)]
    pub flair_upload_id: Option<u64>,
    #[serde(default)]
    pub flair_color: Option<String>,
    #[serde(default)]
    pub flair_background_color: Option<String>,
    #[serde(default)]
    pub bio_raw: Option<String>,
}

/// Response payload for creating a post/topic.
#[derive(Debug, Deserialize)]
pub struct CreatePostResponse {
    pub id: u64,
    pub topic_id: u64,
}

/// Response payload for creating a category.
#[derive(Debug, Deserialize)]
pub struct CreateCategoryResponse {
    pub category: CreatedCategory,
}

/// Created category payload.
#[derive(Debug, Deserialize)]
pub struct CreatedCategory {
    pub id: u64,
}

/// Response payload for creating a group.
#[derive(Debug, Deserialize)]
pub struct CreateGroupResponse {
    pub group: CreatedGroup,
}

/// Created group payload.
#[derive(Debug, Deserialize)]
pub struct CreatedGroup {
    pub id: u64,
}
