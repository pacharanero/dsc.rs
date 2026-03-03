use crate::api::{CategoryInfo, DiscourseClient, TopicSummary};
use crate::cli::ListFormat;
use crate::commands::common::{ensure_api_credentials, select_discourse};
use crate::config::Config;
use crate::utils::{ensure_dir, read_markdown, slugify, write_markdown};
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::Path;

pub fn category_list(
    config: &Config,
    discourse_name: &str,
    format: ListFormat,
    verbose: bool,
    tree: bool,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;
    let categories = client.fetch_categories()?;
    let mut flat = Vec::new();
    for category in categories {
        flatten_categories(&category, &mut flat);
    }
    match format {
        ListFormat::Text => {
            if tree {
                if flat.is_empty() && !verbose {
                    println!("No categories found.");
                    return Ok(());
                }
                print_category_tree(&flat);
            } else {
                let unique = unique_categories(flat);
                if unique.is_empty() && !verbose {
                    println!("No categories found.");
                    return Ok(());
                }
                for category in unique {
                    let id = category.id.unwrap_or_default();
                    println!("{} - {}", id, category.name);
                }
            }
        }
        ListFormat::Json => {
            if tree {
                return Err(anyhow!("--tree is only supported with --format text"));
            }
            let unique = unique_categories(flat);
            let raw = serde_json::to_string_pretty(&unique)?;
            println!("{}", raw);
        }
        ListFormat::Yaml => {
            if tree {
                return Err(anyhow!("--tree is only supported with --format text"));
            }
            let unique = unique_categories(flat);
            let raw = serde_yaml::to_string(&unique)?;
            println!("{}", raw);
        }
    }
    Ok(())
}

pub fn category_copy(
    config: &Config,
    source: &str,
    target: Option<&str>,
    category: &str,
) -> Result<()> {
    let source_discourse = select_discourse(config, Some(source))?;
    let target_name = target.unwrap_or(source);
    let target_discourse = select_discourse(config, Some(target_name))?;
    ensure_api_credentials(source_discourse)?;
    ensure_api_credentials(target_discourse)?;
    let source_client = DiscourseClient::new(source_discourse)?;
    let category_id = resolve_category_id(&source_client, category)?;
    let categories = source_client.fetch_categories()?;
    let category = categories
        .into_iter()
        .find(|cat| cat.id == Some(category_id))
        .ok_or_else(|| anyhow!("category not found: {}", category_id))?;
    let mut copied = category.clone();
    copied.name = format!("Copy of {}", category.name);
    copied.slug = format!("{}-copy", category.slug);
    copied.id = None;
    let target_client = DiscourseClient::new(target_discourse)?;
    let new_id = target_client.create_category(&copied)?;
    println!("Category copied successfully with new ID: {}", new_id);
    Ok(())
}

pub fn category_pull(
    config: &Config,
    discourse_name: &str,
    category: &str,
    local_path: Option<&Path>,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;
    let category_id = resolve_category_id(&client, category)?;
    let category = client.fetch_category(category_id)?;
    let dir = match local_path {
        Some(path) => path.to_path_buf(),
        None => {
            let name = category
                .category
                .as_ref()
                .map(|c| c.slug.clone())
                .unwrap_or_else(|| format!("category-{}", category_id));
            std::env::current_dir()?.join(name)
        }
    };
    ensure_dir(&dir)?;
    for topic in category.topic_list.topics {
        let topic_detail = client.fetch_topic(topic.id, true)?;
        let raw = topic_detail
            .post_stream
            .posts
            .get(0)
            .and_then(|p| p.raw.clone())
            .unwrap_or_default();
        let filename = format!("{}.md", slugify(&topic.title));
        write_markdown(&dir.join(filename), &raw)?;
    }
    println!("Category topics pulled to: {}", dir.display());
    Ok(())
}

pub fn category_push(
    config: &Config,
    discourse_name: &str,
    category: &str,
    local_path: &Path,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;
    let category_id = resolve_category_id(&client, category)?;
    let existing = client.fetch_category(category_id)?;
    let mut topics = existing.topic_list.topics;
    let entries =
        fs::read_dir(local_path).with_context(|| format!("reading {}", local_path.display()))?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let raw = read_markdown(&path)?;
        let title = extract_title(&raw)
            .unwrap_or_else(|| path.file_stem().unwrap().to_string_lossy().to_string());
        if let Some(topic) = find_topic_match(&topics, &title, &path) {
            let detail = client.fetch_topic(topic.id, true)?;
            let post = detail
                .post_stream
                .posts
                .get(0)
                .ok_or_else(|| anyhow!("topic has no posts"))?;
            client.update_post(post.id, &raw)?;
        } else {
            let topic_id = client.create_topic(category_id, &title, &raw)?;
            topics.push(TopicSummary {
                id: topic_id,
                title: title.clone(),
                slug: slugify(&title),
            });
        }
    }
    Ok(())
}

fn resolve_category_id(client: &DiscourseClient, category: &str) -> Result<u64> {
    if let Ok(id) = category.parse::<u64>() {
        return Ok(id);
    }
    let slug = category.trim();
    if slug.is_empty() {
        return Err(anyhow!(
            "missing category identifier for category operation"
        ));
    }
    let categories = client.fetch_categories()?;
    let category = categories
        .into_iter()
        .find(|cat| cat.slug == slug)
        .ok_or_else(|| anyhow!("category not found: {}", slug))?;
    category
        .id
        .ok_or_else(|| anyhow!("category not found: {}", slug))
}

fn flatten_categories(category: &CategoryInfo, out: &mut Vec<CategoryInfo>) {
    out.push(category.clone());
    for sub in &category.subcategory_list {
        flatten_categories(sub, out);
    }
}

fn unique_categories(flat: Vec<CategoryInfo>) -> Vec<CategoryInfo> {
    let mut seen = std::collections::HashSet::new();
    let mut unique = Vec::new();
    for category in flat {
        if let Some(id) = category.id {
            if !seen.insert(id) {
                continue;
            }
        }
        unique.push(category);
    }
    unique
}

fn print_category_tree(categories: &[CategoryInfo]) {
    let mut ordered_ids = Vec::new();
    let mut map = std::collections::HashMap::new();
    for category in categories {
        if let Some(id) = category.id {
            if !map.contains_key(&id) {
                ordered_ids.push(id);
                map.insert(id, category.clone());
            }
        }
    }

    let mut children: std::collections::HashMap<u64, Vec<u64>> = std::collections::HashMap::new();
    for category in map.values() {
        if let (Some(id), Some(parent_id)) = (category.id, category.parent_category_id) {
            if map.contains_key(&parent_id) {
                let entry = children.entry(parent_id).or_default();
                if !entry.contains(&id) {
                    entry.push(id);
                }
            }
        }
    }

    let mut roots = Vec::new();
    for id in &ordered_ids {
        if let Some(category) = map.get(id) {
            match category.parent_category_id {
                Some(parent_id) if map.contains_key(&parent_id) => {}
                _ => roots.push(*id),
            }
        }
    }

    let mut seen = std::collections::HashSet::new();
    let last_index = roots.len().saturating_sub(1);
    for (idx, id) in roots.into_iter().enumerate() {
        let is_last = idx == last_index;
        print_category_node(&map, &children, id, "", is_last, &mut seen);
    }
}

fn print_category_node(
    map: &std::collections::HashMap<u64, CategoryInfo>,
    children: &std::collections::HashMap<u64, Vec<u64>>,
    id: u64,
    prefix: &str,
    is_last: bool,
    seen: &mut std::collections::HashSet<u64>,
) {
    if !seen.insert(id) {
        return;
    }
    if let Some(category) = map.get(&id) {
        let branch = if is_last {
            "└── ".to_string()
        } else {
            "├── ".to_string()
        };
        println!("{}{}{} - {}", prefix, branch, id, category.name);
        if let Some(child_ids) = children.get(&id) {
            let new_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };
            let last_index = child_ids.len().saturating_sub(1);
            for (idx, child_id) in child_ids.iter().enumerate() {
                let child_last = idx == last_index;
                print_category_node(map, children, *child_id, &new_prefix, child_last, seen);
            }
        }
    }
}

fn extract_title(raw: &str) -> Option<String> {
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(title) = line.strip_prefix("# ") {
            return Some(title.trim().to_string());
        }
        break;
    }
    None
}

fn find_topic_match<'a>(
    topics: &'a [TopicSummary],
    title: &str,
    path: &Path,
) -> Option<&'a TopicSummary> {
    let slug = slugify(title);
    topics.iter().find(|topic| {
        topic.slug == slug
            || topic.title.eq_ignore_ascii_case(title)
            || path
                .file_stem()
                .map(|s| s.to_string_lossy().eq_ignore_ascii_case(&topic.slug))
                .unwrap_or(false)
    })
}
