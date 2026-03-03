use anyhow::anyhow;
use reqwest::StatusCode;

pub fn http_error(action: &str, status: StatusCode, text: &str) -> anyhow::Error {
    if status == StatusCode::NOT_FOUND {
        return anyhow!("{action} failed with {} (not found)", status);
    }
    if status == StatusCode::FORBIDDEN {
        return anyhow!("{action} failed with {} (forbidden)", status);
    }
    if status == StatusCode::UNAUTHORIZED {
        return anyhow!("{action} failed with {} (unauthorized)", status);
    }
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return anyhow!("{action} failed with {} (empty response)", status);
    }
    anyhow!("{action} failed with {}: {}", status, trimmed)
}
