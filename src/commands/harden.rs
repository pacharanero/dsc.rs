//! `dsc harden` — turn a fresh Ubuntu server reachable via `ssh root@host`
//! into a hardened box with a non-root sudo user, SSH locked down, rootless
//! Docker, fail2ban, unattended upgrades, swap, and a firewall.
//!
//! **Stage 1 (this file, v0.10.0-alpha):** everything up to and including
//! verifying that new-user SSH actually works. Stops *before* touching
//! sshd_config so a bad pubkey can't self-lockout the operator. Stages 2
//! (sshd tightening) and 3 (fail2ban / upgrades / timezone / swap / docker
//! / ufw) land in follow-up commits.

use crate::config::HardenConfig;
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::Path;
use std::process::Command;

/// SSH target: who + where + on what port. `port == 22` is the default and
/// omitted from ssh args.
#[derive(Clone, Debug)]
pub(crate) struct SshTarget {
    pub user: String,
    pub host: String,
    pub port: u16,
}

impl SshTarget {
    fn as_arg(&self) -> String {
        format!("{}@{}", self.user, self.host)
    }
}

/// Per-step options resolved from CLI flags → `[harden]` config block →
/// built-in defaults (in that precedence order). Stages 2 and 3 will read
/// more fields from this; stage 1 only needs `new_user` / `ssh_port`.
#[allow(dead_code)] // remaining fields are read by stages 2 and 3.
#[derive(Clone, Debug)]
pub(crate) struct Options {
    pub new_user: String,
    pub ssh_port: u16,
    pub docker_install_url: String,
    pub docker_rootless: bool,
    pub swap_size_gb: u32,
    pub journald_max_use: String,
    pub timezone: String,
    pub unattended_security_upgrades: bool,
    pub fail2ban: bool,
    pub mosh: bool,
    pub sshd_ciphers: String,
    pub sshd_kex: String,
    pub sshd_macs: String,
    pub extra_ufw_allow: Vec<String>,
}

/// Modern SSH algorithm pins, drops CBC ciphers and weak KEX/MACs.
const DEFAULT_CIPHERS: &str =
    "chacha20-poly1305@openssh.com,aes256-gcm@openssh.com,aes128-gcm@openssh.com,aes256-ctr,aes192-ctr,aes128-ctr";
const DEFAULT_KEX: &str =
    "curve25519-sha256,curve25519-sha256@libssh.org,diffie-hellman-group16-sha512,diffie-hellman-group18-sha512";
const DEFAULT_MACS: &str =
    "hmac-sha2-512-etm@openssh.com,hmac-sha2-256-etm@openssh.com,umac-128-etm@openssh.com";

/// Resolve final options. CLI overrides win, then `[harden]` config block,
/// then the built-in defaults documented in `dsc.example.toml`.
pub(crate) fn resolve_options(
    cli_new_user: Option<&str>,
    cli_ssh_port: Option<u16>,
    cfg: &HardenConfig,
) -> Options {
    Options {
        new_user: cli_new_user
            .map(str::to_string)
            .or_else(|| cfg.new_user.clone())
            .unwrap_or_else(|| "discourse".to_string()),
        ssh_port: cli_ssh_port
            .or_else(|| cfg.ssh_port.map(|p| p as u16))
            .unwrap_or(2227),
        docker_install_url: cfg
            .docker_install_url
            .clone()
            .unwrap_or_else(|| "https://get.docker.com".to_string()),
        docker_rootless: cfg.docker_rootless.unwrap_or(true),
        swap_size_gb: cfg.swap_size_gb.unwrap_or(2),
        journald_max_use: cfg
            .journald_max_use
            .clone()
            .unwrap_or_else(|| "500M".to_string()),
        timezone: cfg.timezone.clone().unwrap_or_else(|| "UTC".to_string()),
        unattended_security_upgrades: cfg.unattended_security_upgrades.unwrap_or(true),
        fail2ban: cfg.fail2ban.unwrap_or(true),
        mosh: cfg.mosh.unwrap_or(false),
        sshd_ciphers: cfg.sshd_ciphers.clone().unwrap_or_else(|| DEFAULT_CIPHERS.to_string()),
        sshd_kex: cfg.sshd_kex.clone().unwrap_or_else(|| DEFAULT_KEX.to_string()),
        sshd_macs: cfg.sshd_macs.clone().unwrap_or_else(|| DEFAULT_MACS.to_string()),
        extra_ufw_allow: cfg.extra_ufw_allow.clone().unwrap_or_default(),
    }
}

pub fn harden(
    cfg: &HardenConfig,
    host: &str,
    ssh_user: &str,
    new_user: Option<&str>,
    ssh_port: Option<u16>,
    pubkey_file: &Path,
    dry_run: bool,
) -> Result<()> {
    let opts = resolve_options(new_user, ssh_port, cfg);
    let new_user = opts.new_user.as_str();
    let _ssh_port = opts.ssh_port;
    // Preflight: pubkey readable now, before we start SSH-ing around.
    let pubkey = fs::read_to_string(pubkey_file)
        .with_context(|| format!("reading {}", pubkey_file.display()))?
        .trim()
        .to_string();
    if pubkey.is_empty() {
        return Err(anyhow!(
            "pubkey file {} is empty",
            pubkey_file.display()
        ));
    }
    if !looks_like_ssh_pubkey(&pubkey) {
        return Err(anyhow!(
            "pubkey file {} does not look like an SSH public key (expected to start with ssh-ed25519, ssh-rsa, ecdsa-sha2-*, etc.)",
            pubkey_file.display()
        ));
    }

    // Initial SSH target is the operator's starting position — usually
    // `ssh root@host` on port 22.
    let initial = SshTarget {
        user: ssh_user.to_string(),
        host: host.to_string(),
        port: 22,
    };

    announce(&format!(
        "Hardening {} as {} → creating non-root user `{}`",
        host, ssh_user, new_user
    ));

    // --- Preflight probes ---
    let os_release = ssh_run(&initial, "cat /etc/os-release", dry_run)?;
    assert_ubuntu(&os_release, dry_run)?;

    let mem_kb_raw = ssh_run(&initial, "awk '/^MemTotal:/ {print $2}' /proc/meminfo", dry_run)?;
    assert_enough_memory(&mem_kb_raw, dry_run)?;

    // Check free disk on /var — Docker (and later, Discourse rebuilds)
    // needs to fit a fresh container image alongside the running one, so
    // the floor is higher than you'd expect (~5 GB). 30 GB is the
    // practical minimum for a box you expect to keep for any length of
    // time; below that Discourse upgrades will hit disk issues soon.
    let disk_gb_raw = ssh_run(
        &initial,
        "df -B1G --output=avail /var | tail -n 1 | tr -d ' '",
        dry_run,
    )?;
    assert_enough_disk(&disk_gb_raw, dry_run)?;

    let whoami = ssh_run(&initial, "whoami", dry_run)?;
    assert_is_root(&whoami, dry_run)?;

    // --- Step 1: create user ---
    //
    // `adduser --disabled-password --gecos ""` is the non-interactive form
    // of the playbook's `adduser <user>`. Skipped if the user already
    // exists — makes the whole command safely re-runnable.
    let user_exists = ssh_run(
        &initial,
        &format!("id -u {} >/dev/null 2>&1 && echo yes || echo no", shell_quote(new_user)),
        dry_run,
    )?;
    if user_exists.trim() == "yes" {
        announce(&format!("user `{}` already exists, skipping creation", new_user));
    } else {
        announce(&format!("creating user `{}`", new_user));
        ssh_run(
            &initial,
            &format!("adduser --disabled-password --gecos '' {}", shell_quote(new_user)),
            dry_run,
        )?;
    }

    // --- Step 2: sudo NOPASSWD ---
    //
    // The playbook does this via `visudo`; we drop a single-file snippet in
    // /etc/sudoers.d/ instead, which is both safer (not editing the main
    // sudoers) and idempotent (a second write is a no-op). visudo -cf
    // validates syntax before the file is moved into place.
    announce(&format!("granting `{}` sudo NOPASSWD (via /etc/sudoers.d/)", new_user));
    let sudoers_line = format!("{} ALL=(ALL) NOPASSWD: ALL", new_user);
    ssh_run(
        &initial,
        &format!(
            "tmp=$(mktemp) && printf '%s\\n' {} > \"$tmp\" && visudo -cf \"$tmp\" && install -m 0440 \"$tmp\" /etc/sudoers.d/90-{}-nopasswd && rm -f \"$tmp\"",
            shell_quote(&sudoers_line),
            shell_quote(new_user),
        ),
        dry_run,
    )?;

    // --- Step 3: pubkey install ---
    //
    // Create ~/.ssh with strict perms, write authorized_keys, chown to the
    // new user. dedupe on pubkey identity so re-runs don't accumulate.
    announce(&format!("installing pubkey for `{}`", new_user));
    let ak_setup = format!(
        r#"
install -d -m 0700 -o {user} -g {user} /home/{user}/.ssh
touch /home/{user}/.ssh/authorized_keys
chmod 0600 /home/{user}/.ssh/authorized_keys
chown {user}:{user} /home/{user}/.ssh/authorized_keys
grep -qxF {key} /home/{user}/.ssh/authorized_keys || printf '%s\n' {key} >> /home/{user}/.ssh/authorized_keys
"#,
        user = shell_quote(new_user),
        key = shell_quote(&pubkey),
    );
    ssh_run(&initial, ak_setup.trim(), dry_run)?;

    // --- Step 4: verify new-user SSH actually works ---
    //
    // THIS IS THE SELF-LOCKOUT GUARD. Before any further stage touches
    // sshd_config, we prove the new pubkey flow works. If this fails, the
    // operator still has root SSH to debug with.
    let new_target = SshTarget {
        user: new_user.to_string(),
        host: host.to_string(),
        port: 22,
    };
    if dry_run {
        announce(&format!(
            "[dry-run] would verify SSH login as `{}@{}` now — if this failed, stages 2+ would refuse to proceed",
            new_user, host
        ));
    } else {
        announce(&format!(
            "verifying SSH login as `{}@{}` works…",
            new_user, host
        ));
        let who = ssh_run(&new_target, "whoami", false)
            .context(
                "failed to SSH as the new user — NOT proceeding. \
                 The original root SSH is still usable; fix the pubkey or user setup and re-run.",
            )?;
        if who.trim() != new_user {
            return Err(anyhow!(
                "SSH as {} succeeded but `whoami` returned {:?} — something is very wrong, stopping",
                new_user,
                who.trim()
            ));
        }
        announce(&format!("✓ new-user SSH verified ({}@{})", new_user, host));
    }

    announce("Stage 1 complete. sshd is still in its original state (root SSH + password auth still permitted).");
    announce("Stages 2+ (sshd tightening, fail2ban, upgrades, swap, docker, ufw) land in a follow-up commit.");
    Ok(())
}

// --- helpers ---

fn announce(msg: &str) {
    eprintln!("[harden] {}", msg);
}

fn looks_like_ssh_pubkey(s: &str) -> bool {
    let first = s.split_whitespace().next().unwrap_or("");
    matches!(
        first,
        "ssh-ed25519"
            | "ssh-rsa"
            | "ssh-dss"
            | "ecdsa-sha2-nistp256"
            | "ecdsa-sha2-nistp384"
            | "ecdsa-sha2-nistp521"
            | "sk-ecdsa-sha2-nistp256@openssh.com"
            | "sk-ssh-ed25519@openssh.com"
    )
}

fn assert_ubuntu(os_release: &str, dry_run: bool) -> Result<()> {
    if dry_run && os_release.is_empty() {
        return Ok(());
    }
    let is_ubuntu = os_release
        .lines()
        .any(|l| l.trim() == "ID=ubuntu" || l.trim() == "ID=\"ubuntu\"");
    if !is_ubuntu {
        return Err(anyhow!(
            "remote host is not Ubuntu — dsc harden currently only supports Ubuntu 22.04+. \
             Got /etc/os-release:\n{}",
            os_release
        ));
    }
    Ok(())
}

fn assert_enough_memory(mem_kb_raw: &str, dry_run: bool) -> Result<()> {
    if dry_run && mem_kb_raw.is_empty() {
        return Ok(());
    }
    let kb: u64 = mem_kb_raw
        .trim()
        .parse()
        .with_context(|| format!("parsing MemTotal from {:?}", mem_kb_raw))?;
    let mb = kb / 1024;
    if mb < 1000 {
        return Err(anyhow!(
            "remote host has only {} MB RAM — Discourse's hard minimum is 1024 MB. Bail out.",
            mb
        ));
    }
    if mb < 2048 {
        eprintln!(
            "[harden] warning: only {} MB RAM detected. Discourse runs at 1 GB but rebuilds are miserable; 2 GB is the practical floor.",
            mb
        );
    } else {
        announce(&format!("memory OK ({} MB)", mb));
    }
    Ok(())
}

fn assert_enough_disk(gb_raw: &str, dry_run: bool) -> Result<()> {
    if dry_run && gb_raw.is_empty() {
        return Ok(());
    }
    let gb: u64 = gb_raw
        .trim()
        .parse()
        .with_context(|| format!("parsing free-GB from {:?}", gb_raw))?;
    if gb < 5 {
        return Err(anyhow!(
            "only {} GB free on /var — `./launcher rebuild` needs ~5 GB just to land a new image alongside the running one. Bail out and get a bigger disk.",
            gb
        ));
    }
    if gb < 30 {
        eprintln!(
            "[harden] warning: only {} GB free on /var. Discourse runs at 5+ GB but upgrades hit disk issues quickly below ~30 GB. Consider resizing before you regret it.",
            gb
        );
    } else {
        announce(&format!("disk OK ({} GB free on /var)", gb));
    }
    Ok(())
}

fn assert_is_root(whoami: &str, dry_run: bool) -> Result<()> {
    if dry_run && whoami.is_empty() {
        return Ok(());
    }
    if whoami.trim() != "root" {
        return Err(anyhow!(
            "expected to be root on the remote (for stage 1 user creation) but whoami returned {:?}",
            whoami.trim()
        ));
    }
    Ok(())
}

/// Run a command over SSH. In --dry-run mode, prints what would run and
/// returns an empty string — callers must tolerate that (preflight
/// assertions above do so).
fn ssh_run(target: &SshTarget, command: &str, dry_run: bool) -> Result<String> {
    if dry_run {
        eprintln!("[dry-run] ssh {} -- {}", target.as_arg(), oneline(command));
        return Ok(String::new());
    }
    let mut cmd = Command::new("ssh");
    cmd.arg("-o").arg("BatchMode=yes");
    cmd.arg("-o").arg("StrictHostKeyChecking=accept-new");
    cmd.arg("-o").arg("ConnectTimeout=10");
    if target.port != 22 {
        cmd.arg("-p").arg(target.port.to_string());
    }
    cmd.arg("--").arg(target.as_arg()).arg(command);

    let output = cmd
        .output()
        .with_context(|| format!("spawning ssh to {}", target.as_arg()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "ssh to {} failed ({}): {}",
            target.as_arg(),
            output.status,
            stderr.trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Shell-safe single-quote wrapper. Replaces every `'` with `'\''`.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Flatten a multi-line command for dry-run display.
fn oneline(s: &str) -> String {
    let compact = s
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("; ");
    if compact.len() > 200 {
        format!("{}…", &compact[..200])
    } else {
        compact
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_quote_simple() {
        assert_eq!(shell_quote("hello"), "'hello'");
    }

    #[test]
    fn shell_quote_embeds_single_quotes_safely() {
        assert_eq!(shell_quote("a'b"), r"'a'\''b'");
    }

    #[test]
    fn recognises_common_ssh_key_types() {
        assert!(looks_like_ssh_pubkey("ssh-ed25519 AAAAC3... comment"));
        assert!(looks_like_ssh_pubkey("ssh-rsa AAAAB3... me@host"));
        assert!(looks_like_ssh_pubkey("ecdsa-sha2-nistp256 AAA..."));
    }

    #[test]
    fn rejects_non_pubkeys() {
        assert!(!looks_like_ssh_pubkey(""));
        assert!(!looks_like_ssh_pubkey("not an ssh key"));
        assert!(!looks_like_ssh_pubkey("-----BEGIN OPENSSH PRIVATE KEY-----"));
    }

    #[test]
    fn assert_ubuntu_accepts_ubuntu() {
        let os = "NAME=\"Ubuntu\"\nID=ubuntu\nVERSION_ID=\"24.04\"\n";
        assert!(assert_ubuntu(os, false).is_ok());
    }

    #[test]
    fn assert_ubuntu_rejects_debian() {
        let os = "NAME=\"Debian\"\nID=debian\nVERSION_ID=\"12\"\n";
        assert!(assert_ubuntu(os, false).is_err());
    }

    #[test]
    fn memory_bail_below_1024() {
        assert!(assert_enough_memory("800000", false).is_err()); // 800 MB
    }

    #[test]
    fn memory_ok_at_2048() {
        // 2 GB in KB
        assert!(assert_enough_memory("2097152", false).is_ok());
    }

    #[test]
    fn disk_bail_below_5gb() {
        assert!(assert_enough_disk("3", false).is_err());
    }

    #[test]
    fn disk_warn_at_10gb_but_ok() {
        assert!(assert_enough_disk("10", false).is_ok());
    }

    #[test]
    fn disk_happy_at_40gb() {
        assert!(assert_enough_disk("40", false).is_ok());
    }

    #[test]
    fn options_use_builtin_defaults_when_empty() {
        let cfg = HardenConfig::default();
        let opts = resolve_options(None, None, &cfg);
        assert_eq!(opts.new_user, "discourse");
        assert_eq!(opts.ssh_port, 2227);
        assert_eq!(opts.docker_install_url, "https://get.docker.com");
        assert_eq!(opts.swap_size_gb, 2);
        assert_eq!(opts.timezone, "UTC");
        assert!(opts.fail2ban);
        assert!(opts.unattended_security_upgrades);
        assert!(!opts.mosh);
        assert_eq!(opts.journald_max_use, "500M");
    }

    #[test]
    fn options_pick_up_config_block() {
        let cfg = HardenConfig {
            new_user: Some("ops".to_string()),
            ssh_port: Some(2299),
            mosh: Some(true),
            swap_size_gb: Some(0),
            ..HardenConfig::default()
        };
        let opts = resolve_options(None, None, &cfg);
        assert_eq!(opts.new_user, "ops");
        assert_eq!(opts.ssh_port, 2299);
        assert!(opts.mosh);
        assert_eq!(opts.swap_size_gb, 0);
        // Unset fields still get built-in defaults.
        assert_eq!(opts.timezone, "UTC");
    }

    #[test]
    fn cli_flags_override_config_block() {
        let cfg = HardenConfig {
            new_user: Some("ops".to_string()),
            ssh_port: Some(2299),
            ..HardenConfig::default()
        };
        let opts = resolve_options(Some("custom"), Some(40022), &cfg);
        assert_eq!(opts.new_user, "custom");
        assert_eq!(opts.ssh_port, 40022);
    }
}
