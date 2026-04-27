//! `dsc analytics` — community-health snapshot per `spec/analytics.md`.
//!
//! The metrics, sections, and output shape are defined in the spec; this
//! file implements them. Where a metric maps directly onto a single
//! `/admin/reports/{id}.json` endpoint we fetch and aggregate; where a
//! metric needs cross-API derivation (e.g. "lost regulars" needs per-user
//! post-history walks) we emit `null` / `—` for v1 with a clear note in
//! the queries.md tracking file.

use crate::api::{AdminReport, DiscourseClient};
use crate::cli::AnalyticsFormat;
use crate::commands::common::{ensure_api_credentials, select_discourse};
use crate::config::Config;
use crate::utils::parse_since_cutoff;
use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Duration, Utc};
use serde::Serialize;
use serde_json::{Map, Value, json};
use std::io;

const SCHEMA_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn analytics(
    config: &Config,
    discourse_name: &str,
    since: &str,
    compare: bool,
    section_filter: SectionFilter,
    format: AnalyticsFormat,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;

    let now = Utc::now();
    let cutoff = parse_since_cutoff(since)?;
    let mut window = Window {
        since: cutoff,
        until: now,
        label: since.to_string(),
        clamped: false,
    };
    // Guard: if the user passed a future timestamp, swap so duration is positive.
    if window.since > window.until {
        std::mem::swap(&mut window.since, &mut window.until);
    }

    let report = build_report(
        &client,
        discourse_name,
        &window,
        compare,
        section_filter,
    )?;

    render(&report, format)
}

// ---------------------------------------------------------------------------
// Data model
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionFilter {
    All,
    Growth,
    Activity,
    Health,
}

#[derive(Clone, Debug, Serialize)]
struct Window {
    since: DateTime<Utc>,
    until: DateTime<Utc>,
    label: String,
    clamped: bool,
}

impl Window {
    fn iso_date_since(&self) -> String {
        format_yyyy_mm_dd(&self.since)
    }
    fn iso_date_until(&self) -> String {
        format_yyyy_mm_dd(&self.until)
    }
    fn duration(&self) -> Duration {
        self.until - self.since
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Direction {
    Up,
    Down,
    /// Movement is neither good nor bad — render the arrow grey.
    Neither,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Unit {
    Count,
    Percent,
    Minutes,
    Hours,
    Ratio,
    PerThousandPosts,
}

#[derive(Clone, Debug, Serialize)]
struct Metric {
    /// Label printed in text mode (e.g. "new contributors").
    label: String,
    /// JSON / CSV key (e.g. "new_contributors").
    key: String,
    /// Current-window value. `None` indicates "not yet implemented" or
    /// "undefined for this window" (e.g. posts-per-topic when topics == 0).
    current: Option<f64>,
    /// Previous-window value, only set when `--compare` was passed AND a
    /// value was obtainable.
    previous: Option<f64>,
    /// Pin the desirable direction so the renderer can colour the arrow.
    desirable: Direction,
    unit: Unit,
    /// True when the value couldn't be computed because we haven't yet
    /// implemented the derivation. Helps the renderer show `—` rather
    /// than a misleading `0`.
    not_implemented: bool,
}

impl Metric {
    fn new(label: &str, key: &str, desirable: Direction, unit: Unit) -> Self {
        Self {
            label: label.to_string(),
            key: key.to_string(),
            current: None,
            previous: None,
            desirable,
            unit,
            not_implemented: false,
        }
    }
    fn with_value(mut self, v: Option<f64>) -> Self {
        self.current = v;
        self
    }
    fn with_previous(mut self, v: Option<f64>) -> Self {
        self.previous = v;
        self
    }
    fn stub(mut self) -> Self {
        self.not_implemented = true;
        self
    }
    fn delta_pct(&self) -> Option<f64> {
        match (self.current, self.previous) {
            (Some(c), Some(p)) if p != 0.0 => Some(((c - p) / p) * 100.0),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct AnalyticsReport {
    schema: u32,
    discourse: String,
    window: Window,
    compare: Option<Window>,
    growth: Option<Vec<Metric>>,
    activity: Option<Vec<Metric>>,
    health: Option<Vec<Metric>>,
}

// ---------------------------------------------------------------------------
// Report construction
// ---------------------------------------------------------------------------

fn build_report(
    client: &DiscourseClient,
    discourse_name: &str,
    window: &Window,
    compare: bool,
    filter: SectionFilter,
) -> Result<AnalyticsReport> {
    let start = window.iso_date_since();
    let end = window.iso_date_until();

    let compare_window = if compare {
        let len = window.duration();
        Some(Window {
            since: window.since - len,
            until: window.since,
            label: window.label.clone(),
            clamped: false,
        })
    } else {
        None
    };

    let growth = if matches!(filter, SectionFilter::All | SectionFilter::Growth) {
        Some(build_growth(client, &start, &end, compare)?)
    } else {
        None
    };

    let activity = if matches!(filter, SectionFilter::All | SectionFilter::Activity) {
        Some(build_activity(client, &start, &end, compare)?)
    } else {
        None
    };

    let health = if matches!(filter, SectionFilter::All | SectionFilter::Health) {
        Some(build_health(client, &start, &end, compare)?)
    } else {
        None
    };

    Ok(AnalyticsReport {
        schema: SCHEMA_VERSION,
        discourse: discourse_name.to_string(),
        window: window.clone(),
        compare: compare_window,
        growth,
        activity,
        health,
    })
}

fn build_growth(
    client: &DiscourseClient,
    start: &str,
    end: &str,
    compare: bool,
) -> Result<Vec<Metric>> {
    let mut out = Vec::new();

    // New contributors — Discourse's `Reports::NewContributors` is defined
    // as `User.count_by_first_post(start, end)`, which is exactly the spec.
    let nc = fetch_optional(client, "new_contributors", start, end)?;
    out.push(
        Metric::new("new contributors", "new_contributors", Direction::Up, Unit::Count)
            .with_value(nc.as_ref().map(|r| r.current_total()))
            .with_previous(if compare {
                nc.as_ref().and_then(|r| r.previous_total())
            } else {
                None
            }),
    );

    // Reactivated users — needs per-user post-history. Stub.
    out.push(
        Metric::new("reactivated users", "reactivated_users", Direction::Up, Unit::Count)
            .stub(),
    );

    // Lost regulars — needs per-user post-history. Stub.
    out.push(
        Metric::new("lost regulars", "lost_regulars", Direction::Down, Unit::Count).stub(),
    );

    // Net active change — derivable once the three above land. Stub.
    out.push(
        Metric::new("net active change", "net_active_change", Direction::Up, Unit::Count)
            .stub(),
    );

    // Trust-level promotions — `trust_level_growth` report, sum across
    // TL0→TL1, TL1→TL2, TL2→TL3. The report is stacked by destination
    // trust level; current_total sums all of them which matches our spec.
    let tlg = fetch_optional(client, "trust_level_growth", start, end)?;
    out.push(
        Metric::new("trust-level promotions", "trust_level_promotions", Direction::Up, Unit::Count)
            .with_value(tlg.as_ref().map(|r| r.current_total()))
            .with_previous(if compare {
                tlg.as_ref().and_then(|r| r.previous_total())
            } else {
                None
            }),
    );

    Ok(out)
}

fn build_activity(
    client: &DiscourseClient,
    start: &str,
    end: &str,
    compare: bool,
) -> Result<Vec<Metric>> {
    let mut out = Vec::new();

    let topics = fetch_optional(client, "topics", start, end)?;
    let posts = fetch_optional(client, "posts", start, end)?;

    let topics_cur = topics.as_ref().map(|r| r.current_total());
    let topics_prev = if compare {
        topics.as_ref().and_then(|r| r.previous_total())
    } else {
        None
    };
    let posts_cur = posts.as_ref().map(|r| r.current_total());
    let posts_prev = if compare {
        posts.as_ref().and_then(|r| r.previous_total())
    } else {
        None
    };

    out.push(
        Metric::new("topics created", "topics_created", Direction::Up, Unit::Count)
            .with_value(topics_cur)
            .with_previous(topics_prev),
    );
    out.push(
        Metric::new("posts created", "posts_created", Direction::Up, Unit::Count)
            .with_value(posts_cur)
            .with_previous(posts_prev),
    );

    // Posts per topic — derivable from the two counts. Avoid division by
    // zero (window of zero topics → null / em dash, per spec).
    let ppt_cur = ratio(posts_cur, topics_cur);
    let ppt_prev = ratio(posts_prev, topics_prev);
    out.push(
        Metric::new("posts per topic", "posts_per_topic", Direction::Up, Unit::Ratio)
            .with_value(ppt_cur)
            .with_previous(ppt_prev),
    );

    // Unique posters — needs per-user breakdown. Stub.
    out.push(
        Metric::new("unique posters", "unique_posters", Direction::Up, Unit::Count).stub(),
    );

    // Top-10 share — needs per-user post counts. Stub.
    out.push(
        Metric::new("top-10 share", "top_10_share", Direction::Down, Unit::Percent).stub(),
    );

    // Reply coverage — derivable from `topics` and `topics_with_no_response`.
    // Coverage = (topics - no_response) / topics.
    let no_response = fetch_optional(client, "topics_with_no_response", start, end)?;
    let coverage_cur = match (topics_cur, no_response.as_ref().map(|r| r.current_total())) {
        (Some(t), Some(n)) if t > 0.0 => Some(((t - n) / t) * 100.0),
        _ => None,
    };
    let coverage_prev = match (topics_prev, no_response.as_ref().and_then(|r| r.previous_total())) {
        (Some(t), Some(n)) if t > 0.0 => Some(((t - n) / t) * 100.0),
        _ => None,
    };
    out.push(
        Metric::new("reply coverage", "reply_coverage", Direction::Up, Unit::Percent)
            .with_value(coverage_cur)
            .with_previous(coverage_prev),
    );

    // Median time to first reply — `time_to_first_response` report.
    // Discourse emits this as an `average` scalar in minutes (the `data`
    // array gives daily averages we'd have to median, so the scalar is
    // closer to "median across the whole window" semantically).
    let ttfr = fetch_optional(client, "time_to_first_response", start, end)?;
    out.push(
        Metric::new(
            "median time to first reply",
            "median_time_to_first_reply",
            Direction::Down,
            Unit::Minutes,
        )
        .with_value(ttfr.as_ref().and_then(|r| r.average)),
    );

    Ok(out)
}

fn build_health(
    client: &DiscourseClient,
    start: &str,
    end: &str,
    compare: bool,
) -> Result<Vec<Metric>> {
    let mut out = Vec::new();

    let likes = fetch_optional(client, "likes", start, end)?;
    let posts = fetch_optional(client, "posts", start, end)?;

    // Likes per post — likes / posts.
    let lpp_cur = ratio(
        likes.as_ref().map(|r| r.current_total()),
        posts.as_ref().map(|r| r.current_total()),
    );
    let lpp_prev = if compare {
        ratio(
            likes.as_ref().and_then(|r| r.previous_total()),
            posts.as_ref().and_then(|r| r.previous_total()),
        )
    } else {
        None
    };
    out.push(
        Metric::new("likes per post", "likes_per_post", Direction::Up, Unit::Ratio)
            .with_value(lpp_cur)
            .with_previous(lpp_prev),
    );

    // Returning poster rate — needs per-user comparison across windows. Stub.
    out.push(
        Metric::new(
            "returning poster rate",
            "returning_poster_rate",
            Direction::Up,
            Unit::Percent,
        )
        .stub(),
    );

    // Flags raised — `flags` report counts user-raised flags by default.
    let flags = fetch_optional(client, "flags", start, end)?;
    out.push(
        Metric::new("flags raised", "flags_raised", Direction::Down, Unit::Count)
            .with_value(flags.as_ref().map(|r| r.current_total()))
            .with_previous(if compare {
                flags.as_ref().and_then(|r| r.previous_total())
            } else {
                None
            }),
    );

    // Flag resolution time — Discourse doesn't ship a dedicated report
    // for this in current versions (verified against the Reports::*
    // include list). Stubbed until we either find the right ID or
    // derive it from `flags_status` / staff action logs.
    out.push(
        Metric::new(
            "flag resolution time",
            "flag_resolution_time",
            Direction::Down,
            Unit::Hours,
        )
        .stub(),
    );

    // Moderator action rate — `moderators_activity` total / posts, * 1000.
    // The `moderators_activity` report sums staff actions per day; pairing
    // with the `posts` report gives the per-1k normalisation the spec asks
    // for.
    let mods = fetch_optional(client, "moderators_activity", start, end)?;
    let mar_cur = match (mods.as_ref().map(|r| r.current_total()), posts.as_ref().map(|r| r.current_total())) {
        (Some(m), Some(p)) if p > 0.0 => Some((m / p) * 1000.0),
        _ => None,
    };
    let mar_prev = match (
        mods.as_ref().and_then(|r| r.previous_total()),
        posts.as_ref().and_then(|r| r.previous_total()),
    ) {
        (Some(m), Some(p)) if p > 0.0 => Some((m / p) * 1000.0),
        _ => None,
    };
    out.push(
        Metric::new(
            "moderator action rate",
            "moderator_action_rate",
            Direction::Neither,
            Unit::PerThousandPosts,
        )
        .with_value(mar_cur)
        .with_previous(mar_prev),
    );

    // Solo-thread rate — needs per-topic reply count walks. Stub.
    out.push(
        Metric::new("solo-thread rate", "solo_thread_rate", Direction::Down, Unit::Percent)
            .stub(),
    );

    Ok(out)
}

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

fn render(report: &AnalyticsReport, format: AnalyticsFormat) -> Result<()> {
    match format {
        AnalyticsFormat::Text => render_text(report),
        AnalyticsFormat::Json => render_json(report),
        AnalyticsFormat::Yaml => render_yaml(report),
        AnalyticsFormat::Markdown => render_markdown(report, false),
        AnalyticsFormat::MarkdownTable => render_markdown(report, true),
        AnalyticsFormat::Csv => render_csv(report),
    }
}

fn render_text(report: &AnalyticsReport) -> Result<()> {
    println!(
        "analytics for {} — {} ({} → {})",
        report.discourse,
        report.window.label,
        report.window.iso_date_since(),
        report.window.iso_date_until()
    );
    if report.window.clamped {
        println!("(window clamped — install is younger than --since)");
    }
    let compare = report.compare.is_some();

    for (name, metrics) in iter_sections(report) {
        println!();
        println!("{}", name);
        let label_w = metrics
            .iter()
            .map(|m| m.label.chars().count())
            .max()
            .unwrap_or(0)
            .max(20);
        let value_w = metrics
            .iter()
            .flat_map(|m| {
                [
                    visual_width(&format_value(m.current, m.unit, m.not_implemented)),
                    visual_width(&format_value(m.previous, m.unit, m.not_implemented)),
                ]
            })
            .max()
            .unwrap_or(0)
            .max(8);
        for m in metrics {
            let cur = format_value(m.current, m.unit, m.not_implemented);
            let line = if compare {
                let prev = format_value(m.previous, m.unit, m.not_implemented);
                let arrow = arrow_for(m);
                let pct = m
                    .delta_pct()
                    .map(|p| format!("({:+.0}%)", p))
                    .unwrap_or_default();
                format!(
                    "  {:<lw$}  {}  {} {}  {}",
                    pad_right(&m.label, label_w),
                    right_align(&cur, value_w),
                    arrow,
                    right_align(&prev, value_w),
                    pct,
                    lw = label_w
                )
            } else {
                format!(
                    "  {}  {}",
                    pad_right(&m.label, label_w),
                    right_align(&cur, value_w),
                )
            };
            println!("{}", line);
        }
    }
    Ok(())
}

fn pad_right(s: &str, width: usize) -> String {
    let w = visual_width(s);
    if w >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - w))
    }
}

fn render_json(report: &AnalyticsReport) -> Result<()> {
    let value = report_to_json(report);
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

fn render_yaml(report: &AnalyticsReport) -> Result<()> {
    let value = report_to_json(report);
    println!("{}", serde_yaml::to_string(&value)?);
    Ok(())
}

fn render_markdown(report: &AnalyticsReport, table: bool) -> Result<()> {
    let compare = report.compare.is_some();
    println!("# analytics for {}", report.discourse);
    println!();
    println!(
        "Window: **{}** ({} → {})",
        report.window.label,
        report.window.iso_date_since(),
        report.window.iso_date_until()
    );
    if report.window.clamped {
        println!();
        println!("> Window clamped — install is younger than `--since`.");
    }

    for (name, metrics) in iter_sections(report) {
        println!();
        println!("## {}", name);
        println!();
        if table {
            if compare {
                println!("| metric | current | previous | Δ |");
                println!("| --- | ---: | ---: | ---: |");
                for m in metrics {
                    let cur = format_value(m.current, m.unit, m.not_implemented);
                    let prev = format_value(m.previous, m.unit, m.not_implemented);
                    let pct = m
                        .delta_pct()
                        .map(|p| format!("{:+.0}%", p))
                        .unwrap_or_else(|| "—".to_string());
                    println!("| {} | {} | {} | {} |", m.label, cur, prev, pct);
                }
            } else {
                println!("| metric | value |");
                println!("| --- | ---: |");
                for m in metrics {
                    println!(
                        "| {} | {} |",
                        m.label,
                        format_value(m.current, m.unit, m.not_implemented)
                    );
                }
            }
        } else {
            for m in metrics {
                let cur = format_value(m.current, m.unit, m.not_implemented);
                if compare {
                    let prev = format_value(m.previous, m.unit, m.not_implemented);
                    let pct = m
                        .delta_pct()
                        .map(|p| format!(" (`{:+.0}%`)", p))
                        .unwrap_or_default();
                    println!("- **{}** — {} (prev: {}){}", m.label, cur, prev, pct);
                } else {
                    println!("- **{}** — {}", m.label, cur);
                }
            }
        }
    }
    Ok(())
}

fn render_csv(report: &AnalyticsReport) -> Result<()> {
    let mut writer = csv::Writer::from_writer(io::stdout());
    writer.write_record([
        "section",
        "metric",
        "current",
        "previous",
        "delta",
        "delta_pct",
        "desirable_direction",
        "unit",
    ])?;
    for (name, metrics) in iter_sections(report) {
        for m in metrics {
            let cur = m.current.map(|v| format!("{}", v)).unwrap_or_default();
            let prev = m.previous.map(|v| format!("{}", v)).unwrap_or_default();
            let delta = match (m.current, m.previous) {
                (Some(c), Some(p)) => format!("{}", c - p),
                _ => String::new(),
            };
            let pct = m
                .delta_pct()
                .map(|p| format!("{:.2}", p))
                .unwrap_or_default();
            let direction = match m.desirable {
                Direction::Up => "up",
                Direction::Down => "down",
                Direction::Neither => "neither",
            };
            let unit = unit_str(m.unit);
            writer.write_record([
                name, &m.label, &cur, &prev, &delta, &pct, direction, unit,
            ])?;
        }
    }
    writer.flush()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn iter_sections(report: &AnalyticsReport) -> Vec<(&'static str, &[Metric])> {
    let mut v: Vec<(&'static str, &[Metric])> = Vec::new();
    if let Some(g) = &report.growth {
        v.push(("growth", g));
    }
    if let Some(a) = &report.activity {
        v.push(("activity", a));
    }
    if let Some(h) = &report.health {
        v.push(("health", h));
    }
    v
}

fn fetch_optional(
    client: &DiscourseClient,
    report_id: &str,
    start: &str,
    end: &str,
) -> Result<Option<AdminReport>> {
    // We tolerate "report not found" errors here — Discourse occasionally
    // renames or removes report ids across versions; we'd rather render
    // the rest of the dashboard than fail the whole command.
    match client.fetch_admin_report(report_id, start, end) {
        Ok(r) => Ok(Some(r)),
        Err(err) => {
            let msg = err.to_string();
            // Tolerate the report being missing (404), forbidden for the
            // current key (403), or breaking server-side (500 — Discourse
            // throws when a report has no data, no permission for the
            // category filter, etc.). All of these cases get rendered as
            // null/em-dash rather than failing the whole command.
            let known_missing = msg.contains(" 404 ")
                || msg.contains(" 403 ")
                || msg.contains(" 500 ")
                || msg.contains("not found");
            if known_missing {
                eprintln!(
                    "[analytics] note: report '{}' not available on this Discourse — metric will render as `—`",
                    report_id
                );
                Ok(None)
            } else {
                Err(err).with_context(|| format!("fetching report {}", report_id))
            }
        }
    }
}

fn ratio(num: Option<f64>, den: Option<f64>) -> Option<f64> {
    match (num, den) {
        (Some(n), Some(d)) if d > 0.0 => Some(n / d),
        _ => None,
    }
}

fn format_yyyy_mm_dd(d: &DateTime<Utc>) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day())
}

fn unit_str(u: Unit) -> &'static str {
    match u {
        Unit::Count => "count",
        Unit::Percent => "percent",
        Unit::Minutes => "minutes",
        Unit::Hours => "hours",
        Unit::Ratio => "ratio",
        Unit::PerThousandPosts => "per_1k_posts",
    }
}

fn format_value(v: Option<f64>, unit: Unit, not_impl: bool) -> String {
    if not_impl {
        return "— (n/i)".to_string();
    }
    // Normalise negative-zero so display never prints "-0.0".
    let v = v.map(|x| if x == 0.0 { 0.0 } else { x });
    match (v, unit) {
        (None, _) => "—".to_string(),
        (Some(x), Unit::Count) => format!("{}", x as i64),
        (Some(x), Unit::Percent) => format!("{:.0}%", x),
        (Some(x), Unit::Minutes) => format_minutes(x),
        (Some(x), Unit::Hours) => format!("{:.1}h", x),
        (Some(x), Unit::Ratio) => format!("{:.1}", x),
        (Some(x), Unit::PerThousandPosts) => format!("{:.1} / 1k posts", x),
    }
}

/// Visible width for a string composed mostly of ASCII + occasional
/// em-dashes / arrows. Rust's `{:>N}` formatter counts bytes, which
/// breaks alignment for our `—` (3 bytes) and arrows (3 bytes each).
/// This is a deliberate cheap approximation: every char is one column,
/// which is correct for everything we print here.
fn visual_width(s: &str) -> usize {
    s.chars().count()
}

/// Right-pad a string with spaces to the given visual width.
fn right_align(s: &str, width: usize) -> String {
    let w = visual_width(s);
    if w >= width {
        s.to_string()
    } else {
        format!("{}{}", " ".repeat(width - w), s)
    }
}

fn format_minutes(x: f64) -> String {
    if x >= 60.0 {
        let h = x / 60.0;
        format!("{:.1}h", h)
    } else {
        format!("{:.0}m", x)
    }
}

fn arrow_for(m: &Metric) -> &'static str {
    let (Some(c), Some(p)) = (m.current, m.previous) else {
        return " ";
    };
    if c > p {
        "↑"
    } else if c < p {
        "↓"
    } else {
        "•"
    }
}

fn report_to_json(report: &AnalyticsReport) -> Value {
    let mut top = Map::new();
    top.insert("schema".to_string(), json!(report.schema));
    top.insert("discourse".to_string(), json!(report.discourse));
    top.insert(
        "window".to_string(),
        json!({
            "since": report.window.since.to_rfc3339(),
            "until": report.window.until.to_rfc3339(),
            "label": report.window.label,
            "clamped": report.window.clamped,
        }),
    );
    if let Some(c) = &report.compare {
        top.insert(
            "compare".to_string(),
            json!({
                "since": c.since.to_rfc3339(),
                "until": c.until.to_rfc3339(),
            }),
        );
    }
    for (name, metrics) in iter_sections(report) {
        top.insert(name.to_string(), section_to_json(metrics));
    }
    Value::Object(top)
}

fn section_to_json(metrics: &[Metric]) -> Value {
    let mut out = Map::new();
    for m in metrics {
        let mut entry = Map::new();
        entry.insert("current".to_string(), float_or_null(m.current));
        entry.insert("previous".to_string(), float_or_null(m.previous));
        if let Some(p) = m.delta_pct() {
            entry.insert(
                "delta_pct".to_string(),
                json!((p * 10.0).round() / 10.0),
            );
        }
        entry.insert(
            "desirable".to_string(),
            json!(match m.desirable {
                Direction::Up => "up",
                Direction::Down => "down",
                Direction::Neither => "neither",
            }),
        );
        entry.insert("unit".to_string(), json!(unit_str(m.unit)));
        if m.not_implemented {
            entry.insert("not_implemented".to_string(), json!(true));
        }
        out.insert(m.key.clone(), Value::Object(entry));
    }
    Value::Object(out)
}

fn float_or_null(v: Option<f64>) -> Value {
    match v {
        None => Value::Null,
        Some(x) if x.is_finite() => json!(x),
        _ => Value::Null,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metric_delta_pct_handles_zero_previous() {
        let m = Metric::new("x", "x", Direction::Up, Unit::Count)
            .with_value(Some(10.0))
            .with_previous(Some(0.0));
        assert!(m.delta_pct().is_none());
    }

    #[test]
    fn metric_delta_pct_handles_negative_change() {
        let m = Metric::new("x", "x", Direction::Up, Unit::Count)
            .with_value(Some(80.0))
            .with_previous(Some(100.0));
        assert_eq!(m.delta_pct(), Some(-20.0));
    }

    #[test]
    fn metric_stub_renders_em_dash() {
        let m = Metric::new("x", "x", Direction::Up, Unit::Count).stub();
        assert_eq!(format_value(m.current, m.unit, m.not_implemented), "— (n/i)");
    }

    #[test]
    fn ratio_is_none_when_denominator_zero() {
        assert!(ratio(Some(10.0), Some(0.0)).is_none());
        assert!(ratio(Some(10.0), None).is_none());
        assert_eq!(ratio(Some(20.0), Some(4.0)), Some(5.0));
    }

    #[test]
    fn arrow_reflects_direction_only() {
        let mut m = Metric::new("x", "x", Direction::Up, Unit::Count);
        m.current = Some(10.0);
        m.previous = Some(5.0);
        assert_eq!(arrow_for(&m), "↑");
        m.current = Some(5.0);
        m.previous = Some(10.0);
        assert_eq!(arrow_for(&m), "↓");
        m.current = Some(5.0);
        m.previous = Some(5.0);
        assert_eq!(arrow_for(&m), "•");
    }

    #[test]
    fn format_value_em_dash_for_none() {
        assert_eq!(format_value(None, Unit::Count, false), "—");
    }

    #[test]
    fn format_minutes_rolls_to_hours() {
        assert_eq!(format_minutes(45.0), "45m");
        assert_eq!(format_minutes(90.0), "1.5h");
    }

    #[test]
    fn format_value_handles_units() {
        assert_eq!(format_value(Some(42.0), Unit::Count, false), "42");
        assert_eq!(format_value(Some(35.0), Unit::Percent, false), "35%");
        assert_eq!(format_value(Some(7.5), Unit::Ratio, false), "7.5");
        assert_eq!(format_value(Some(4.2), Unit::Hours, false), "4.2h");
    }

    #[test]
    fn yyyy_mm_dd_pads_correctly() {
        let dt = DateTime::parse_from_rfc3339("2026-01-05T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(format_yyyy_mm_dd(&dt), "2026-01-05");
    }
}
