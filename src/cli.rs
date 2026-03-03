use clap::{Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "dsc")]
#[command(about = "Discourse CLI", long_about = None)]
pub struct Cli {
    #[arg(long, short = 'c')]
    pub config: Option<PathBuf>,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(visible_alias = "ls")]
    List {
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: OutputFormat,
        #[arg(long, value_name = "tag1,tag2")]
        tags: Option<String>,
        #[arg(long, short = 'v')]
        verbose: bool,
        #[command(subcommand)]
        command: Option<ListCommand>,
    },
    Add {
        names: String,
        #[arg(long, short = 'i')]
        interactive: bool,
    },
    Import {
        path: Option<PathBuf>,
    },
    Update {
        name: String,
        #[arg(long, short = 'p')]
        parallel: bool,
        #[arg(long, short = 'm')]
        max: Option<usize>,
        #[arg(long, short = 'g')]
        post_changelog: bool,
    },
    Emoji {
        #[command(subcommand)]
        command: EmojiCommand,
    },
    Topic {
        #[command(subcommand)]
        command: TopicCommand,
    },
    Category {
        #[command(subcommand)]
        command: CategoryCommand,
    },
    Group {
        #[command(subcommand)]
        command: GroupCommand,
    },
    Backup {
        #[command(subcommand)]
        command: BackupCommand,
    },
    Palette {
        #[command(subcommand)]
        command: PaletteCommand,
    },
    Plugin {
        #[command(subcommand)]
        command: PluginCommand,
    },
    Theme {
        #[command(subcommand)]
        command: ThemeCommand,
    },
    Setting {
        #[command(subcommand)]
        command: SettingCommand,
    },
    Completions {
        #[arg(value_enum)]
        shell: CompletionShell,
        #[arg(long, short = 'd')]
        dir: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum ListCommand {
    /// Sort discourse entries by name and rewrite config in-place.
    /// Also inserts placeholder values for unset template keys.
    Tidy,
}

#[derive(Subcommand)]
pub enum EmojiCommand {
    Add {
        discourse: String,
        emoji_path: PathBuf,
        emoji_name: Option<String>,
    },

    /// List custom emojis on a Discourse.
    List {
        discourse: String,
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
        #[arg(long, short = 'v')]
        verbose: bool,
        #[arg(long, short = 'i')]
        inline: bool,
    },
}

#[derive(Subcommand)]
pub enum TopicCommand {
    Pull {
        discourse: String,
        topic_id: u64,
        local_path: Option<PathBuf>,
    },
    Push {
        discourse: String,
        local_path: PathBuf,
        topic_id: u64,
    },
    Sync {
        discourse: String,
        topic_id: u64,
        local_path: PathBuf,
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

#[derive(Subcommand)]
pub enum CategoryCommand {
    List {
        discourse: String,
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
        #[arg(long, short = 'v')]
        verbose: bool,
        #[arg(long)]
        tree: bool,
    },
    Copy {
        discourse: String,
        #[arg(long, short = 't')]
        target: Option<String>,
        category: String,
    },
    Pull {
        discourse: String,
        category: String,
        local_path: Option<PathBuf>,
    },
    Push {
        discourse: String,
        local_path: PathBuf,
        category: String,
    },
}

#[derive(Subcommand)]
pub enum GroupCommand {
    List {
        discourse: String,
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
        #[arg(long, short = 'v')]
        verbose: bool,
    },
    Info {
        discourse: String,
        group: u64,
        #[arg(long, short = 'f', value_enum, default_value = "json")]
        format: StructuredFormat,
    },
    Members {
        discourse: String,
        group: u64,
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
    },
    Copy {
        discourse: String,
        #[arg(long, short = 't')]
        target: Option<String>,
        group: u64,
    },
}

#[derive(Subcommand)]
pub enum BackupCommand {
    Create {
        discourse: String,
    },
    List {
        discourse: String,
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: OutputFormat,
        #[arg(long, short = 'v')]
        verbose: bool,
    },
    Restore {
        discourse: String,
        backup_path: String,
    },
}

#[derive(Subcommand)]
pub enum PaletteCommand {
    List {
        discourse: String,
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
        #[arg(long, short = 'v')]
        verbose: bool,
    },
    Pull {
        discourse: String,
        palette_id: u64,
        local_path: Option<PathBuf>,
    },
    Push {
        discourse: String,
        local_path: PathBuf,
        palette_id: Option<u64>,
    },
}

#[derive(Subcommand)]
pub enum PluginCommand {
    List {
        discourse: String,
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
        #[arg(long, short = 'v')]
        verbose: bool,
    },
    Install {
        discourse: String,
        url: String,
    },
    Remove {
        discourse: String,
        name: String,
    },
}

#[derive(Subcommand)]
pub enum ThemeCommand {
    List {
        discourse: String,
        #[arg(long, short = 'f', value_enum, default_value = "text")]
        format: ListFormat,
        #[arg(long, short = 'v')]
        verbose: bool,
    },
    Install {
        discourse: String,
        url: String,
    },
    Remove {
        discourse: String,
        name: String,
    },
}

#[derive(Subcommand)]
pub enum SettingCommand {
    Set {
        setting: String,
        value: String,
        #[arg(long, value_name = "tag1,tag2")]
        tags: Option<String>,
    },
}

#[derive(ValueEnum, Clone, Copy)]
pub enum CompletionShell {
    Bash,
    Zsh,
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
    #[value(alias = "plaintext")]
    Text,
    Markdown,
    MarkdownTable,
    Json,
    #[value(alias = "yml")]
    Yaml,
    Csv,
}

#[derive(ValueEnum, Clone, Copy)]
pub enum ListFormat {
    Text,
    Json,
    #[value(alias = "yml")]
    Yaml,
}

#[derive(ValueEnum, Clone, Copy)]
pub enum StructuredFormat {
    Json,
    #[value(alias = "yml")]
    Yaml,
}
