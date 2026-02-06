use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Trim trailing slashes from a base URL.
pub fn normalize_baseurl(baseurl: &str) -> String {
    baseurl.trim_end_matches('/').to_string()
}

/// Create a URL-safe slug from arbitrary input.
pub fn slugify(input: &str) -> String {
    let out = input
        .to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .collect::<Vec<_>>()
        .join("-")
        .trim_end_matches('-')
        .to_string();

    if out.is_empty() {
        "untitled".to_string()
    } else {
        out
    }
}

/// Ensure a directory exists.
pub fn ensure_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).context(format!("creating {}", path.display()))
}

/// Resolve a topic path from a user-provided path and a topic title.
pub fn resolve_topic_path(
    provided: Option<&Path>,
    title: &str,
    default_dir: &Path,
) -> Result<PathBuf> {
    let filename = format!("{}.md", slugify(title));
    match provided {
        Some(path) if path.exists() && path.is_dir() => Ok(path.join(filename)),
        Some(path) if path.extension().is_some() => Ok(path.to_path_buf()),
        Some(path) => Ok(path.join(filename)),
        None => Ok(default_dir.join(filename)),
    }
}

/// Read a Markdown file.
pub fn read_markdown(path: &Path) -> Result<String> {
    fs::read_to_string(path).context(format!("reading {}", path.display()))
}

/// Write a Markdown file, creating parent directories if needed.
pub fn write_markdown(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    fs::write(path, content).context(format!("writing {}", path.display()))
}
