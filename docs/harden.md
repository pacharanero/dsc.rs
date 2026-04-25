# dsc harden

Turn a fresh Ubuntu server reachable via `ssh root@host` into a hardened box ready for `dsc install`. **WIP** — currently ships **stage 1 only**; stages 2 and 3 are in development.

## What stage 1 does

```text
dsc harden <host> --pubkey-file <path> [--new-user <name>] [--ssh-port <port>] [--ssh-user root]
```

Starting from a fresh `ssh root@host` that you can already log in to (typical for a cloud-provisioned VM with your initial SSH key on creation):

1. **Preflight** — confirms the remote is Ubuntu, has ≥ 1 GB RAM (warns < 2 GB), has ≥ 5 GB free on `/var` (warns < 30 GB), and that the SSH user is currently root.
2. **Creates the new user** with `adduser --disabled-password`. Skipped if the user already exists.
3. **Grants sudo NOPASSWD** by dropping a single-file snippet into `/etc/sudoers.d/`, validated through `visudo -cf` before being moved into place. Safer than editing the main sudoers, and idempotent.
4. **Installs the supplied pubkey** into the new user's `authorized_keys` with correct perms. Deduplicated, so re-running doesn't append duplicate lines.
5. **Verifies new-user SSH actually works** by opening a second SSH session as the new user and running `whoami`. **If this fails, the run errors out before any subsequent stage tightens sshd** — so a bad pubkey can never lock you out of the box. The original `ssh root@host` is still usable for debugging.

After stage 1, sshd is **still in its original state** — root SSH and password auth both still permitted. Stages 2+ tighten that down once we know the new-user path works.

## What stages 2 and 3 will add

Tracked in [`.marcus/harden-install-notes.md`](https://github.com/pacharanero/dsc/tree/main/.marcus) (private). Briefly:

- **Stage 2:** an `/etc/ssh/sshd_config.d/90-dsc-harden.conf` drop-in pinning the new SSH port, `PermitRootLogin no`, `PasswordAuthentication no`, `MaxAuthTries 3`, `LoginGraceTime 30`, `AllowUsers <new-user>`, idle session timeouts, and a curated cipher / KEX / MAC algorithm list. `sshd -t` validation before reload, then a third-session verification on the new port.
- **Stage 3:** UTC timezone, time-sync verified, swap file (2 GB by default), journald log cap, unattended security upgrades, fail2ban, rootless Docker (per the Bawmedical playbook — `setcap cap_net_bind_service=ep` on rootlesskit, `loginctl enable-linger`), and `ufw` opened for Discourse's standard ports.

All steps land idempotent — re-running `dsc harden` on an existing box will skip steps that are already configured rather than fight the existing state.

## Configuration (`[harden]` block)

Every flag has a sensible built-in default. You can override defaults globally in `dsc.toml`'s `[harden]` block, and override that on a per-run basis with the CLI flag. Resolution: **CLI flag → `[harden]` block → built-in default**.

```toml
[harden]
new_user                     = "discourse"
ssh_port                     = 2227
docker_install_url           = "https://get.docker.com"
docker_rootless              = true
swap_size_gb                 = 2          # 0 to skip
journald_max_use             = "500M"
timezone                     = "UTC"
unattended_security_upgrades = true
fail2ban                     = true
mosh                         = false      # opt-in; opens UDP 60000-61000
# sshd_ciphers, sshd_kex, sshd_macs override dsc's pinned modern lists
# extra_ufw_allow = ["3000/tcp", "192.168.1.0/24"]
```

Read [`dsc.example.toml`](https://github.com/pacharanero/dsc/blob/main/dsc.example.toml) for the full annotated block.

## Why publish a hardening routine?

A reasonable concern: doesn't publishing the exact steps `dsc harden` takes give attackers a blueprint? Short answer — yes a tiny bit, but no in any meaningful sense, and the auditability gain dwarfs the fingerprinting cost.

**What an attacker actually learns:**

- The defaults you'll likely run (port 2227, user `discourse`, fail2ban tuning, ufw rule list). Mostly fingerprinting value, not exploit value. Port scanners find non-default SSH in under a minute anyway, and `PasswordAuthentication no` makes the username irrelevant for brute force.
- The list of packages and configs. Already implied by "this is a Discourse server"; nothing new.
- Bug surface in the hardening tool itself. This is the real risk — see SECURITY.md.

**What you gain by publishing:**

- **Auditability.** You can read [`src/commands/harden.rs`](https://github.com/pacharanero/dsc/blob/main/src/commands/harden.rs) before running it as root on a fresh box. The alternative — closed binary with full root — is much worse for trust.
- **Crowd review.** Security researchers can spot mistakes before attackers do. Every major hardening tool is open (ansible-hardening, OpenSCAP, dev-sec.io, CIS, lynis) for exactly this reason.
- **Reproducibility.** You can rebuild the same posture by hand from the published steps if you don't want the tool.

**Mitigations dsc applies:**

- Every "magic" default is configurable — port, username, swap size, packages, the lot. Power users who want extra obscurity can deviate; defaults stay readable for everyone else.
- No secrets, backdoor users, or default keys are ever shipped.
- Every non-obvious step has an inline `// why:` comment so reviewers can reason quickly.
- Vulnerabilities should be reported privately — see [SECURITY.md](https://github.com/pacharanero/dsc/blob/main/SECURITY.md).

**The one genuinely new risk:** once enough people use `dsc harden`, attackers can fingerprint dsc-hardened boxes (specific port + user + ufw rule combination + sshd algorithm list). For most users this is acceptable — being identifiably-hardened is still safer than being unhardened — but worth knowing so power users can opt out of any default they care about.

## Testing the new user before going further

Stage 1's self-lockout guard catches the common failure mode (bad pubkey, wrong username, sshd quirks), but it's still good practice to test the new-user SSH yourself before invoking stage 2. From your laptop:

```bash
ssh -i ~/.ssh/<server-key> <new-user>@<host> 'sudo whoami'   # should print "root"
```

If that succeeds, you're safe to run subsequent stages.

## Examples

```bash
# Bare-minimum first run on a fresh Hetzner box; uses every default.
dsc harden 192.0.2.1 --pubkey-file ~/.ssh/myserver.pub

# Dry-run first to inspect the exact commands.
dsc --dry-run harden 192.0.2.1 --pubkey-file ~/.ssh/myserver.pub

# Override a default on a per-run basis.
dsc harden 192.0.2.1 --pubkey-file ~/.ssh/myserver.pub --new-user ops --ssh-port 40022
```

## Related

- [`dsc install`](https://github.com/pacharanero/dsc) — declarative Discourse install on a hardened box. WIP, lands after `dsc harden` is feature-complete.
- [`dsc update`](update.md) — runs OS + Discourse rebuilds via SSH. Already shipped; complements `dsc harden` once the box is provisioned.
- [`dsc config check`](config.md) — verifies API and SSH connectivity for every install in `dsc.toml`. Good first thing to run after `dsc install` adds your new box.
