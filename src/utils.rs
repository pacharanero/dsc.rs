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
    let out = input
        .to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_simple_ascii() {
        assert_eq!(slugify("Hello World"), "hello-world");
    }

    #[test]
    fn slugify_collapses_runs_of_non_alnum() {
        assert_eq!(slugify("a   b___c!!!d"), "a-b-c-d");
    }

    #[test]
    fn slugify_trims_leading_and_trailing_dashes() {
        assert_eq!(slugify("   hello   "), "hello");
        assert_eq!(slugify("!!!foo!!!"), "foo");
    }

    #[test]
    fn slugify_empty_input_returns_untitled() {
        assert_eq!(slugify(""), "untitled");
        assert_eq!(slugify("   "), "untitled");
        assert_eq!(slugify("!!!"), "untitled");
    }

    #[test]
    fn slugify_preserves_numbers() {
        assert_eq!(slugify("Topic 42 - intro"), "topic-42-intro");
    }

    #[test]
    fn slugify_lowercases() {
        assert_eq!(slugify("ABCxyz"), "abcxyz");
    }

    #[test]
    fn normalize_baseurl_strips_trailing_slashes() {
        assert_eq!(normalize_baseurl("https://example.com/"), "https://example.com");
        assert_eq!(normalize_baseurl("https://example.com///"), "https://example.com");
        assert_eq!(normalize_baseurl("https://example.com"), "https://example.com");
    }

    #[test]
    fn normalize_baseurl_preserves_no_trailing() {
        assert_eq!(normalize_baseurl(""), "");
    }

    #[test]
    fn resolve_topic_path_uses_title_when_no_path_given() {
        let default_dir = Path::new("/tmp/dsc-test");
        let out = resolve_topic_path(None, "Hello World", default_dir).unwrap();
        assert_eq!(out, default_dir.join("hello-world.md"));
    }

    #[test]
    fn resolve_topic_path_uses_given_path_with_extension() {
        let default_dir = Path::new("/tmp/dsc-test");
        let explicit = Path::new("/tmp/custom.md");
        let out = resolve_topic_path(Some(explicit), "Ignored", default_dir).unwrap();
        assert_eq!(out, explicit);
    }
}
