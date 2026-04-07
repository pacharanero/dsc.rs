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
    Add {
        /// Comma-separated discourse names to add.
        names: String,
        /// Prompt for additional optional fields while adding.
        #[arg(long, short = 'i')]
        interactive: bool,
    },
    /// Import Discourses from a file or stdin.
    Import {
        /// Path to import input (text/CSV). Reads stdin when omitted.
        path: Option<PathBuf>,
    },
    /// Run remote OS + Discourse update workflow for one or all Discourses.
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
    Emoji {
        #[command(subcommand)]
        command: EmojiCommand,
    },
    /// Pull/push/sync topics as local Markdown.
    Topic {
        #[command(subcommand)]
        command: TopicCommand,
    },
    /// List/copy/pull/push categories.
    Category {
        #[command(subcommand)]
        command: CategoryCommand,
    },
    /// List/inspect/copy groups.
    Group {
        #[command(subcommand)]
        command: GroupCommand,
    },
    /// Create/list/restore backups.
    Backup {
        #[command(subcommand)]
        command: BackupCommand,
    },
    /// List/pull/push color palettes.
    Palette {
        #[command(subcommand)]
        command: PaletteCommand,
    },
    /// List/install/remove plugins.
    Plugin {
        #[command(subcommand)]
        command: PluginCommand,
    },
    /// List/install/remove/pull/push/duplicate themes.
    Theme {
        #[command(subcommand)]
        command: ThemeCommand,
    },
    /// Update site settings.
    Setting {
        #[command(subcommand)]
        command: SettingCommand,
    },
    /// Generate shell completion scripts.
    Completions {
        /// Target shell.
        #[arg(value_enum)]
        shell: CompletionShell,
        /// Output directory. Prints to stdout when omitted.
        #[arg(long, short = 'd')]
        dir: Option<PathBuf>,
    },
    /// Print the dsc version.
    Version,
}

#[derive(Subcommand)]
pub enum ListCommand {
    /// Sort discourse entries by name and rewrite config in-place.
    /// Also inserts placeholder values for unset template keys.
    Tidy,
}

#[derive(Subcommand)]
pub enum EmojiCommand {
    /// Upload one emoji file, or bulk-upload from a directory.
    Add {
        /// Discourse name.
        discourse: String,
        /// Local file or directory path.
        emoji_path: PathBuf,
        /// Optional emoji name (file uploads only).
        emoji_name: Option<String>,
    },

    /// List custom emojis on a Discourse.
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
    Pull {
        /// Discourse name.
        discourse: String,
        /// Topic ID.
        topic_id: u64,
        /// Destination file or directory (auto-derived when omitted).
        local_path: Option<PathBuf>,
    },
    /// Push a local Markdown file to a topic.
    Push {
        /// Discourse name.
        discourse: String,
        /// Local Markdown file path.
        local_path: PathBuf,
        /// Topic ID.
        topic_id: u64,
    },
    /// Sync a topic and local Markdown file using newest timestamp.
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
}

#[derive(Subcommand)]
pub enum CategoryCommand {
    /// List categories.
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
    Pull {
        /// Discourse name.
        discourse: String,
        /// Category ID or slug.
        category: String,
        /// Destination directory (auto-derived when omitted).
        local_path: Option<PathBuf>,
    },
    /// Push local Markdown files into a category.
    Push {
        /// Discourse name.
        discourse: String,
        /// Local directory containing Markdown files.
        local_path: PathBuf,
        /// Category ID or slug.
        category: String,
    },
}

#[derive(Subcommand)]
pub enum GroupCommand {
    /// List groups.
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
    Copy {
        /// Source discourse name.
        discourse: String,
        /// Target discourse name (defaults to source when omitted).
        #[arg(long, short = 't')]
        target: Option<String>,
        /// Group ID.
        group: u64,
    },
}

#[derive(Subcommand)]
pub enum BackupCommand {
    /// Create a new backup.
    Create {
        /// Discourse name.
        discourse: String,
    },
    /// List backups.
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
    Pull {
        /// Discourse name.
        discourse: String,
        /// Palette ID.
        palette_id: u64,
        /// Destination file path (auto-derived when omitted).
        local_path: Option<PathBuf>,
    },
    /// Push local JSON to create or update a palette.
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
    Install {
        /// Discourse name.
        discourse: String,
        /// Plugin repository URL.
        url: String,
    },
    /// Remove a plugin by name.
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
    Install {
        /// Discourse name.
        discourse: String,
        /// Theme repository URL.
        url: String,
    },
    /// Remove a theme by name.
    Remove {
        /// Discourse name.
        discourse: String,
        /// Theme name.
        name: String,
    },
    /// Pull a theme to a local JSON file.
    Pull {
        /// Discourse name.
        discourse: String,
        /// Theme ID (from `dsc theme list`).
        theme_id: u64,
        /// Destination file path (auto-derived from theme name when omitted).
        local_path: Option<PathBuf>,
    },
    /// Push a local JSON file to create or update a theme.
    Push {
        /// Discourse name.
        discourse: String,
        /// Local JSON file path.
        local_path: PathBuf,
        /// Theme ID to update (creates a new theme when omitted).
        theme_id: Option<u64>,
    },
    /// Duplicate a theme and print the new theme ID.
    Duplicate {
        /// Discourse name.
        discourse: String,
        /// Theme ID to duplicate (from `dsc theme list`).
        theme_id: u64,
    },
}

#[derive(Subcommand)]
pub enum SettingCommand {
    /// Set a site setting on a Discourse (or all tagged Discourses).
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
    Get {
        /// Discourse name.
        discourse: String,
        /// Setting key.
        setting: String,
    },

    /// List all site settings.
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
