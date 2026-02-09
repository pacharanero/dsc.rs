use super::client::DiscourseClient;
use super::models::{CategoriesResponse, CategoryInfo, CategoryResponse, CreateCategoryResponse};
use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use std::collections::HashMap;

impl DiscourseClient {
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
}
