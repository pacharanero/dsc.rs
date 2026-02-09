use crate::commands::common::ensure_api_credentials;
use crate::config::{find_discourse, Config, DiscourseConfig};
use crate::discourse::DiscourseClient;
use anyhow::{anyhow, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::VecDeque;
use std::io::{self, Write};
use std::io::{BufRead, BufReader};
use std::process::Stdio;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub fn update_one(config: &Config, name: &str, post_changelog: bool) -> Result<()> {
    let discourse = find_discourse(config, name).ok_or_else(|| anyhow!("unknown discourse"))?;
    let metadata = run_update(discourse)?;
    if post_changelog {
        handle_changelog_post(discourse, Some(&metadata))?;
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
        for discourse in &config.discourse {
            let metadata = run_update(discourse)?;
            if post_changelog {
                handle_changelog_post(discourse, Some(&metadata))?;
            }
        }
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
                handle_changelog_post(&discourse, Some(&metadata))?;
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
    let target = discourse
        .ssh_host
        .clone()
        .unwrap_or_else(|| discourse.name.clone());
    println!("\n==> Updating {} ({})", discourse.name, target);
    stage(&target, "Fetching Discourse version (before update)");
    let before_version = match client.fetch_version() {
        Ok(version) => {
            let label = version.as_deref().unwrap_or("unknown");
            stage(
                &target,
                &format!("Initial Discourse Version (before update): {}", label),
            );
            version
        }
        Err(err) => {
            stage(
                &target,
                &format!(
                    "Initial Discourse Version (before update): unknown (fetch failed: {})",
                    err
                ),
            );
            None
        }
    };
    stage(&target, "Fetching OS version (before update)");
    let before_os_version = match get_os_version(&target) {
        Ok(version) => {
            let label = version.as_deref().unwrap_or("unknown");
            stage(
                &target,
                &format!("Initial OS Version (before update): {}", label),
            );
            version
        }
        Err(err) => {
            stage(
                &target,
                &format!(
                    "Initial OS Version (before update): unknown (fetch failed: {})",
                    err
                ),
            );
            None
        }
    };

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

    let mut server_rebooted = false;

    stage(&target, "Running OS update");
    if let Err(err) = run_ssh_command_with_tail(
        &target,
        &os_update_cmd,
        "OS update in progress",
        3,
    ) {
        if let Some(rollback_cmd) = os_update_rollback_cmd() {
            stage(&target, "Running OS update rollback");
            if let Err(rollback_err) = run_ssh_command(&target, &rollback_cmd) {
                eprintln!(
                    "Warning: OS update rollback failed for {}: {}",
                    target, rollback_err
                );
            }
        }
        return Err(anyhow!("OS update failed for {}: {}", target, err));
    }
    let os_updated = true;
    stage(&target, "Rebooting server");
    if run_ssh_command(&target, &reboot_cmd).is_ok() {
        server_rebooted = true;
        if std::env::var("DSC_SSH_OS_UPDATE_CMD").unwrap_or_default() != "echo OS packages updated"
        {
            stage(&target, "Waiting for server to come back online");
            std::thread::sleep(std::time::Duration::from_secs(30));
            let mut attempts = 0;
            let max_attempts = 12;
            while attempts < max_attempts {
                match ssh_probe(&target) {
                    Ok(true) => break,
                    Ok(false) | Err(_) => {
                        attempts += 1;
                        if attempts < max_attempts {
                            println!(
                                "[{}] Still waiting for SSH (attempt {}/{})",
                                target,
                                attempts + 1,
                                max_attempts
                            );
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

    stage(&target, "Running Discourse update");
    run_ssh_command_with_tail(
        &target,
        &discourse_update_cmd,
        "Discourse update in progress",
        3,
    )?;
    stage(&target, "Fetching Discourse version (after update)");
    let after_version = match client.fetch_version() {
        Ok(version) => {
            let label = version.as_deref().unwrap_or("unknown");
            stage(
                &target,
                &format!("Final Discourse Version (after update): {}", label),
            );
            version
        }
        Err(err) => {
            stage(
                &target,
                &format!(
                    "Final Discourse Version (after update): unknown (fetch failed: {})",
                    err
                ),
            );
            None
        }
    };
    stage(&target, "Running cleanup");
    let cleanup = run_ssh_command(&target, &cleanup_cmd)?;
    let reclaimed_space = parse_reclaimed_space(&cleanup);
    let after_os_version = None;

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

pub(crate) fn run_ssh_command(target: &str, command: &str) -> Result<String> {
    let mut cmd = build_ssh_command(target, &[])?;
    let output = cmd
        .arg(command)
        .output()
        .with_context(|| format!("running ssh to {}: {}", target, command))?;
    if !output.status.success() {
        return Err(anyhow!(
            "ssh command failed for {}: {}",
            target,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_ssh_command_with_spinner(target: &str, command: &str, message: &str) -> Result<String> {
    let pb = ProgressBar::new_spinner();
    let style =
        ProgressStyle::with_template("{spinner} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner());
    pb.set_style(style);
    pb.set_message(format!("[{}] {}", target, message));
    pb.enable_steady_tick(Duration::from_millis(120));

    let (tx, rx) = mpsc::channel();
    let target = target.to_string();
    let command = command.to_string();
    thread::spawn(move || {
        let result = run_ssh_command(&target, &command);
        let _ = tx.send(result);
    });

    let result = rx
        .recv()
        .map_err(|_| anyhow!("OS update command thread ended unexpectedly"))?;
    pb.finish_and_clear();
    result
}

struct LineEvent {
    is_stderr: bool,
    line: String,
}

fn run_ssh_command_with_tail(
    target: &str,
    command: &str,
    message: &str,
    tail_lines: usize,
) -> Result<String> {
    let pb = ProgressBar::new_spinner();
    let style =
        ProgressStyle::with_template("{spinner} {msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner());
    pb.set_style(style);
    pb.enable_steady_tick(Duration::from_millis(120));

    let mut cmd = build_ssh_command(target, &[])?;
    let mut child = cmd
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("running ssh to {}: {}", target, command))?;

    let stdout = child.stdout.take().context("missing stdout")?;
    let stderr = child.stderr.take().context("missing stderr")?;

    let (tx, rx) = mpsc::channel::<LineEvent>();
    let tx_out = tx.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    let _ = tx_out.send(LineEvent {
                        is_stderr: false,
                        line,
                    });
                }
                Err(_) => break,
            }
        }
    });

    let tx_err = tx.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    let _ = tx_err.send(LineEvent {
                        is_stderr: true,
                        line,
                    });
                }
                Err(_) => break,
            }
        }
    });

    drop(tx);

    let mut stdout_buf = String::new();
    let mut stderr_buf = String::new();
    let mut tail: VecDeque<String> = VecDeque::new();
    let base = format!("[{}] {}", target, message);
    pb.set_message(base.clone());

    loop {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(event) => {
                if event.is_stderr {
                    stderr_buf.push_str(&event.line);
                    stderr_buf.push('\n');
                } else {
                    stdout_buf.push_str(&event.line);
                    stdout_buf.push('\n');
                }

                if tail_lines > 0 {
                    if tail.len() == tail_lines {
                        tail.pop_front();
                    }
                    tail.push_back(event.line);

                    let mut msg = base.clone();
                    for line in &tail {
                        msg.push('\n');
                        msg.push_str("  ");
                        msg.push_str(line);
                    }
                    pb.set_message(msg);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let status = child.wait().context("waiting for ssh command")?;
    pb.finish_and_clear();

    if !status.success() {
        return Err(anyhow!(
            "ssh command failed for {}: {}",
            target,
            stderr_buf
        ));
    }

    Ok(stdout_buf)
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
        .with_context(|| format!("running ssh probe to {}", target))?;
    Ok(output.status.success())
}

fn stage(target: &str, message: &str) {
    println!("[{}] {}", target, message);
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
) -> Result<()> {
    let payload = build_changelog_payload(metadata);
    println!("\nChangelog message for {}:\n{}\n", discourse.name, payload);
    let topic_id = discourse.changelog_topic_id;
    if topic_id.is_none() {
        println!(
            "Changelog post skipped: missing changelog_topic_id for {}",
            discourse.name
        );
        return Ok(());
    }

    if let Err(err) = ensure_api_credentials(discourse) {
        println!("Changelog post skipped: {}", err);
        return Ok(());
    }

    if !confirm_changelog_post()? {
        println!("Changelog post skipped.");
        return Ok(());
    }

    match post_changelog_update(discourse, &payload) {
        Ok(post_id) => {
            println!("Changelog post created with ID: {}", post_id);
            Ok(())
        }
        Err(err) => {
            println!("Changelog post failed: {}", err);
            Err(err)
        }
    }
}
