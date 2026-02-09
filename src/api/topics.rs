use super::client::DiscourseClient;
use super::models::{CreatePostResponse, TopicResponse};
use anyhow::{anyhow, Context, Result};
use serde_json::Value;

impl DiscourseClient {
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
}
