use serde::{Deserialize, Serialize};

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
