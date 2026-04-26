# dsc analytics — specification

A new top-level command that pulls a small, deliberately chosen set of metrics from a Discourse to help an admin or community manager assess **community health**, **growth**, and **activity** at a glance.

This document is normative for the design and output of the command. User-facing documentation will live at `docs/analytics.md` once the command is implemented.

## Motivation

The Discourse admin dashboard exposes many charts, but several of the most prominent ones are weak or misleading signals on their own:

- **Page views** and **consolidated page views** are inflated by crawlers and logged-out skim traffic. They don't tell you whether the community is alive.
- **Signups** count anyone who created an account, including spam regs that never post a single word. They overstate growth.
- **Topic views** and **profile views** behave the same way and reward "viral" but low-engagement content.
- **DAU** in isolation moves with seasonality and notification emails; it's only useful as a ratio (DAU/MAU) and across consecutive periods.
- **"Time to first response"** is reported without context (median vs mean, including or excluding solved topics, etc.) and reads as a vanity number.

`dsc analytics` deliberately omits these as headline numbers. The metrics it does report are chosen so that each one, on its own, tells you something actionable about the community.

## Goals

1. One command, one screen of output, every number interpretable without a footnote.
2. Every metric has a clear "good direction" (up / down / stable) and a baseline you can compare against.
3. Every metric is reproducible from Discourse's public/admin APIs without any third-party analytics provider.
4. Machine-readable output for piping into dashboards, journals, and cron-driven trend tracking.

## Non-goals

- Not a replacement for full BI / data warehouse analytics.
- Not a real-time monitoring tool. Granularity is daily at best, weekly is typical.
- No traffic / referrer / SEO metrics — those belong elsewhere.
- No per-user drill-down. `dsc user info` and `dsc user activity` cover that.

## Command shape

```text
dsc analytics <discourse> [--since <when>] [--compare] [--section health|growth|activity|all]
                          [--format text|json|yaml|markdown|markdown-table|csv]
```

Aliases: `dsc stats` is accepted as an alias for discoverability but `analytics` is canonical.

### Flags

- `--since <when>` (or `-s`) — the window to report on. Accepts the same relative durations as `dsc user activity` (`24h`, `7d`, `30d`, `1w`, `1m`, `90s`) or an ISO-8601 date/timestamp. Defaults to `30d`.
- `--compare` (or `-c`) — also fetch the immediately preceding window of equal length and show a delta column. For example with `--since 30d --compare`, the prior 30 days are pulled and reported alongside.
- `--section` — restrict output to one section. Defaults to `all`.
- `--format` (or `-f`) — see "Format baseline" in `spec/spec.md`. Default is `text`. `markdown` and `markdown-table` are intended for piping into a `dsc topic reply` so that admins can keep a rolling community-health thread on their own forum (mirrors the `dsc user activity` archive workflow).

### Auth

- Requires admin scope. Most reports are admin-only on Discourse.
- If the configured key is not admin, the command should fail fast with the standard `missing api key for {discourse}; please set apikey or check your config` message rather than silently dropping sections.

### Honours `--dry-run`?

No. Read-only command.

## Sections and metrics

Output is grouped into three sections. Each metric prints on its own line: a short label, the value for the window, and (with `--compare`) the prior-window value and an arrow.

### Growth

The question this section answers: **is the community gaining or losing engaged people?**

| Metric                     | Definition                                                                                                                 | Why it's chosen over the obvious alternative                                                              |
| -------------------------- | -------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| New contributors           | Users whose **first ever post** was in the window.                                                                         | Replaces "signups". Counts only people who actually started participating.                                |
| Reactivated users          | Users who posted in the window after **≥ 90 days** of silence.                                                             | Surfaces win-back without conflating with new growth.                                                     |
| Lost regulars              | Users who posted ≥ 4 times in the prior 90 days but **zero times** in the window. Reported only when `--since` is ≥ 30d.   | The honest counterpart to "new contributors" — most dashboards never show churn at all.                   |
| Net active change          | New contributors + reactivated − lost regulars.                                                                            | A single net-flow number that captures whether the active base grew.                                      |
| Trust-level promotions     | Count of TL0→TL1, TL1→TL2, TL2→TL3 promotions in the window.                                                               | "People crossing the engagement threshold" beats "total users at TL1+".                                   |

### Activity

The question this section answers: **is the place busy and broad, or quiet and lopsided?**

| Metric                     | Definition                                                                                                                              | Why it's chosen over the obvious alternative                                                                              |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Topics created             | New topics in the window (excluding PMs and topics in archetype `private_message`).                                                      | Straightforward.                                                                                                          |
| Posts created              | New posts (replies + topic OPs).                                                                                                         | Straightforward.                                                                                                          |
| Posts per topic            | Posts created ÷ topics created.                                                                                                          | Conversation depth. A spike in topics with a flat ratio means activity isn't producing discussion.                        |
| Unique posters             | Distinct users who created at least one post in the window.                                                                              | Breadth. Pairs with the next metric to detect lopsidedness.                                                               |
| Top-10 share               | % of all posts in the window written by the 10 most active posters.                                                                      | A concentration warning. Healthy public communities sit roughly in the 30–55% band; sustained > 70% is a fragility signal. |
| Reply coverage             | % of topics created in the window that received **at least one reply from a different user** within 7 days.                              | The inverse of "no-reply rate". Replaces the ambiguous "time to first response" headline.                                 |
| Median time to first reply | Median minutes from topic creation to first reply, computed only over topics that received a reply.                                      | Same denominator caveat is now visible because it sits next to "reply coverage".                                          |

### Health

The question this section answers: **are the people here treating the place, and each other, well?**

| Metric                     | Definition                                                                                                                              | Why it's chosen over the obvious alternative                                                              |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Likes per post             | Likes given in the window ÷ posts created in the window.                                                                                 | Cheap proxy for peer recognition. Tracks well across periods even though the absolute number is noisy.    |
| Returning poster rate      | % of posters in the window who also posted in the **previous** window of equal length.                                                  | Real retention. More honest than DAU/MAU on its own.                                                      |
| Flags raised               | User-raised flags in the window (excluding system / auto flags).                                                                         | Conflict signal. Spikes warrant attention even when other metrics look fine.                              |
| Flag resolution time       | Median hours from flag creation to staff disposition.                                                                                    | Moderation responsiveness. Pairs with "flags raised" to distinguish "calm" from "neglected".              |
| Moderator action rate      | Moderator actions per 1,000 posts in the window (silences, suspends, post deletions, topic closes, recategorisations).                   | Normalises moderator load against site activity. A jump per 1,000 posts is informative; raw counts aren't.|
| Solo-thread rate           | % of topics in the window where only the OP posted (no replies from anyone else, ever).                                                  | Loneliness signal. Rising solo-thread rate is one of the earliest indicators of community decay.          |

## Output

### Text (default)

Single screen. Section headers, fixed-width metric labels, right-aligned numbers, optional delta column with arrows under `--compare`.

```text
analytics for myforum — last 30 days (2026-03-27 → 2026-04-26)

growth
  new contributors           42      ↑ 31     (+35%)
  reactivated users          11      ↑  8     (+38%)
  lost regulars               7      ↑  5     (+40%)
  net active change         +46      ↑ 34
  trust-level promotions     19      ↑ 12     (+58%)

activity
  topics created            128      ↑ 117    (+9%)
  posts created             964      ↑ 902    (+7%)
  posts per topic            7.5     ↓  7.7
  unique posters            188      ↑ 174    (+8%)
  top-10 share              41%      ↓ 47%
  reply coverage            83%      ↑ 79%
  median time to first reply 38m     ↓ 51m

health
  likes per post             1.9     ↓  2.1
  returning poster rate     61%      ↑ 58%
  flags raised               14      ↑  9
  flag resolution time       4.2h    ↑  6.1h
  moderator action rate      6.3 / 1k posts   ↓ 7.0
  solo-thread rate          12%      ↓ 15%
```

The arrow direction is colour-coded by **whether the change is desirable**, not just by sign:

- "lost regulars going up" is bad → red.
- "solo-thread rate going down" is good → green.
- "median time to first reply going down" is good → green.

A single column of arrows that always meant "up" would be misleading. Each metric carries an internal `desirable_direction` so the renderer can colour it correctly.

### JSON / YAML

Stable schema, suitable for trend tracking via cron:

```jsonc
{
  "discourse": "myforum",
  "window": { "since": "2026-03-27T00:00:00Z", "until": "2026-04-26T00:00:00Z", "label": "30d" },
  "compare": { "since": "2026-02-25T00:00:00Z", "until": "2026-03-27T00:00:00Z" },
  "growth": {
    "new_contributors":      { "current": 42, "previous": 31, "delta_pct":  35.5, "desirable": "up"   },
    "reactivated_users":     { "current": 11, "previous":  8, "delta_pct":  37.5, "desirable": "up"   },
    "lost_regulars":         { "current":  7, "previous":  5, "delta_pct":  40.0, "desirable": "down" },
    "net_active_change":     { "current": 46, "previous": 34, "desirable": "up"   },
    "trust_level_promotions":{ "current": 19, "previous": 12, "delta_pct":  58.3, "desirable": "up"   }
  },
  "activity": { /* … */ },
  "health":   { /* … */ }
}
```

- `delta_pct` is omitted when the previous value is zero (otherwise it's `Infinity` and unhelpful).
- All durations are emitted in seconds in JSON/YAML; the text/markdown renderers humanise them.
- Schema is versioned via a top-level `"schema": 1` field so downstream consumers can pin.

### Markdown / markdown-table

Intended for piping into a topic reply or a status page. `markdown` produces a heading per section followed by a bullet list; `markdown-table` produces one table per section. Example workflow:

```bash
# Weekly community-health roll-up posted to a private staff topic
dsc analytics myforum --since 7d --compare --format markdown \
  | dsc topic reply myforum 4242 --title "Community health — $(date -u +%Y-W%V)"
```

### CSV

One row per metric, columns: `section,metric,current,previous,delta,delta_pct,desirable_direction,unit`. Stable enough to append to a tracking spreadsheet.

## Data sources

All metrics are computed from the Discourse admin reports/API surface. The implementation MUST prefer the highest-level endpoint that already returns the metric, only falling back to derivation when no first-class report exists. Concretely:

- `/admin/reports/{id}.json?start_date=…&end_date=…` covers most counts: `signups`, `topics`, `posts`, `likes`, `flags`, `users_by_trust_level`, `trust_level_growth`, `moderators_activity`, `time_to_first_response`, `topics_with_no_response`, etc.
- `/admin/users/list/{filter}.json` is used for new-contributor first-post detection where the report doesn't carry it.
- `/admin/dashboard.json` is **not** trusted as a source — it bundles many of the metrics this command intentionally avoids and the bundle changes between Discourse releases.

Caching: a per-invocation cache in memory only. No on-disk cache in this iteration.

## Edge cases and explicit decisions

- **New install with < 30 days of history.** If `--since` exceeds the install age, the window is clamped to the install age and a one-line note is printed under the heading: `(window clamped — install is N days old)`. JSON includes `"clamped": true`.
- **Window of zero topics.** Sections still render; `posts per topic`, `reply coverage`, and `median time to first reply` print `—` (em dash) and JSON emits `null`. They MUST NOT print `NaN` or `0`.
- **Discourse running with `login_required = true` and the API key lacks admin scope.** Fail fast, do not silently degrade.
- **Anonymised / deleted users.** Their posts still count in totals; they're excluded from "unique posters" because Discourse anonymises the username.
- **PMs, whisper posts, and posts in private categories that the API key can't see.** Excluded everywhere. The command reports community-public health, not staff-channel health.
- **Time zones.** Windows are computed in UTC. The text header prints UTC dates explicitly so cron'd output is unambiguous.

## CLI consistency conformance

This command must comply with the standards in `spec/spec.md`:

- `--format text|json|yaml` baseline plus the listed extras.
- Empty / clamped sections follow the empty-list behaviour: text renders `—`; JSON/YAML emit `null` or `0` as appropriate (never strings like `"n/a"`).
- Errors use the standard messages (`discourse not found: …`, `missing apikey for …`).
- Short flags lowercase; `-s`, `-c`, `-f` chosen to mirror existing commands.

## Future work (out of scope for v1)

- Per-category breakdown (`--category support,announcements`).
- Per-group breakdown (`--group regulars`).
- A `dsc analytics watch` long-running mode that posts a weekly roll-up to a configured topic on a schedule.
- A `--baseline <path>` flag that loads a previously emitted JSON snapshot and computes deltas against it, decoupling "compare" from a fixed prior window.
- Surfacing of "topics that are spiking" (a leaderboard of topics whose post rate in the last 24h exceeded their 7-day baseline by N×). Useful but not headline-health.
