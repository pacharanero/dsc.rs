use crate::cli::OutputFormat;
use crate::commands::common::{ensure_api_credentials, select_discourse};
use crate::config::Config;
use crate::discourse::DiscourseClient;
use anyhow::Result;
use std::io;

pub fn backup_create(config: &Config, discourse_name: &str) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;
    client.create_backup()?;
    Ok(())
}

pub fn backup_list(config: &Config, discourse_name: &str, format: OutputFormat) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;
    let response = client.list_backups()?;
    let mut backups = response
        .get("backups")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    backups.sort_by(|a, b| backup_created_at(b).cmp(&backup_created_at(a)));
    let global_location = backup_location_response(&response);
    let backup_size = |backup: &serde_json::Value| -> String {
        backup
            .get("size")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string())
            .or_else(|| {
                backup
                    .get("size_bytes")
                    .and_then(|v| v.as_u64())
                    .map(|v| v.to_string())
            })
            .unwrap_or_else(|| "unknown".to_string())
    };

    match format {
        OutputFormat::Plaintext => {
            if let Some(latest) = backups.first() {
                let filename = backup_filename(latest);
                let created_at = backup_created_at(latest).unwrap_or("unknown");
                let location = backup_location(latest, global_location.as_deref());
                println!(
                    "Latest backup: {} - {} - {}",
                    filename, created_at, location
                );
            }
            for backup in &backups {
                let filename = backup_filename(backup);
                let created_at = backup_created_at(backup).unwrap_or("unknown");
                let size = backup_size(backup);
                let location = backup_location(backup, global_location.as_deref());
                println!("{} - {} - {} - {}", filename, created_at, size, location);
            }
        }
        OutputFormat::Markdown => {
            if let Some(latest) = backups.first() {
                let filename = backup_filename(latest);
                let created_at = backup_created_at(latest).unwrap_or("unknown");
                let location = backup_location(latest, global_location.as_deref());
                println!(
                    "Latest backup: {} ({}) - {}",
                    filename, created_at, location
                );
            }
            for backup in &backups {
                let filename = backup_filename(backup);
                let created_at = backup_created_at(backup).unwrap_or("unknown");
                let size = backup_size(backup);
                let location = backup_location(backup, global_location.as_deref());
                println!("- {} ({}) - {} - {}", filename, created_at, size, location);
            }
        }
        OutputFormat::MarkdownTable => {
            println!("| Filename | Created At | Size | Location |");
            println!("| --- | --- | --- | --- |");
            for backup in &backups {
                let filename = backup_filename(backup);
                let created_at = backup_created_at(backup).unwrap_or("unknown");
                let size = backup_size(backup);
                let location = backup_location(backup, global_location.as_deref());
                println!(
                    "| {} | {} | {} | {} |",
                    filename, created_at, size, location
                );
            }
        }
        OutputFormat::Json => {
            let raw = serde_json::to_string_pretty(&response)?;
            println!("{}", raw);
        }
        OutputFormat::Yaml => {
            let raw = serde_yaml::to_string(&response)?;
            println!("{}", raw);
        }
        OutputFormat::Csv => {
            let mut writer = csv::Writer::from_writer(io::stdout());
            writer.write_record(["filename", "created_at", "size", "location"])?;
            for backup in &backups {
                let filename = backup_filename(backup);
                let created_at = backup_created_at(backup).unwrap_or("");
                let size = backup
                    .get("size")
                    .and_then(|v| v.as_str())
                    .map(|v| v.to_string())
                    .or_else(|| {
                        backup
                            .get("size_bytes")
                            .and_then(|v| v.as_u64())
                            .map(|v| v.to_string())
                    })
                    .unwrap_or_default();
                let location = backup_location(backup, global_location.as_deref());
                writer.write_record([filename, created_at, &size, &location])?;
            }
            writer.flush()?;
        }
    }
    Ok(())
}

pub fn backup_restore(config: &Config, discourse_name: &str, backup_path: &str) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;
    client.restore_backup(backup_path)?;
    Ok(())
}

fn backup_filename(backup: &serde_json::Value) -> &str {
    backup
        .get("filename")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
}

fn backup_created_at(backup: &serde_json::Value) -> Option<&str> {
    backup.get("created_at").and_then(|v| v.as_str())
}

fn backup_location_response(response: &serde_json::Value) -> Option<String> {
    let keys = [
        "backup_location",
        "location",
        "storage_location",
        "backup_store",
        "upload_destination",
    ];
    for key in keys {
        if let Some(value) = response.get(key).and_then(|v| v.as_str()) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn backup_location(backup: &serde_json::Value, global: Option<&str>) -> String {
    if let Some(global) = global {
        return global.to_string();
    }
    if let Some(location) = backup
        .get("location")
        .and_then(|v| v.as_str())
        .or_else(|| backup.get("backup_location").and_then(|v| v.as_str()))
        .or_else(|| backup.get("storage_location").and_then(|v| v.as_str()))
        .or_else(|| backup.get("upload_destination").and_then(|v| v.as_str()))
    {
        return location.to_string();
    }
    if let Some(url) = backup
        .get("url")
        .and_then(|v| v.as_str())
        .or_else(|| backup.get("path").and_then(|v| v.as_str()))
    {
        return location_from_url(url);
    }
    "unknown".to_string()
}

fn location_from_url(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.starts_with('/') {
        return "local".to_string();
    }
    if let Some(rest) = trimmed.split("//").nth(1) {
        return rest.split('/').next().unwrap_or(trimmed).to_string();
    }
    trimmed.to_string()
}
