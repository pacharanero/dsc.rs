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

/// Parse a `--since`-style value. Accepts either a relative duration
/// (`7d`, `24h`, `30m`, `1w`, `90s`) or an ISO-8601 absolute timestamp
/// (`2026-04-01`, `2026-04-01T12:00:00Z`). Returns the resulting cutoff
/// instant (now - duration, or the ISO value itself).
pub fn parse_since_cutoff(input: &str) -> anyhow::Result<chrono::DateTime<chrono::Utc>> {
    use anyhow::anyhow;
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("empty --since value"));
    }

    if let Some(duration) = parse_relative_duration(trimmed) {
        return Ok(chrono::Utc::now() - duration);
    }

    // Try RFC3339 (full timestamp).
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(trimmed) {
        return Ok(dt.with_timezone(&chrono::Utc));
    }
    // Try date-only — treat as midnight UTC.
    if let Ok(d) = chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        return Ok(
            chrono::NaiveDateTime::new(d, chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                .and_utc(),
        );
    }

    Err(anyhow!(
        "unrecognised --since value: {:?} (expected e.g. `7d`, `24h`, `30m`, `1w`, or an ISO-8601 timestamp)",
        input
    ))
}

/// Parse a relative duration like `7d`, `24h`, `1w`, `1m`, `90s`, `1y`.
///
/// Calendar units (`m`, `y`) are imprecise; for windows we use these
/// conventions:
///
/// - `s` — seconds
/// - `min` — minutes (use this rather than `m` to avoid the months-vs-minutes
///   ambiguity)
/// - `h` — hours
/// - `d` — days
/// - `w` — weeks (= 7 days)
/// - `m` — **months** (= 30 days; matches what most users mean by "1m" in
///   analytics windows)
/// - `y` — years (= 365 days)
///
/// For exact calendar math, pass an ISO-8601 timestamp instead.
pub fn parse_relative_duration(input: &str) -> Option<chrono::Duration> {
    let s = input.trim();
    if s.len() < 2 {
        return None;
    }
    // Order matters: `min` must be tried before `m` so we don't read
    // "10min" as "10mi" + "n".
    let multi_char_units = [("min", 60i64)];
    for (suffix, secs_per_unit) in multi_char_units {
        if let Some(digits) = s.strip_suffix(suffix) {
            let n: i64 = digits.parse().ok()?;
            return Some(chrono::Duration::seconds(n * secs_per_unit));
        }
    }
    let (digits, unit) = s.split_at(s.len() - 1);
    let n: i64 = digits.parse().ok()?;
    match unit {
        "s" => Some(chrono::Duration::seconds(n)),
        "h" => Some(chrono::Duration::hours(n)),
        "d" => Some(chrono::Duration::days(n)),
        "w" => Some(chrono::Duration::weeks(n)),
        "m" => Some(chrono::Duration::days(n * 30)),
        "y" => Some(chrono::Duration::days(n * 365)),
        _ => None,
    }
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

    #[test]
    fn parse_relative_duration_common_units() {
        assert_eq!(
            parse_relative_duration("7d"),
            Some(chrono::Duration::days(7))
        );
        assert_eq!(
            parse_relative_duration("24h"),
            Some(chrono::Duration::hours(24))
        );
        assert_eq!(
            parse_relative_duration("30min"),
            Some(chrono::Duration::minutes(30))
        );
        assert_eq!(
            parse_relative_duration("1w"),
            Some(chrono::Duration::weeks(1))
        );
        assert_eq!(
            parse_relative_duration("90s"),
            Some(chrono::Duration::seconds(90))
        );
    }

    #[test]
    fn parse_relative_duration_rejects_nonsense() {
        assert!(parse_relative_duration("").is_none());
        assert!(parse_relative_duration("d").is_none());
        assert!(parse_relative_duration("7x").is_none());
        assert!(parse_relative_duration("abc").is_none());
        assert!(parse_relative_duration("3M").is_none()); // case-sensitive
    }

    #[test]
    fn parse_relative_duration_treats_m_as_months() {
        // `m` = months (= 30 days). Users naturally write `1m` for "one
        // month" in analytics windows; we match that. Use `min` for the
        // rare minutes case.
        assert_eq!(
            parse_relative_duration("1m"),
            Some(chrono::Duration::days(30))
        );
        assert_eq!(
            parse_relative_duration("3m"),
            Some(chrono::Duration::days(90))
        );
    }

    #[test]
    fn parse_relative_duration_minutes_via_min_suffix() {
        assert_eq!(
            parse_relative_duration("5min"),
            Some(chrono::Duration::minutes(5))
        );
        assert_eq!(
            parse_relative_duration("90min"),
            Some(chrono::Duration::minutes(90))
        );
    }

    #[test]
    fn parse_relative_duration_accepts_years_as_365d() {
        assert_eq!(
            parse_relative_duration("1y"),
            Some(chrono::Duration::days(365))
        );
        assert_eq!(
            parse_relative_duration("2y"),
            Some(chrono::Duration::days(730))
        );
    }

    #[test]
    fn parse_since_cutoff_iso_date() {
        let cutoff = parse_since_cutoff("2026-01-01").unwrap();
        assert_eq!(cutoff.to_rfc3339(), "2026-01-01T00:00:00+00:00");
    }

    #[test]
    fn parse_since_cutoff_iso_timestamp() {
        let cutoff = parse_since_cutoff("2026-04-15T12:30:00Z").unwrap();
        assert_eq!(cutoff.to_rfc3339(), "2026-04-15T12:30:00+00:00");
    }

    #[test]
    fn parse_since_cutoff_relative_is_in_the_past() {
        let now = chrono::Utc::now();
        let cutoff = parse_since_cutoff("7d").unwrap();
        let diff = now - cutoff;
        // Should be very close to 7 days (within a second).
        assert!(
            (diff - chrono::Duration::days(7)).num_seconds().abs() < 2,
            "expected ~7 day delta, got {}",
            diff
        );
    }

    #[test]
    fn parse_since_cutoff_rejects_garbage() {
        assert!(parse_since_cutoff("not a date").is_err());
        assert!(parse_since_cutoff("").is_err());
    }
}

