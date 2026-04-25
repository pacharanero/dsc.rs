use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "dsc")]
#[command(about = "Discourse CLI", long_about = None)]
pub struct Cli {
    /// Path to the config file. If omitted, dsc searches standard locations.
    #[arg(long, short = 'c')]
    pub config: Option<PathBuf>,
    /// Describe destructive actions without sending them. Read-only commands
    /// ignore the flag.
    #[arg(long, short = 'n', global = true)]
    pub dry_run: bool,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List configured Discourses.
    #[command(visible_alias = "ls")]
    List {
        /// Output format for the listing.
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: OutputFormat,
        /// Filter by tags (comma/semicolon separated, match-any).
        #[arg(long, value_name = "tag1,tag2")]
        tags: Option<String>,
        /// Open each listed Discourse base URL in a browser tab/window.
        #[arg(long, short = 'o')]
        open: bool,
        /// Include empty results and verbose listing details where supported.
        #[arg(long, short = 'v')]
        verbose: bool,
        #[command(subcommand)]
        command: Option<ListCommand>,
    },
    /// Add one or more Discourses to the config.
    #[command(visible_alias = "a")]
    Add {
        /// Comma-separated discourse names to add.
        names: String,
        /// Prompt for additional optional fields while adding.
        #[arg(long, short = 'i')]
        interactive: bool,
    },
    /// Import Discourses from a file or stdin.
    #[command(visible_alias = "imp")]
    Import {
        /// Path to import input (text/CSV). Reads stdin when omitted.
        path: Option<PathBuf>,
    },
    /// Run remote OS + Discourse update workflow for one or all Discourses.
    #[command(visible_alias = "up")]
    Update {
        /// Discourse name, or 'all' to update every configured Discourse.
        name: String,
        /// Parallel update mode for `dsc update all`.
        #[arg(long, short = 'p')]
        parallel: bool,
        /// Maximum workers when parallel mode is enabled (default: 3).
        #[arg(long, short = 'm')]
        max: Option<usize>,
        /// Disable changelog posting (posting prompt is on by default).
        #[arg(long = "no-changelog", action = ArgAction::SetFalse, default_value_t = true)]
        post_changelog: bool,
        /// Auto-confirm changelog posting prompt (non-interactive mode).
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Manage custom emoji.
    #[command(visible_alias = "em")]
    Emoji {
        #[command(subcommand)]
        command: EmojiCommand,
    },
    /// Pull/push/sync topics as local Markdown.
    #[command(visible_alias = "t")]
    Topic {
        #[command(subcommand)]
        command: TopicCommand,
    },
    /// List/copy/pull/push categories.
    #[command(visible_alias = "cat")]
    Category {
        #[command(subcommand)]
        command: CategoryCommand,
    },
    /// List/inspect/copy groups.
    #[command(visible_alias = "grp")]
    Group {
        #[command(subcommand)]
        command: GroupCommand,
    },
    /// Operations that act from a user's perspective.
    #[command(visible_alias = "usr")]
    User {
        #[command(subcommand)]
        command: UserCommand,
    },
    /// Send invites — single or bulk from a file.
    #[command(visible_alias = "inv")]
    Invite {
        #[command(subcommand)]
        command: InviteCommand,
    },
    /// Manage API keys (admin scope).
    #[command(visible_alias = "ak")]
    ApiKey {
        #[command(subcommand)]
        command: ApiKeyCommand,
    },
    /// Send and list private messages.
    #[command(visible_alias = "msg")]
    Pm {
        #[command(subcommand)]
        command: PmCommand,
    },
    /// Create/list/restore backups.
    #[command(visible_alias = "bk")]
    Backup {
        #[command(subcommand)]
        command: BackupCommand,
    },
    /// List/pull/push color palettes.
    #[command(visible_alias = "pal")]
    Palette {
        #[command(subcommand)]
        command: PaletteCommand,
    },
    /// List/install/remove plugins.
    #[command(visible_alias = "plg")]
    Plugin {
        #[command(subcommand)]
        command: PluginCommand,
    },
    /// List/install/remove/pull/push/duplicate themes.
    #[command(visible_alias = "th")]
    Theme {
        #[command(subcommand)]
        command: ThemeCommand,
    },
    /// Update site settings.
    #[command(visible_alias = "set")]
    Setting {
        #[command(subcommand)]
        command: SettingCommand,
    },
    /// List tags and apply/remove them on topics.
    #[command(visible_alias = "tg")]
    Tag {
        #[command(subcommand)]
        command: TagCommand,
    },
    /// Post-level operations: edit / delete / move.
    #[command(visible_alias = "po")]
    Post {
        #[command(subcommand)]
        command: PostCommand,
    },
    /// Open a Discourse in the default browser.
    #[command(visible_alias = "o")]
    Open {
        /// Discourse name.
        discourse: String,
    },
    /// Harden a fresh Ubuntu server reachable via `ssh root@host`.
    ///
    /// **Stage 1 (current):** creates a non-root sudo user, installs the
    /// given pubkey to their authorized_keys, and verifies the new-user
    /// SSH login works. Does NOT yet tighten sshd_config, install Docker
    /// / fail2ban / etc — those come in follow-up releases.
    ///
    /// Defaults can be overridden in the `[harden]` block of dsc.toml;
    /// the flags below override that block on a per-run basis.
    #[command(visible_alias = "hd")]
    Harden {
        /// Target hostname or IP (reachable via SSH).
        host: String,
        /// Username to SSH in as initially. Defaults to `root`, which is
        /// what a fresh cloud-provisioned box typically has.
        #[arg(long, default_value = "root")]
        ssh_user: String,
        /// Username for the new sudo-enabled non-root account. Overrides
        /// `[harden].new_user` from dsc.toml. Built-in default: `discourse`.
        #[arg(long)]
        new_user: Option<String>,
        /// SSH port to move the daemon to in stage 2. Overrides
        /// `[harden].ssh_port`. Built-in default: 2227. Parsed now so the
        /// CLI is stable; not yet applied in stage 1.
        #[arg(long)]
        ssh_port: Option<u16>,
        /// Path to an SSH public key file whose contents will be added to
        /// the new user's authorized_keys. A typical value is
        /// `~/.ssh/<hostname>.pub` — the per-server keypair pattern in
        /// the Bawmedical hardening playbook.
        #[arg(long)]
        pubkey_file: PathBuf,
    },
    /// Search topics on a Discourse.
    #[command(visible_alias = "s")]
    Search {
        /// Discourse name.
        discourse: String,
        /// Search query (passed through verbatim, including any
        /// Discourse filter syntax like `category:foo` or `@user`).
        query: String,
        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
    },
    /// Upload a file. Prints the resulting upload:// short URL by default.
    #[command(visible_alias = "u")]
    Upload {
        /// Discourse name.
        discourse: String,
        /// Path to the file to upload.
        file: PathBuf,
        /// Discourse upload context. Default `composer` is correct for
        /// embedding in posts; other values include `avatar`,
        /// `profile_background`, `card_background`, `custom_emoji`.
        #[arg(long, short = 't', default_value = "composer")]
        upload_type: String,
        /// Output format. Text mode prints just the short URL.
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
    },
    /// Inspect and validate configuration.
    #[command(visible_alias = "cfg")]
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Generate shell completion scripts.
    #[command(visible_alias = "comp")]
    Completions {
        /// Target shell.
        #[arg(value_enum)]
        shell: CompletionShell,
        /// Output directory. Prints to stdout when omitted.
        #[arg(long, short = 'd')]
        dir: Option<PathBuf>,
    },
    /// Print the dsc version.
    #[command(visible_alias = "ver")]
    Version,
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Probe each configured Discourse: API auth and (optionally) SSH reachability.
    #[command(visible_alias = "ck")]
    Check {
        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
        /// Skip the SSH reachability probe.
        #[arg(long)]
        skip_ssh: bool,
    },
}

#[derive(Subcommand)]
pub enum ListCommand {
    /// Sort discourse entries by name and rewrite config in-place.
    /// Also inserts placeholder values for unset template keys.
    #[command(visible_alias = "ty")]
    Tidy,
}

#[derive(Subcommand)]
pub enum EmojiCommand {
    /// Upload one emoji file, or bulk-upload from a directory.
    #[command(visible_alias = "a")]
    Add {
        /// Discourse name.
        discourse: String,
        /// Local file or directory path.
        emoji_path: PathBuf,
        /// Optional emoji name (file uploads only).
        emoji_name: Option<String>,
    },

    /// List custom emojis on a Discourse.
    #[command(visible_alias = "ls")]
    List {
        /// Discourse name.
        discourse: String,
        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
        /// Include additional fields where supported.
        #[arg(long, short = 'v')]
        verbose: bool,
        /// Render inline images when terminal protocol support is available.
        #[arg(long, short = 'i')]
        inline: bool,
    },
}

#[derive(Subcommand)]
pub enum TopicCommand {
    /// Pull a topic to a local Markdown file.
    #[command(visible_alias = "pl")]
    Pull {
        /// Discourse name.
        discourse: String,
        /// Topic ID.
        topic_id: u64,
        /// Destination file or directory (auto-derived when omitted).
        local_path: Option<PathBuf>,
    },
    /// Push a local Markdown file to a topic.
    #[command(visible_alias = "ps")]
    Push {
        /// Discourse name.
        discourse: String,
        /// Topic ID.
        topic_id: u64,
        /// Local Markdown file path.
        local_path: PathBuf,
    },
    /// Sync a topic and local Markdown file using newest timestamp.
    #[command(visible_alias = "sy")]
    Sync {
        /// Discourse name.
        discourse: String,
        /// Topic ID.
        topic_id: u64,
        /// Local Markdown file path.
        local_path: PathBuf,
        /// Skip sync confirmation prompt.
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Reply to a topic with content from a file or stdin.
    #[command(visible_alias = "r")]
    Reply {
        /// Discourse name.
        discourse: String,
        /// Topic ID.
        topic_id: u64,
        /// Input file path. Reads stdin when omitted or `-`.
        local_path: Option<PathBuf>,
    },
    /// Create a new topic in a category, body from a file or stdin.
    #[command(visible_alias = "n")]
    New {
        /// Discourse name.
        discourse: String,
        /// Target category ID.
        category_id: u64,
        /// Topic title.
        #[arg(long, short = 't')]
        title: String,
        /// Input file path. Reads stdin when omitted or `-`.
        local_path: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum CategoryCommand {
    /// List categories.
    #[command(visible_alias = "ls")]
    List {
        /// Discourse name.
        discourse: String,
        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
        /// Include additional fields where supported.
        #[arg(long, short = 'v')]
        verbose: bool,
        /// Show category hierarchy tree.
        #[arg(long)]
        tree: bool,
    },
    /// Copy a category to another Discourse.
    #[command(visible_alias = "cp")]
    Copy {
        /// Source discourse name.
        discourse: String,
        /// Target discourse name (defaults to source when omitted).
        #[arg(long, short = 't')]
        target: Option<String>,
        /// Category ID or slug.
        category: String,
    },
    /// Pull all topics from a category into local Markdown files.
    #[command(visible_alias = "pl")]
    Pull {
        /// Discourse name.
        discourse: String,
        /// Category ID or slug.
        category: String,
        /// Destination directory (auto-derived when omitted).
        local_path: Option<PathBuf>,
    },
    /// Push local Markdown files into a category.
    #[command(visible_alias = "ps")]
    Push {
        /// Discourse name.
        discourse: String,
        /// Category ID or slug.
        category: String,
        /// Local directory containing Markdown files.
        local_path: PathBuf,
    },
}

#[derive(Subcommand)]
pub enum GroupCommand {
    /// List groups.
    #[command(visible_alias = "ls")]
    List {
        /// Discourse name.
        discourse: String,
        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
        /// Include additional fields where supported.
        #[arg(long, short = 'v')]
        verbose: bool,
    },
    /// Show group details.
    #[command(visible_alias = "i")]
    Info {
        /// Discourse name.
        discourse: String,
        /// Group ID.
        group: u64,
        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value = "json")]
        format: StructuredFormat,
    },
    /// List members of a group.
    #[command(visible_alias = "m")]
    Members {
        /// Discourse name.
        discourse: String,
        /// Group ID.
        group: u64,
        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
    },
    /// Copy a group to another Discourse.
    #[command(visible_alias = "cp")]
    Copy {
        /// Source discourse name.
        discourse: String,
        /// Target discourse name (defaults to source when omitted).
        #[arg(long, short = 't')]
        target: Option<String>,
        /// Group ID.
        group: u64,
    },
    /// Bulk add members to a group from a file (or stdin) of email addresses.
    #[command(visible_alias = "a")]
    Add {
        /// Discourse name.
        discourse: String,
        /// Group ID.
        group: u64,
        /// Path to a file of email addresses (one per line; blank
        /// lines and `#` comments are ignored). Reads stdin when
        /// omitted or `-`.
        local_path: Option<PathBuf>,
        /// Send Discourse notifications to added users.
        #[arg(long)]
        notify: bool,
    },
}

#[derive(Subcommand)]
pub enum BackupCommand {
    /// Create a new backup.
    #[command(visible_alias = "cr")]
    Create {
        /// Discourse name.
        discourse: String,
    },
    /// List backups.
    #[command(visible_alias = "ls")]
    List {
        /// Discourse name.
        discourse: String,
        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: OutputFormat,
        /// Include additional fields where supported.
        #[arg(long, short = 'v')]
        verbose: bool,
    },
    /// Restore a backup.
    #[command(visible_alias = "rs")]
    Restore {
        /// Discourse name.
        discourse: String,
        /// Backup filename/path on the target system.
        backup_path: String,
    },
}

#[derive(Subcommand)]
pub enum PaletteCommand {
    /// List color palettes.
    #[command(visible_alias = "ls")]
    List {
        /// Discourse name.
        discourse: String,
        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
        /// Include additional fields where supported.
        #[arg(long, short = 'v')]
        verbose: bool,
    },
    /// Pull a palette to local JSON.
    #[command(visible_alias = "pl")]
    Pull {
        /// Discourse name.
        discourse: String,
        /// Palette ID.
        palette_id: u64,
        /// Destination file path (auto-derived when omitted).
        local_path: Option<PathBuf>,
    },
    /// Push local JSON to create or update a palette.
    #[command(visible_alias = "ps")]
    Push {
        /// Discourse name.
        discourse: String,
        /// Local JSON file path.
        local_path: PathBuf,
        /// Palette ID to update (creates a new palette when omitted).
        palette_id: Option<u64>,
    },
}

#[derive(Subcommand)]
pub enum PluginCommand {
    /// List installed plugins.
    #[command(visible_alias = "ls")]
    List {
        /// Discourse name.
        discourse: String,
        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
        /// Include additional fields where supported.
        #[arg(long, short = 'v')]
        verbose: bool,
    },
    /// Install a plugin from URL.
    #[command(visible_alias = "i")]
    Install {
        /// Discourse name.
        discourse: String,
        /// Plugin repository URL.
        url: String,
    },
    /// Remove a plugin by name.
    #[command(visible_alias = "rm")]
    Remove {
        /// Discourse name.
        discourse: String,
        /// Plugin name.
        name: String,
    },
}

#[derive(Subcommand)]
pub enum ThemeCommand {
    /// List installed themes.
    #[command(visible_alias = "ls")]
    List {
        /// Discourse name.
        discourse: String,
        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
        /// Include additional fields where supported.
        #[arg(long, short = 'v')]
        verbose: bool,
    },
    /// Install a theme from URL.
    #[command(visible_alias = "i")]
    Install {
        /// Discourse name.
        discourse: String,
        /// Theme repository URL.
        url: String,
    },
    /// Remove a theme by name.
    #[command(visible_alias = "rm")]
    Remove {
        /// Discourse name.
        discourse: String,
        /// Theme name.
        name: String,
    },
    /// Pull a theme to a local JSON file.
    #[command(visible_alias = "pl")]
    Pull {
        /// Discourse name.
        discourse: String,
        /// Theme ID (from `dsc theme list`).
        theme_id: u64,
        /// Destination file path (auto-derived from theme name when omitted).
        local_path: Option<PathBuf>,
    },
    /// Push a local JSON file to create or update a theme.
    #[command(visible_alias = "ps")]
    Push {
        /// Discourse name.
        discourse: String,
        /// Local JSON file path.
        local_path: PathBuf,
        /// Theme ID to update (creates a new theme when omitted).
        theme_id: Option<u64>,
    },
    /// Duplicate a theme and print the new theme ID.
    #[command(visible_alias = "dup")]
    Duplicate {
        /// Discourse name.
        discourse: String,
        /// Theme ID to duplicate (from `dsc theme list`).
        theme_id: u64,
    },
}

#[derive(Subcommand)]
pub enum PmCommand {
    /// Send a private message.
    #[command(visible_alias = "s")]
    Send {
        /// Discourse name.
        discourse: String,
        /// Recipient(s) — comma-separated usernames or group names.
        recipients: String,
        /// PM title / subject.
        #[arg(long, short = 't')]
        title: String,
        /// Input file path. Reads stdin when omitted or `-`.
        local_path: Option<PathBuf>,
    },
    /// List PMs for a user.
    #[command(visible_alias = "ls")]
    List {
        /// Discourse name.
        discourse: String,
        /// Username whose PMs to list.
        username: String,
        /// Direction / view: inbox | sent | archive | unread | new.
        #[arg(long, short = 'd', default_value = "inbox")]
        direction: String,
        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
    },
}

#[derive(Subcommand)]
pub enum ApiKeyCommand {
    /// List API keys.
    #[command(visible_alias = "ls")]
    List {
        /// Discourse name.
        discourse: String,
        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
    },
    /// Create a new API key. The secret is only shown at creation time —
    /// capture it from the output.
    #[command(visible_alias = "cr")]
    Create {
        /// Discourse name.
        discourse: String,
        /// Description / label for the key (shown in admin UI).
        description: String,
        /// Username the key acts as. Omit for a global all-users key.
        #[arg(long, short = 'u')]
        username: Option<String>,
        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
    },
    /// Revoke an API key by ID.
    #[command(visible_alias = "rm")]
    Revoke {
        /// Discourse name.
        discourse: String,
        /// API key ID (from `dsc api-key list`).
        key_id: u64,
    },
}

#[derive(Subcommand)]
pub enum InviteCommand {
    /// Invite a single email address.
    #[command(visible_alias = "s")]
    Send {
        /// Discourse name.
        discourse: String,
        /// Email address to invite.
        email: String,
        /// Add invitee to one or more groups on accept (repeatable).
        #[arg(long, short = 'g')]
        group: Vec<u64>,
        /// Land the invitee on a specific topic on accept.
        #[arg(long, short = 't')]
        topic: Option<u64>,
        /// Custom invitation message.
        #[arg(long, short = 'm')]
        message: Option<String>,
    },
    /// Bulk-invite from a file (or stdin) of email addresses.
    #[command(visible_alias = "b")]
    Bulk {
        /// Discourse name.
        discourse: String,
        /// Path to a file of email addresses (one per line; blank lines and
        /// `#` comments ignored). Reads stdin when omitted or `-`.
        local_path: Option<PathBuf>,
        /// Add every invitee to one or more groups on accept (repeatable).
        #[arg(long, short = 'g')]
        group: Vec<u64>,
        /// Land every invitee on a specific topic on accept.
        #[arg(long, short = 't')]
        topic: Option<u64>,
        /// Custom invitation message attached to each invite.
        #[arg(long, short = 'm')]
        message: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum UserCommand {
    /// List users via the admin users endpoint.
    #[command(visible_alias = "ls")]
    List {
        /// Discourse name.
        discourse: String,
        /// Listing type: active | new | staff | suspended | silenced | staged.
        #[arg(long, short = 'l', default_value = "active")]
        listing: String,
        /// Page number (Discourse paginates 100 per page).
        #[arg(long, short = 'p', default_value_t = 1)]
        page: u32,
        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
    },
    /// Show detailed info for a user.
    #[command(visible_alias = "i")]
    Info {
        /// Discourse name.
        discourse: String,
        /// Username.
        username: String,
        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
    },
    /// Suspend a user.
    #[command(visible_alias = "sus")]
    Suspend {
        /// Discourse name.
        discourse: String,
        /// Username.
        username: String,
        /// When the suspension ends. ISO-8601 timestamp (e.g.
        /// `2026-12-31T00:00:00Z`) or `forever`.
        #[arg(long, short = 'u', default_value = "forever")]
        until: String,
        /// Reason shown to the user and in the audit log.
        #[arg(long, short = 'r', default_value = "")]
        reason: String,
    },
    /// Remove a suspension from a user.
    #[command(visible_alias = "uns")]
    Unsuspend {
        /// Discourse name.
        discourse: String,
        /// Username.
        username: String,
    },
    /// Silence a user (prevents posting; less visible than suspend).
    #[command(visible_alias = "sil")]
    Silence {
        /// Discourse name.
        discourse: String,
        /// Username.
        username: String,
        /// When the silence ends. ISO-8601 timestamp; empty means
        /// indefinite.
        #[arg(long, short = 'u', default_value = "")]
        until: String,
        /// Reason shown to the user and in the audit log.
        #[arg(long, short = 'r', default_value = "")]
        reason: String,
    },
    /// Lift a silence on a user.
    #[command(visible_alias = "unsil")]
    Unsilence {
        /// Discourse name.
        discourse: String,
        /// Username.
        username: String,
    },
    /// Grant the user the admin or moderator role.
    #[command(visible_alias = "pr")]
    Promote {
        /// Discourse name.
        discourse: String,
        /// Username.
        username: String,
        /// Role to grant.
        #[arg(long, short = 'r', value_enum)]
        role: RoleArg,
    },
    /// Revoke the user's admin or moderator role.
    #[command(visible_alias = "de")]
    Demote {
        /// Discourse name.
        discourse: String,
        /// Username.
        username: String,
        /// Role to revoke.
        #[arg(long, short = 'r', value_enum)]
        role: RoleArg,
    },
    /// Create a new user. `--approve` also marks the account approved
    /// (needed when site requires manual approval). Password is either
    /// supplied via stdin (`--password-stdin`) or omitted — in the
    /// latter case the user will have to set one via the reset flow.
    #[command(visible_alias = "cr")]
    Create {
        /// Discourse name.
        discourse: String,
        /// New user's email address.
        email: String,
        /// New user's username.
        username: String,
        /// Display name (optional).
        #[arg(long, short = 'N')]
        name: Option<String>,
        /// Read the password from stdin instead of auto-reset.
        #[arg(long)]
        password_stdin: bool,
        /// Also mark the user approved (for sites with manual approval).
        #[arg(long)]
        approve: bool,
    },
    /// Trigger Discourse's password-reset email flow for a user.
    #[command(name = "password-reset", visible_aliases = ["pwreset", "pw-reset"])]
    PasswordReset {
        /// Discourse name.
        discourse: String,
        /// Username or email.
        username: String,
    },
    /// Set a user's primary email address. Requires admin scope.
    #[command(name = "email-set", visible_alias = "email")]
    EmailSet {
        /// Discourse name.
        discourse: String,
        /// Username.
        username: String,
        /// New email address.
        email: String,
    },
    /// Show a user's recent public activity (topics + replies by default).
    ///
    /// Built for the "archive my own activity to a journal forum" loop —
    /// pipe the markdown output straight into `dsc topic reply`/`topic new`.
    #[command(visible_alias = "act")]
    Activity {
        /// Discourse name (the *source* forum to read activity from).
        discourse: String,
        /// Username whose activity to read.
        username: String,
        /// How far back to look. Accepts `7d`, `24h`, `30m`, `1w`, `90s`, or
        /// an ISO-8601 timestamp / date. Omit to fetch everything available.
        #[arg(long, short = 's')]
        since: Option<String>,
        /// Action types to include, comma-separated. Default: topics,replies.
        /// Also recognises: mentions, quotes, likes, edits, responses.
        #[arg(long, short = 't', default_value = "topics,replies")]
        types: String,
        /// Hard cap on number of items returned.
        #[arg(long, short = 'L')]
        limit: Option<u32>,
        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value = "markdown")]
        format: ActivityFormatArg,
    },
    /// Manage a user's group memberships.
    #[command(visible_alias = "g")]
    Groups {
        #[command(subcommand)]
        command: UserGroupsCommand,
    },
}

#[derive(ValueEnum, Clone, Copy)]
pub enum ActivityFormatArg {
    Text,
    Json,
    #[value(alias = "yml")]
    Yaml,
    #[value(alias = "md")]
    Markdown,
    Csv,
}

#[derive(ValueEnum, Clone, Copy)]
pub enum RoleArg {
    Admin,
    Moderator,
}

#[derive(Subcommand)]
pub enum UserGroupsCommand {
    /// List the groups a user belongs to.
    #[command(visible_alias = "ls")]
    List {
        /// Discourse name.
        discourse: String,
        /// Target username.
        username: String,
        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
    },
    /// Add a user to a group.
    #[command(visible_alias = "a")]
    Add {
        /// Discourse name.
        discourse: String,
        /// Target username.
        username: String,
        /// Group ID.
        group_id: u64,
        /// Send Discourse notification to the user.
        #[arg(long)]
        notify: bool,
    },
    /// Remove a user from a group.
    #[command(visible_alias = "rm")]
    Remove {
        /// Discourse name.
        discourse: String,
        /// Target username.
        username: String,
        /// Group ID.
        group_id: u64,
    },
}

#[derive(Subcommand)]
pub enum PostCommand {
    /// Edit a post by ID. Reads the new body from file or stdin.
    #[command(visible_alias = "e")]
    Edit {
        /// Discourse name.
        discourse: String,
        /// Post ID.
        post_id: u64,
        /// Input file path. Reads stdin when omitted or `-`.
        local_path: Option<PathBuf>,
    },
    /// Delete a post by ID.
    #[command(visible_alias = "rm")]
    Delete {
        /// Discourse name.
        discourse: String,
        /// Post ID.
        post_id: u64,
    },
    /// Move a post to a different topic.
    #[command(visible_alias = "mv")]
    Move {
        /// Discourse name.
        discourse: String,
        /// Post ID to move.
        post_id: u64,
        /// Destination topic ID.
        #[arg(long = "to-topic", short = 't')]
        to_topic: u64,
    },
}

#[derive(Subcommand)]
pub enum TagCommand {
    /// List every tag on the Discourse.
    #[command(visible_alias = "ls")]
    List {
        /// Discourse name.
        discourse: String,
        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
    },
    /// Add a tag to a topic.
    #[command(visible_alias = "a")]
    Apply {
        /// Discourse name.
        discourse: String,
        /// Topic ID.
        topic_id: u64,
        /// Tag to add.
        tag: String,
    },
    /// Remove a tag from a topic.
    #[command(visible_alias = "rm")]
    Remove {
        /// Discourse name.
        discourse: String,
        /// Topic ID.
        topic_id: u64,
        /// Tag to remove.
        tag: String,
    },
}

#[derive(Subcommand)]
pub enum SettingCommand {
    /// Set a site setting on a Discourse (or all tagged Discourses).
    #[command(visible_alias = "s")]
    Set {
        /// Discourse name. Required when targeting a single discourse.
        discourse: String,
        /// Setting key.
        setting: String,
        /// Setting value.
        value: String,
        /// Optional tag filter (comma/semicolon separated, match-any). Ignored when discourse is specified.
        #[arg(long, value_name = "tag1,tag2")]
        tags: Option<String>,
    },

    /// Get the current value of a site setting.
    #[command(visible_alias = "g")]
    Get {
        /// Discourse name.
        discourse: String,
        /// Setting key.
        setting: String,
    },

    /// List all site settings.
    #[command(visible_alias = "ls")]
    List {
        /// Discourse name.
        discourse: String,
        /// Output format.
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
        /// Show output even when list is empty.
        #[arg(long, short = 'v')]
        verbose: bool,
    },
}

#[derive(ValueEnum, Clone, Copy)]
pub enum CompletionShell {
    /// Bash shell.
    Bash,
    /// Zsh shell.
    Zsh,
    /// Fish shell.
    Fish,
}

impl From<CompletionShell> for Shell {
    fn from(value: CompletionShell) -> Self {
        match value {
            CompletionShell::Bash => Shell::Bash,
            CompletionShell::Zsh => Shell::Zsh,
            CompletionShell::Fish => Shell::Fish,
        }
    }
}

#[derive(ValueEnum, Clone)]
pub enum OutputFormat {
    /// Plain text.
    #[value(alias = "plaintext")]
    Text,
    /// Markdown list.
    Markdown,
    /// Markdown table.
    MarkdownTable,
    /// Pretty JSON.
    Json,
    /// YAML.
    #[value(alias = "yml")]
    Yaml,
    /// CSV.
    Csv,
    /// One base URL per line (pipe-friendly).
    #[value(alias = "url")]
    Urls,
}

#[derive(ValueEnum, Clone, Copy)]
pub enum ListFormat {
    /// Plain text.
    Text,
    /// Pretty JSON.
    Json,
    /// YAML.
    #[value(alias = "yml")]
    Yaml,
}

#[derive(ValueEnum, Clone, Copy)]
pub enum StructuredFormat {
    /// Pretty JSON.
    Json,
    /// YAML.
    #[value(alias = "yml")]
    Yaml,
}
