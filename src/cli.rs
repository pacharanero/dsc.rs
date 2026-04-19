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
    /// Manage a user's group memberships.
    #[command(visible_alias = "g")]
    Groups {
        #[command(subcommand)]
        command: UserGroupsCommand,
    },
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
