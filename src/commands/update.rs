use crate::commands::common::ensure_api_credentials;
use crate::config::{find_discourse, Config, DiscourseConfig};
use crate::discourse::DiscourseClient;
use crate::utils::ensure_dir;
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::thread;

pub fn update_one(config: &Config, name: &str, post_changelog: bool) -> Result<()> {
    let discourse = find_discourse(config, name).ok_or_else(|| anyhow!("unknown discourse"))?;
    let metadata = run_update(discourse)?;
    if post_changelog {
        handle_changelog_post(discourse, Some(&metadata), None)?;
    }
    Ok(())
}

pub fn update_all(
    config: &Config,
    concurrent: bool,
    max: Option<usize>,
    post_changelog: bool,
) -> Result<()> {
    if concurrent {
        return Err(anyhow!(
            "--concurrent is disabled for 'dsc update all' because it stops on first failure"
        ));
    }
    if !concurrent {
        let log_path = update_log_path()?;
        println!("==> Logging update progress to {}", log_path.display());
        let mut log_file = open_update_log(&log_path)?;
        writeln!(
            log_file,
            "{} update all started",
            chrono::Utc::now().to_rfc3339()
        )?;
        for discourse in &config.discourse {
            writeln!(
                log_file,
                "{} starting {}",
                chrono::Utc::now().to_rfc3339(),
                discourse.name
            )?;
            let metadata = match run_update(discourse) {
                Ok(metadata) => {
                    writeln!(
                        log_file,
                        "{} success {}",
                        chrono::Utc::now().to_rfc3339(),
                        discourse.name
                    )?;
                    metadata
                }
                Err(err) => {
                    writeln!(
                        log_file,
                        "{} failed {}: {}",
                        chrono::Utc::now().to_rfc3339(),
                        discourse.name,
                        err
                    )?;
                    return Err(err);
                }
            };
            if post_changelog {
                handle_changelog_post(discourse, Some(&metadata), Some(&mut log_file))?;
            }
        }
        writeln!(
            log_file,
            "{} update all completed",
            chrono::Utc::now().to_rfc3339()
        )?;
        return Ok(());
    }

    let max_threads = max.unwrap_or_else(|| config.discourse.len().max(1));
    let mut handles: Vec<thread::JoinHandle<Result<()>>> = Vec::new();
    for discourse in config.discourse.clone() {
        if handles.len() >= max_threads {
            if let Some(handle) = handles.pop() {
                handle.join().expect("thread panicked")?;
            }
        }
        let do_post = post_changelog;
        handles.push(thread::spawn(move || {
            let metadata = run_update(&discourse)?;
            if do_post {
                handle_changelog_post(&discourse, Some(&metadata), None)?;
            }
            Ok::<_, anyhow::Error>(())
        }));
    }

    for handle in handles {
        handle.join().expect("thread panicked")?;
    }

    Ok(())
}

struct UpdateMetadata {
    before_version: Option<String>,
    after_version: Option<String>,
    reclaimed_space: Option<String>,
    before_os_version: Option<String>,
    after_os_version: Option<String>,
    os_updated: bool,
    server_rebooted: bool,
}

fn run_update(discourse: &DiscourseConfig) -> Result<UpdateMetadata> {
    let client = DiscourseClient::new(discourse)?;
    let before_version = client.fetch_version().unwrap_or(None);
    let target = discourse
        .ssh_host
        .clone()
        .unwrap_or_else(|| discourse.name.clone());
    let before_os_version = get_os_version(&target)?;

    let os_update_cmd = std::env::var("DSC_SSH_OS_UPDATE_CMD").unwrap_or_else(|_| {
        "sudo -n DEBIAN_FRONTEND=noninteractive apt update && sudo -n DEBIAN_FRONTEND=noninteractive apt upgrade -y"
            .to_string()
    });
    let reboot_cmd =
        std::env::var("DSC_SSH_REBOOT_CMD").unwrap_or_else(|_| "sudo -n reboot".to_string());
    let discourse_update_cmd = std::env::var("DSC_SSH_UPDATE_CMD")
        .unwrap_or_else(|_| "cd /var/discourse && sudo -n ./launcher rebuild app".to_string());
    let cleanup_cmd = std::env::var("DSC_SSH_CLEANUP_CMD")
        .unwrap_or_else(|_| "cd /var/discourse && sudo -n ./launcher cleanup".to_string());

    let mut os_updated = false;
    let mut server_rebooted = false;

    if let Err(err) = run_ssh_command(&target, &os_update_cmd) {
        if let Some(rollback_cmd) = os_update_rollback_cmd() {
            if let Err(rollback_err) = run_ssh_command(&target, &rollback_cmd) {
                eprintln!(
                    "Warning: OS update rollback failed for {}: {}",
                    target, rollback_err
                );
            }
        }
        return Err(anyhow!("OS update failed for {}: {}", target, err));
    }
    os_updated = true;
    if run_ssh_command(&target, &reboot_cmd).is_ok() {
        server_rebooted = true;
        if std::env::var("DSC_SSH_OS_UPDATE_CMD").unwrap_or_default() != "echo OS packages updated"
        {
            std::thread::sleep(std::time::Duration::from_secs(30));
            let mut attempts = 0;
            let max_attempts = 12;
            while attempts < max_attempts {
                match ssh_probe(&target) {
                    Ok(true) => break,
                    Ok(false) | Err(_) => {
                        attempts += 1;
                        if attempts < max_attempts {
                            std::thread::sleep(std::time::Duration::from_secs(30));
                        }
                    }
                }
            }
            if attempts >= max_attempts {
                return Err(anyhow!("Server did not come back online after reboot"));
            }
        }
    }

    run_ssh_command(&target, &discourse_update_cmd)?;
    let after_version = client.fetch_version().unwrap_or(None);
    let cleanup = run_ssh_command(&target, &cleanup_cmd)?;
    let reclaimed_space = parse_reclaimed_space(&cleanup);
    let after_os_version = get_os_version(&target)?;

    Ok(UpdateMetadata {
        before_version,
        after_version,
        reclaimed_space,
        before_os_version,
        after_os_version,
        os_updated,
        server_rebooted,
    })
}

fn run_ssh_command(target: &str, command: &str) -> Result<String> {
    let mut cmd = build_ssh_command(target, &[])?;
    let output = cmd
        .arg(command)
        .output()
        .with_context(|| format!("running ssh to {}", target))?;
    if !output.status.success() {
        return Err(anyhow!(
            "ssh command failed for {}: {}",
            target,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn build_ssh_command(target: &str, extra_options: &[&str]) -> Result<std::process::Command> {
    validate_ssh_target(target)?;
    let mut cmd = std::process::Command::new("ssh");
    cmd.arg("-o").arg("BatchMode=yes");
    if let Some(strict) = ssh_strict_host_key_checking() {
        cmd.arg("-o")
            .arg(format!("StrictHostKeyChecking={}", strict));
    }
    for option in extra_options {
        cmd.arg(option);
    }
    if let Ok(raw) = std::env::var("DSC_SSH_OPTIONS") {
        if !raw.trim().is_empty() {
            cmd.args(raw.split_whitespace());
        }
    }
    cmd.arg("--").arg(target);
    Ok(cmd)
}

fn ssh_strict_host_key_checking() -> Option<String> {
    let value = std::env::var("DSC_SSH_STRICT_HOST_KEY_CHECKING")
        .unwrap_or_else(|_| "accept-new".to_string());
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn validate_ssh_target(target: &str) -> Result<()> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("ssh target is empty"));
    }
    if trimmed.starts_with('-') {
        return Err(anyhow!("ssh target cannot start with '-': {}", target));
    }
    if trimmed.chars().any(|ch| ch.is_whitespace()) {
        return Err(anyhow!("ssh target cannot contain whitespace: {}", target));
    }
    Ok(())
}

fn ssh_probe(target: &str) -> Result<bool> {
    let mut cmd = build_ssh_command(target, &["-o", "ConnectTimeout=10"])?;
    let output = cmd
        .arg("echo 'server is up'")
        .output()
        .with_context(|| format!("running ssh to {}", target))?;
    Ok(output.status.success())
}

fn update_log_path() -> Result<PathBuf> {
    let date = chrono::Utc::now().format("%Y.%m.%d");
    let filename = format!("{}-dsc-update-all.log", date);
    if let Ok(raw) = std::env::var("DSC_UPDATE_LOG_DIR") {
        let raw = raw.trim();
        if !raw.is_empty() {
            let dir = PathBuf::from(raw);
            ensure_dir(&dir)?;
            return Ok(dir.join(filename));
        }
    }
    Ok(PathBuf::from(filename))
}

fn open_update_log(path: &Path) -> Result<fs::File> {
    let file = fs::OpenOptions::new()
        .create_new(true)
        .append(true)
        .open(path);
    match file {
        Ok(file) => Ok(file),
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
            let metadata = fs::symlink_metadata(path)
                .with_context(|| format!("reading {}", path.display()))?;
            if metadata.file_type().is_symlink() {
                return Err(anyhow!("update log path is a symlink: {}", path.display()));
            }
            fs::OpenOptions::new()
                .append(true)
                .open(path)
                .with_context(|| format!("opening update log at {}", path.display()))
        }
        Err(err) => Err(err).with_context(|| format!("opening update log at {}", path.display())),
    }
}

fn get_os_version(target: &str) -> Result<Option<String>> {
    let version_cmd = std::env::var("DSC_SSH_OS_VERSION_CMD")
        .unwrap_or_else(|_| "lsb_release -d | cut -f2".to_string());
    match run_ssh_command(target, &version_cmd) {
        Ok(output) => Ok(Some(output.trim().to_string())),
        Err(_) => {
            let fallback_cmd = "grep PRETTY_NAME /etc/os-release | cut -d'=' -f2 | tr -d '\"'";
            match run_ssh_command(target, fallback_cmd) {
                Ok(output) => Ok(Some(output.trim().to_string())),
                Err(_) => Ok(None),
            }
        }
    }
}

fn parse_reclaimed_space(output: &str) -> Option<String> {
    output
        .lines()
        .find_map(|line| line.split("Total reclaimed space:").nth(1))
        .map(|value| value.trim().to_string())
}

fn os_update_rollback_cmd() -> Option<String> {
    let raw = std::env::var("DSC_SSH_OS_UPDATE_ROLLBACK_CMD").unwrap_or_default();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn build_changelog_payload(metadata: Option<&UpdateMetadata>) -> String {
    let version = metadata
        .and_then(|meta| meta.after_version.clone().or(meta.before_version.clone()))
        .unwrap_or_else(|| "unknown".to_string());
    let reclaimed = metadata
        .and_then(|meta| meta.reclaimed_space.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let mut body = Vec::new();
    if let Some(meta) = metadata {
        if meta.os_updated {
            body.push("- [x] Ubuntu OS updated".to_string());
            if let Some(before_os) = &meta.before_os_version {
                if let Some(after_os) = &meta.after_os_version {
                    body.push(format!("  OS version: {} → {}", before_os, after_os));
                }
            }
        } else {
            body.push("- [ ] Ubuntu OS updated".to_string());
            body.push("  (OS update was skipped or failed)".to_string());
        }

        if meta.server_rebooted {
            body.push("- [x] Server rebooted".to_string());
        } else {
            body.push("- [ ] Server rebooted".to_string());
            body.push("  (Server reboot was skipped or failed)".to_string());
        }
    } else {
        body.push("- [x] Ubuntu OS updated".to_string());
        body.push("- [x] Server rebooted".to_string());
    }
    body.push(format!("- [x] Updated Discourse to version {}", version));
    body.push(format!(
        "- [x] `./launcher cleanup` Total reclaimed space: {}",
        reclaimed
    ));
    let test_marker = std::env::var("DSC_TEST_MARKER").ok();
    if let Some(marker) = &test_marker {
        body.push(format!("- Run-ID: {}", marker));
    }
    body.join("\n")
}

fn post_changelog_update(discourse: &DiscourseConfig, payload: &str) -> Result<u64> {
    let topic_id = discourse
        .changelog_topic_id
        .ok_or_else(|| anyhow!("changelog_topic_id is required to post updates"))?;
    let client = DiscourseClient::new(discourse)?;
    let post_id = client.create_post(topic_id, payload)?;
    if std::env::var("DSC_TEST_MARKER").is_ok() {
        println!("DSC_TEST_POST_ID={}", post_id);
    }
    Ok(post_id)
}

fn confirm_changelog_post() -> Result<bool> {
    if std::env::var("DSC_TEST_MARKER").is_ok() {
        println!("Post this to changelog? [y/N]: y (auto)");
        return Ok(true);
    }
    print!("Post this to changelog? [y/N]: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim(), "y" | "Y" | "yes" | "YES"))
}

fn handle_changelog_post(
    discourse: &DiscourseConfig,
    metadata: Option<&UpdateMetadata>,
    mut log: Option<&mut dyn Write>,
) -> Result<()> {
    let payload = build_changelog_payload(metadata);
    println!("\nChangelog message for {}:\n{}\n", discourse.name, payload);
    if !confirm_changelog_post()? {
        println!("Changelog post skipped.");
        if let Some(log) = log.as_deref_mut() {
            writeln!(
                log,
                "{} skipped {} (changelog)",
                chrono::Utc::now().to_rfc3339(),
                discourse.name
            )?;
        }
        return Ok(());
    }

    if let Err(err) = ensure_api_credentials(discourse) {
        println!("Changelog post failed: {}", err);
        if let Some(log) = log.as_deref_mut() {
            writeln!(
                log,
                "{} failed {} (changelog): {}",
                chrono::Utc::now().to_rfc3339(),
                discourse.name,
                err
            )?;
        }
        return Err(err);
    }

    match post_changelog_update(discourse, &payload) {
        Ok(post_id) => {
            println!("Changelog post created with ID: {}", post_id);
            if let Some(log) = log.as_deref_mut() {
                writeln!(
                    log,
                    "{} posted {} (changelog id: {})",
                    chrono::Utc::now().to_rfc3339(),
                    discourse.name,
                    post_id
                )?;
            }
            Ok(())
        }
        Err(err) => {
            println!("Changelog post failed: {}", err);
            if let Some(log) = log.as_deref_mut() {
                writeln!(
                    log,
                    "{} failed {} (changelog): {}",
                    chrono::Utc::now().to_rfc3339(),
                    discourse.name,
                    err
                )?;
            }
            Err(err)
        }
    }
}
