use anyhow::{Context, Result};
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

/// Trim trailing slashes from a base URL.
pub fn normalize_baseurl(baseurl: &str) -> String {
    baseurl.trim_end_matches('/').to_string()
}

/// Create a URL-safe slug from arbitrary input.
pub fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    while out.starts_with('-') {
        out.remove(0);
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "untitled".to_string()
    } else {
        out
    }
}

/// Ensure a directory exists.
pub fn ensure_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("creating {}", path.display()))?;
    Ok(())
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
    let raw = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(raw)
}

/// Write a Markdown file, creating parent directories if needed.
pub fn write_markdown(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent)?;
    }
    fs::write(path, content).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn color_mode() -> &'static str {
    match std::env::var("DSC_COLOR") {
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "always" => "always",
            "never" => "never",
            _ => "auto",
        },
        Err(_) => "auto",
    }
}

fn color_allowed_for_stdout() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    match color_mode() {
        "always" => true,
        "never" => false,
        _ => std::io::stdout().is_terminal(),
    }
}

fn discourse_color_code(key: &str) -> u8 {
    const COLORS: [u8; 12] = [31, 32, 33, 34, 35, 36, 91, 92, 93, 94, 95, 96];
    let hash = key.bytes().fold(0usize, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(b as usize)
    });
    COLORS[hash % COLORS.len()]
}

pub fn color_discourse_label(label: &str, key: &str) -> String {
    if !color_allowed_for_stdout() {
        return label.to_string();
    }
    let code = discourse_color_code(key);
    format!("\x1b[1;{}m{}\x1b[0m", code, label)
}
