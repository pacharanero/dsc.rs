//! Wrapper around `/admin/reports/{report_id}.json`.
//!
//! Each Discourse report, when called with `start_date` + `end_date`, returns
//! both the requested window (`data`) AND the immediately preceding window
//! of equal length (`prev_data`) in a single response — so `--compare` does
//! not require a second API call for metrics that come straight from a
//! report.

use super::client::DiscourseClient;
use super::error::http_error;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// One day-bucket from a report's `data` / `prev_data` array. `x` is an
/// ISO-8601 date string (YYYY-MM-DD) for daily reports.
///
/// Discourse occasionally emits `y` as `false`, `null`, or a string for
/// non-counter reports. We coerce all of those to `0.0` rather than
/// failing the whole analytics run.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReportPoint {
    #[serde(default)]
    pub x: String,
    #[serde(default, deserialize_with = "deserialize_lenient_f64")]
    pub y: f64,
}

fn deserialize_lenient_f64<'de, D>(de: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error as _;
    let v = serde_json::Value::deserialize(de)?;
    match v {
        serde_json::Value::Number(n) => {
            n.as_f64().ok_or_else(|| D::Error::custom("non-finite number"))
        }
        serde_json::Value::Bool(_) | serde_json::Value::Null => Ok(0.0),
        serde_json::Value::String(s) => s.parse::<f64>().or(Ok(0.0)),
        _ => Ok(0.0),
    }
}

/// Discourse's `data` is normally an array of points but in rare degenerate
/// cases (e.g. some `top_*` reports on freshly-installed sites) it shows
/// up as `false` or `null`. Treat any non-array as an empty list.
fn deserialize_lenient_points<'de, D>(de: D) -> Result<Vec<ReportPoint>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(de)?;
    match v {
        serde_json::Value::Array(_) => {
            serde_json::from_value(v).map_err(serde::de::Error::custom)
        }
        _ => Ok(Vec::new()),
    }
}

/// Same idea for `prev_data`, except it's optional. Discourse emits
/// `prev_data: false` when comparison data isn't available — coerce to None.
fn deserialize_lenient_optional_points<'de, D>(
    de: D,
) -> Result<Option<Vec<ReportPoint>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(de)?;
    match v {
        serde_json::Value::Array(_) => {
            serde_json::from_value(v).map(Some).map_err(serde::de::Error::custom)
        }
        _ => Ok(None),
    }
}

/// Discourse's `average` emits as a number, `false`, or null depending on
/// whether the report has a meaningful average. Coerce false/null/missing
/// to `None`.
fn deserialize_lenient_optional_f64<'de, D>(de: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(de)?;
    match v {
        serde_json::Value::Number(n) => Ok(n.as_f64()),
        serde_json::Value::String(s) => Ok(s.parse::<f64>().ok()),
        _ => Ok(None),
    }
}

/// Raw report payload, distilled from `/admin/reports/{id}.json`. Only the
/// fields `dsc analytics` actually reads are deserialised — Discourse
/// emits a lot more (axis labels, chart modes, descriptions) that we don't
/// care about.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AdminReport {
    #[serde(default, alias = "type")]
    pub report_type: String,
    #[serde(default, deserialize_with = "deserialize_lenient_points")]
    pub data: Vec<ReportPoint>,
    #[serde(default, deserialize_with = "deserialize_lenient_optional_points")]
    pub prev_data: Option<Vec<ReportPoint>>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    #[serde(default)]
    pub prev_start_date: Option<String>,
    #[serde(default)]
    pub prev_end_date: Option<String>,
    /// Some reports (e.g. `time_to_first_response`) emit an `average`
    /// scalar that is more meaningful than summing the daily points.
    /// Discourse occasionally emits this as `false` when no average is
    /// computable; we coerce that to None.
    #[serde(default, deserialize_with = "deserialize_lenient_optional_f64")]
    pub average: Option<f64>,
    /// Whether higher-is-better — Discourse marks this on each report, and
    /// we use it to set the default `desirable` direction when our spec
    /// doesn't already pin one.
    #[serde(default)]
    pub higher_is_better: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ReportEnvelope {
    report: AdminReport,
}

impl AdminReport {
    /// Sum of `data[].y` — for count-per-day reports this is the window total.
    pub fn current_total(&self) -> f64 {
        self.data.iter().map(|p| p.y).sum()
    }

    /// Sum of `prev_data[].y`. Returns `None` when prev_data is absent.
    pub fn previous_total(&self) -> Option<f64> {
        self.prev_data.as_ref().map(|d| d.iter().map(|p| p.y).sum())
    }
}

impl DiscourseClient {
    /// Fetch a single admin report. `start` and `end` are ISO-8601 date
    /// strings (YYYY-MM-DD) — the format Discourse's report controller
    /// expects.
    pub fn fetch_admin_report(
        &self,
        report_id: &str,
        start: &str,
        end: &str,
    ) -> Result<AdminReport> {
        // The report id is taken straight from the spec list; sanity-check
        // it matches the same regex Discourse enforces server-side.
        if !report_id_is_valid(report_id) {
            return Err(anyhow::anyhow!(
                "invalid report id {:?} — must match /^[a-z0-9_]+$/",
                report_id
            ));
        }
        let path = format!(
            "/admin/reports/{}.json?start_date={}&end_date={}",
            report_id, start, end
        );
        let response = self.get(&path)?;
        let status = response.status();
        let text = response.text().context("reading admin report response")?;
        if !status.is_success() {
            return Err(http_error(
                &format!("admin report {} request", report_id),
                status,
                &text,
            ));
        }
        let env: ReportEnvelope =
            serde_json::from_str(&text).with_context(|| {
                format!("parsing admin report {} response", report_id)
            })?;
        Ok(env.report)
    }
}

fn report_id_is_valid(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_id_validation() {
        assert!(report_id_is_valid("signups"));
        assert!(report_id_is_valid("time_to_first_response"));
        assert!(!report_id_is_valid(""));
        assert!(!report_id_is_valid("Signups"));
        assert!(!report_id_is_valid("../../../../etc/passwd"));
        assert!(!report_id_is_valid("topics; rm -rf"));
    }

    #[test]
    fn current_total_sums_data() {
        let r = AdminReport {
            report_type: "signups".to_string(),
            data: vec![
                ReportPoint { x: "2026-04-01".into(), y: 3.0 },
                ReportPoint { x: "2026-04-02".into(), y: 5.0 },
                ReportPoint { x: "2026-04-03".into(), y: 0.0 },
            ],
            prev_data: None,
            start_date: None,
            end_date: None,
            prev_start_date: None,
            prev_end_date: None,
            average: None,
            higher_is_better: None,
        };
        assert_eq!(r.current_total(), 8.0);
        assert_eq!(r.previous_total(), None);
    }

    #[test]
    fn previous_total_when_prev_data_present() {
        let r = AdminReport {
            report_type: "posts".to_string(),
            data: vec![ReportPoint { x: "2026-04-01".into(), y: 10.0 }],
            prev_data: Some(vec![
                ReportPoint { x: "2026-03-01".into(), y: 4.0 },
                ReportPoint { x: "2026-03-02".into(), y: 6.0 },
            ]),
            start_date: None,
            end_date: None,
            prev_start_date: None,
            prev_end_date: None,
            average: None,
            higher_is_better: None,
        };
        assert_eq!(r.current_total(), 10.0);
        assert_eq!(r.previous_total(), Some(10.0));
    }
}
