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
    let backups = response
        .get("backups")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
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
            for backup in &backups {
                let filename = backup
                    .get("filename")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let created_at = backup
                    .get("created_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let size = backup_size(backup);
                println!("{} - {} - {}", filename, created_at, size);
            }
        }
        OutputFormat::Markdown => {
            for backup in &backups {
                let filename = backup
                    .get("filename")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let created_at = backup
                    .get("created_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let size = backup_size(backup);
                println!("- {} ({}) - {}", filename, created_at, size);
            }
        }
        OutputFormat::MarkdownTable => {
            println!("| Filename | Created At | Size |");
            println!("| --- | --- | --- |");
            for backup in &backups {
                let filename = backup
                    .get("filename")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let created_at = backup
                    .get("created_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let size = backup_size(backup);
                println!("| {} | {} | {} |", filename, created_at, size);
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
            writer.write_record(["filename", "created_at", "size"])?;
            for backup in &backups {
                let filename = backup
                    .get("filename")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let created_at = backup
                    .get("created_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
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
                writer.write_record([filename, created_at, &size])?;
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
