use anyhow::{Context, Result, anyhow};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};
use dsc::config::{Config, DiscourseConfig, find_discourse, load_config, save_config};
use dsc::discourse::{CategoryInfo, DiscourseClient, TopicSummary};
use dsc::utils::{ensure_dir, read_markdown, resolve_topic_path, slugify, write_markdown};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::thread;

#[derive(Parser)]
#[command(name = "dsc")]
#[command(about = "Discourse CLI", long_about = None)]
struct Cli {
    #[arg(long, short = 'c', default_value = "dsc.toml")]
    config: PathBuf,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    List {
        #[arg(long, short = 'f', value_enum, default_value = "plaintext")]
        format: OutputFormat,
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
        #[arg(long, short = 'C')]
        concurrent: bool,
        #[arg(long, short = 'm')]
        max: Option<usize>,
        #[arg(long, short = 'p')]
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
    Completions {
        #[arg(value_enum)]
        shell: CompletionShell,
        #[arg(long, short = 'd')]
        dir: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum EmojiCommand {
    Add {
        emoji_path: PathBuf,
        emoji_name: String,
        discourse_name: Option<String>,
        #[arg(long, short = 'd')]
        discourse: Option<String>,
    },
}

#[derive(Subcommand)]
enum TopicCommand {
    Pull {
        topic_id: u64,
        local_path: Option<PathBuf>,
        #[arg(long, short = 'd')]
        discourse: Option<String>,
    },
    Push {
        local_path: PathBuf,
        topic_id: u64,
        #[arg(long, short = 'd')]
        discourse: Option<String>,
    },
    Sync {
        topic_id: u64,
        local_path: PathBuf,
        #[arg(long, short = 'd')]
        discourse: Option<String>,
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum CategoryCommand {
    List {
        discourse_name: Option<String>,
        #[arg(long, short = 'd')]
        discourse: Option<String>,
    },
    Copy {
        #[arg(long, short = 'd', required = true)]
        discourse: String,
        category_id: u64,
    },
    Pull {
        category_id: u64,
        local_path: Option<PathBuf>,
        #[arg(long, short = 'd')]
        discourse: Option<String>,
    },
    Push {
        local_path: PathBuf,
        category_id: u64,
        #[arg(long, short = 'd')]
        discourse: Option<String>,
    },
}

#[derive(Subcommand)]
enum GroupCommand {
    List {
        #[arg(long, short = 'd', required = true)]
        discourse: String,
    },
    Info {
        #[arg(long, short = 'd', required = true)]
        discourse: String,
        #[arg(long, short = 'g', required = true)]
        group: u64,
    },
    Copy {
        #[arg(long, short = 'd', required = true)]
        discourse: String,
        #[arg(long, short = 't')]
        target: Option<String>,
        #[arg(long, short = 'g', required = true)]
        group: u64,
    },
}

#[derive(Subcommand)]
enum BackupCommand {
    Create {
        #[arg(long, short = 'd', required = true)]
        discourse: String,
    },
    List {
        #[arg(long, short = 'd', required = true)]
        discourse: String,
    },
    Restore {
        #[arg(long, short = 'd', required = true)]
        discourse: String,
        backup_path: String,
    },
}

#[derive(ValueEnum, Clone, Copy)]
enum CompletionShell {
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
enum OutputFormat {
    Plaintext,
    Markdown,
    MarkdownTable,
    Json,
    Yaml,
    Csv,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = load_config(&cli.config)?;

    match cli.command {
        Commands::List { format } => list_discourses(&config, format)?,
        Commands::Add { names, interactive } => {
            add_discourses(&mut config, &names, interactive)?;
            save_config(&cli.config, &config)?;
        }
        Commands::Import { path } => {
            import_discourses(&mut config, path.as_deref())?;
            save_config(&cli.config, &config)?;
        }
        Commands::Update {
            name,
            concurrent,
            max,
            post_changelog,
        } => {
            if name != "all" && (concurrent || max.is_some()) {
                return Err(anyhow!(
                    "--concurrent/--max only apply to 'dsc update all'"
                ));
            }
            if name == "all" {
                if max.is_some() && !concurrent {
                    return Err(anyhow!("--max requires --concurrent"));
                }
                update_all(&config, concurrent, max, post_changelog)?;
            } else {
                update_one(&config, &name, post_changelog)?;
            }
        }
        Commands::Emoji { command } => match command {
            EmojiCommand::Add {
                emoji_path,
                emoji_name,
                discourse_name,
                discourse,
            } => {
                let name = merge_discourse_name(discourse_name.as_deref(), discourse.as_deref());
                add_emoji(&config, name, &emoji_path, &emoji_name)?;
            }
        },
        Commands::Topic { command } => match command {
            TopicCommand::Pull {
                topic_id,
                local_path,
                discourse,
            } => topic_pull(
                &config,
                topic_id,
                local_path.as_deref(),
                discourse.as_deref(),
            )?,
            TopicCommand::Push {
                local_path,
                topic_id,
                discourse,
            } => topic_push(&config, topic_id, &local_path, discourse.as_deref())?,
            TopicCommand::Sync {
                topic_id,
                local_path,
                discourse,
                yes,
            } => topic_sync(&config, topic_id, &local_path, discourse.as_deref(), yes)?,
        },
        Commands::Category { command } => match command {
            CategoryCommand::List {
                discourse_name,
                discourse,
            } => category_list(&config, discourse_name.as_deref(), discourse.as_deref())?,
            CategoryCommand::Copy {
                discourse,
                category_id,
            } => category_copy(&config, &discourse, category_id)?,
            CategoryCommand::Pull {
                category_id,
                local_path,
                discourse,
            } => category_pull(
                &config,
                category_id,
                local_path.as_deref(),
                discourse.as_deref(),
            )?,
            CategoryCommand::Push {
                local_path,
                category_id,
                discourse,
            } => category_push(&config, category_id, &local_path, discourse.as_deref())?,
        },
        Commands::Group { command } => match command {
            GroupCommand::List { discourse } => group_list(&config, &discourse)?,
            GroupCommand::Info { discourse, group } => {
                group_info(&config, &discourse, group)?
            }
            GroupCommand::Copy {
                discourse,
                target,
                group,
            } => group_copy(&config, &discourse, target.as_deref(), group)?,
        },
        Commands::Backup { command } => match command {
            BackupCommand::Create { discourse } => backup_create(&config, &discourse)?,
            BackupCommand::List { discourse } => backup_list(&config, &discourse)?,
            BackupCommand::Restore {
                discourse,
                backup_path,
            } => backup_restore(&config, &discourse, &backup_path)?,
        },
        Commands::Completions { shell, dir } => {
            write_completions(shell, dir.as_deref())?;
        }
    }

    Ok(())
}

fn write_completions(shell: CompletionShell, dir: Option<&Path>) -> Result<()> {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    match dir {
        Some(dir) => {
            ensure_dir(dir)?;
            let filename = match shell {
                CompletionShell::Bash => "dsc.bash",
                CompletionShell::Zsh => "dsc.zsh",
                CompletionShell::Fish => "dsc.fish",
            };
            let path = dir.join(filename);
            let mut file =
                fs::File::create(&path).with_context(|| format!("creating {}", path.display()))?;
            let generator: Shell = shell.into();
            generate(generator, &mut cmd, name, &mut file);
            println!("{}", path.display());
        }
        None => {
            let mut stdout = io::stdout();
            let generator: Shell = shell.into();
            generate(generator, &mut cmd, name, &mut stdout);
        }
    }
    Ok(())
}

fn list_discourses(config: &Config, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Plaintext => {
            for d in &config.discourse {
                println!("{} - {}", d.name, d.baseurl);
            }
        }
        OutputFormat::Markdown => {
            for d in &config.discourse {
                println!("- {} ({})", d.name, d.baseurl);
            }
        }
        OutputFormat::MarkdownTable => {
            println!("| Name | Base URL |");
            println!("| --- | --- |");
            for d in &config.discourse {
                println!("| {} | {} |", d.name, d.baseurl);
            }
        }
        OutputFormat::Json => {
            let raw = serde_json::to_string_pretty(&config.discourse)?;
            println!("{}", raw);
        }
        OutputFormat::Yaml => {
            let raw = serde_yaml::to_string(&config.discourse)?;
            println!("{}", raw);
        }
        OutputFormat::Csv => {
            let mut writer = csv::Writer::from_writer(io::stdout());
            writer.write_record(["name", "baseurl", "tags"])?;
            for d in &config.discourse {
                let tags = d.tags.as_ref().map(|t| t.join(";")).unwrap_or_default();
                writer.write_record([d.name.as_str(), d.baseurl.as_str(), &tags])?;
            }
            writer.flush()?;
        }
    }
    Ok(())
}

fn add_discourses(config: &mut Config, names: &str, interactive: bool) -> Result<()> {
    let entries = names
        .split(',')
        .map(|name| name.trim())
        .filter(|name| !name.is_empty());
    for name in entries {
        if config.discourse.iter().any(|d| d.name == name) {
            continue;
        }
        let mut entry = DiscourseConfig {
            name: name.to_string(),
            ..DiscourseConfig::default()
        };
        if interactive {
            entry.baseurl = prompt("Base URL")?;
            entry.apikey = prompt_optional("API key")?;
            entry.api_username = prompt_optional("API username")?;
            let tags = prompt_optional("Tags (comma-separated)")?;
            entry.tags = tags.map(|t| {
                t.split(',')
                    .map(|tag| tag.trim().to_string())
                    .filter(|tag| !tag.is_empty())
                    .collect::<Vec<_>>()
            });
        }
        config.discourse.push(entry);
    }
    Ok(())
}

fn import_discourses(config: &mut Config, path: Option<&Path>) -> Result<()> {
    let mut raw = String::new();
    if let Some(path) = path {
        if path == Path::new("-") {
            io::stdin().read_to_string(&mut raw)?;
        } else {
            raw =
                fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        }
    } else {
        io::stdin().read_to_string(&mut raw)?;
    }
    import_from_string(config, &raw, path)?;
    Ok(())
}

fn import_from_string(config: &mut Config, raw: &str, path: Option<&Path>) -> Result<()> {
    let is_csv = path.and_then(|p| p.extension().and_then(|s| s.to_str())) == Some("csv")
        || looks_like_csv(raw);
    if is_csv {
        import_csv(config, raw)?;
    } else {
        import_text(config, raw)?;
    }
    Ok(())
}

fn import_text(config: &mut Config, raw: &str) -> Result<()> {
    for line in raw.lines() {
        let url = line.trim();
        if url.is_empty() {
            continue;
        }
        let name = fetch_name_from_url(url).unwrap_or_else(|_| slugify(url));
        config.discourse.push(DiscourseConfig {
            name,
            baseurl: url.to_string(),
            ..DiscourseConfig::default()
        });
    }
    Ok(())
}

fn import_csv(config: &mut Config, raw: &str) -> Result<()> {
    let mut reader = csv::Reader::from_reader(raw.as_bytes());
    for result in reader.records() {
        let record = result?;
        let name = record.get(0).unwrap_or("").trim();
        let url = record.get(1).unwrap_or("").trim();
        if url.is_empty() {
            continue;
        }
        let name = if name.is_empty() {
            fetch_name_from_url(url).unwrap_or_else(|_| slugify(url))
        } else {
            name.to_string()
        };
        let tags = record
            .get(2)
            .map(parse_tags)
            .filter(|t| !t.is_empty());
        config.discourse.push(DiscourseConfig {
            name,
            baseurl: url.to_string(),
            tags,
            ..DiscourseConfig::default()
        });
    }
    Ok(())
}

fn looks_like_csv(raw: &str) -> bool {
    let first = raw.lines().find(|line| !line.trim().is_empty());
    let Some(first) = first else { return false };
    let lower = first.to_ascii_lowercase();
    lower.contains("name") && lower.contains("url") && first.contains(',')
}

fn parse_tags(raw: &str) -> Vec<String> {
    raw.split(|ch| ch == ';' || ch == ',')
        .map(|tag| tag.trim().to_string())
        .filter(|tag| !tag.is_empty())
        .collect()
}

fn fetch_name_from_url(baseurl: &str) -> Result<String> {
    let temp = DiscourseConfig {
        name: "temp".to_string(),
        baseurl: baseurl.to_string(),
        ..DiscourseConfig::default()
    };
    let client = DiscourseClient::new(&temp)?;
    let title = client.fetch_site_title()?;
    Ok(slugify(&title))
}

fn update_one(config: &Config, name: &str, post_changelog: bool) -> Result<()> {
    let discourse = find_discourse(config, name).ok_or_else(|| anyhow!("unknown discourse"))?;
    let metadata = run_update(discourse)?;
    if post_changelog {
        post_changelog_update(discourse, Some(&metadata))?;
    }
    Ok(())
}

fn update_all(
    config: &Config,
    concurrent: bool,
    max: Option<usize>,
    post_changelog: bool,
) -> Result<()> {
    if !concurrent {
        for discourse in &config.discourse {
            let metadata = run_update(discourse)?;
            if post_changelog {
                post_changelog_update(discourse, Some(&metadata))?;
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
                post_changelog_update(&discourse, Some(&metadata))?;
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
}

fn run_update(discourse: &DiscourseConfig) -> Result<UpdateMetadata> {
    let client = DiscourseClient::new(discourse)?;
    let before_version = client.fetch_version().unwrap_or(None);
    let target = discourse
        .ssh_host
        .clone()
        .unwrap_or_else(|| discourse.name.clone());
    let update_cmd = std::env::var("DSC_SSH_UPDATE_CMD")
        .unwrap_or_else(|_| "cd /var/discourse && ./launcher rebuild app".to_string());
    let cleanup_cmd = std::env::var("DSC_SSH_CLEANUP_CMD")
        .unwrap_or_else(|_| "cd /var/discourse && ./launcher cleanup".to_string());
    run_ssh_command(&target, &update_cmd)?;
    let after_version = client.fetch_version().unwrap_or(None);
    let cleanup = run_ssh_command(&target, &cleanup_cmd)?;
    let reclaimed_space = parse_reclaimed_space(&cleanup);
    Ok(UpdateMetadata {
        before_version,
        after_version,
        reclaimed_space,
    })
}

fn run_ssh_command(target: &str, command: &str) -> Result<String> {
    let output = std::process::Command::new("ssh")
        .arg("-o")
        .arg("BatchMode=yes")
        .arg(target)
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

fn parse_reclaimed_space(output: &str) -> Option<String> {
    output
        .lines()
        .find_map(|line| line.split("Total reclaimed space:").nth(1))
        .map(|value| value.trim().to_string())
}

fn post_changelog_update(
    discourse: &DiscourseConfig,
    metadata: Option<&UpdateMetadata>,
) -> Result<()> {
    let topic_id = discourse
        .changelog_topic_id
        .ok_or_else(|| anyhow!("changelog_topic_id is required to post updates"))?;
    let client = DiscourseClient::new(discourse)?;
    let version = metadata
        .and_then(|meta| meta.after_version.clone().or(meta.before_version.clone()))
        .unwrap_or_else(|| "unknown".to_string());
    let reclaimed = metadata
        .and_then(|meta| meta.reclaimed_space.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let mut body = vec![
        "- [x] Ubuntu OS updated".to_string(),
        "- [x] Server rebooted".to_string(),
        format!("- [x] Updated Discourse to version {}", version),
        format!(
            "- [x] `./launcher cleanup` Total reclaimed space: {}",
            reclaimed
        ),
    ];
    if let Ok(marker) = std::env::var("DSC_TEST_MARKER") {
        body.push(format!("- Run-ID: {}", marker));
    }
    let payload = body.join("\n");
    client.create_post(topic_id, &payload)?;
    Ok(())
}

fn add_emoji(
    config: &Config,
    discourse_name: Option<&str>,
    emoji_path: &Path,
    emoji_name: &str,
) -> Result<()> {
    let discourse = select_discourse(config, discourse_name)?;
    let client = DiscourseClient::new(discourse)?;
    client.upload_emoji(emoji_path, emoji_name)?;
    Ok(())
}

fn topic_pull(
    config: &Config,
    topic_id: u64,
    local_path: Option<&Path>,
    discourse_name: Option<&str>,
) -> Result<()> {
    let discourse = select_discourse(config, discourse_name)?;
    let client = DiscourseClient::new(discourse)?;
    let topic = client.fetch_topic(topic_id, true)?;
    let raw = topic
        .post_stream
        .posts
        .get(0)
        .and_then(|p| p.raw.clone())
        .ok_or_else(|| anyhow!("topic has no raw content"))?;
    let target = resolve_topic_path(local_path, &topic.title, &std::env::current_dir()?)?;
    write_markdown(&target, &raw)?;
    println!("{}", target.display());
    Ok(())
}

fn topic_push(
    config: &Config,
    topic_id: u64,
    local_path: &Path,
    discourse_name: Option<&str>,
) -> Result<()> {
    let discourse = select_discourse(config, discourse_name)?;
    let client = DiscourseClient::new(discourse)?;
    let topic = client.fetch_topic(topic_id, true)?;
    let post = topic
        .post_stream
        .posts
        .get(0)
        .ok_or_else(|| anyhow!("topic has no posts"))?;
    let raw = read_markdown(local_path)?;
    client.update_post(post.id, &raw)?;
    Ok(())
}

fn topic_sync(
    config: &Config,
    topic_id: u64,
    local_path: &Path,
    discourse_name: Option<&str>,
    assume_yes: bool,
) -> Result<()> {
    let discourse = select_discourse(config, discourse_name)?;
    let client = DiscourseClient::new(discourse)?;
    let topic = client.fetch_topic(topic_id, true)?;
    let post = topic
        .post_stream
        .posts
        .get(0)
        .ok_or_else(|| anyhow!("topic has no posts"))?;
    let local_meta =
        fs::metadata(local_path).with_context(|| format!("reading {}", local_path.display()))?;
    let local_mtime = local_meta.modified()?;

    let remote_ts = post
        .updated_at
        .as_deref()
        .or(post.created_at.as_deref())
        .ok_or_else(|| anyhow!("missing remote timestamps"))?;
    let remote_time = chrono::DateTime::parse_from_rfc3339(remote_ts)
        .context("parsing remote timestamp")?
        .with_timezone(&chrono::Utc);

    println!(
        "Local file:  {}",
        chrono::DateTime::<chrono::Utc>::from(local_mtime)
    );
    println!("Remote post: {}", remote_time);

    let pull = remote_time > chrono::DateTime::<chrono::Utc>::from(local_mtime);
    if !assume_yes && !confirm_sync(pull)? {
        return Ok(());
    }

    if pull {
        let raw = post
            .raw
            .clone()
            .ok_or_else(|| anyhow!("missing raw content"))?;
        write_markdown(local_path, &raw)?;
    } else {
        let raw = read_markdown(local_path)?;
        client.update_post(post.id, &raw)?;
    }

    Ok(())
}

fn confirm_sync(pull: bool) -> Result<bool> {
    let action = if pull {
        "pull from Discourse"
    } else {
        "push to Discourse"
    };
    print!("Proceed to {}? [y/N]: ", action);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(input.trim(), "y" | "Y" | "yes" | "YES"))
}

fn category_list(
    config: &Config,
    discourse_name: Option<&str>,
    discourse_flag: Option<&str>,
) -> Result<()> {
    let discourse = select_discourse(config, merge_discourse_name(discourse_name, discourse_flag))?;
    let client = DiscourseClient::new(discourse)?;
    let categories = client.fetch_categories()?;
    let mut flat = Vec::new();
    for category in categories {
        flatten_categories(&category, &mut flat);
    }
    let mut seen = std::collections::HashSet::new();
    for category in flat {
        if let Some(id) = category.id {
            if !seen.insert(id) {
                continue;
            }
        }
        let id = category.id.unwrap_or_default();
        println!("{} - {}", id, category.name);
    }
    Ok(())
}

fn flatten_categories(category: &CategoryInfo, out: &mut Vec<CategoryInfo>) {
    out.push(category.clone());
    for sub in &category.subcategory_list {
        flatten_categories(sub, out);
    }
}

fn category_copy(
    config: &Config,
    discourse_name: &str,
    category_id: u64,
) -> Result<()> {
    let discourse = find_discourse(config, discourse_name)
        .ok_or_else(|| anyhow!("unknown discourse {}", discourse_name))?;
    let client = DiscourseClient::new(discourse)?;
    let categories = client.fetch_categories()?;
    let category = categories
        .into_iter()
        .find(|cat| cat.id == Some(category_id))
        .ok_or_else(|| anyhow!("category not found"))?;
    let mut copied = category.clone();
    copied.name = format!("Copy of {}", category.name);
    copied.slug = format!("{}-copy", category.slug);
    copied.id = None;
    let new_id = client.create_category(&copied)?;
    println!("{}", new_id);
    Ok(())
}

fn group_list(config: &Config, discourse_name: &str) -> Result<()> {
    let discourse = find_discourse(config, discourse_name)
        .ok_or_else(|| anyhow!("unknown discourse {}", discourse_name))?;
    let client = DiscourseClient::new(discourse)?;
    let groups = client.fetch_groups()?;
    for group in groups {
        let full_name = group.full_name.unwrap_or_else(|| "-".to_string());
        println!("{} - {} ({})", group.id, group.name, full_name);
    }
    Ok(())
}

fn group_info(config: &Config, discourse_name: &str, group_id: u64) -> Result<()> {
    let discourse = find_discourse(config, discourse_name)
        .ok_or_else(|| anyhow!("unknown discourse {}", discourse_name))?;
    let client = DiscourseClient::new(discourse)?;
    let groups = client.fetch_groups()?;
    let group_summary = groups
        .into_iter()
        .find(|item| item.id == group_id)
        .ok_or_else(|| anyhow!("group not found"))?;
    let group = client.fetch_group_detail(group_summary.id, Some(&group_summary.name))?;
    let raw = serde_json::to_string_pretty(&group)?;
    println!("{}", raw);
    Ok(())
}

fn group_copy(config: &Config, source: &str, target: Option<&str>, group_id: u64) -> Result<()> {
    let source_discourse =
        find_discourse(config, source).ok_or_else(|| anyhow!("unknown discourse {}", source))?;
    let target_discourse_name = target.unwrap_or(source);
    let target_discourse = find_discourse(config, target_discourse_name)
        .ok_or_else(|| anyhow!("unknown discourse {}", target_discourse_name))?;

    let source_client = DiscourseClient::new(source_discourse)?;
    let groups = source_client.fetch_groups()?;
    let group_summary = groups
        .into_iter()
        .find(|item| item.id == group_id)
        .ok_or_else(|| anyhow!("group not found"))?;
    let mut group =
        source_client.fetch_group_detail(group_summary.id, Some(&group_summary.name))?;
    group.name = format!("{}-copy", slugify(&group.name));
    if let Some(full_name) = group.full_name.clone() {
        group.full_name = Some(format!("Copy of {}", full_name));
    }

    let target_client = DiscourseClient::new(target_discourse)?;
    let new_id = target_client.create_group(&group)?;
    println!("{}", new_id);
    Ok(())
}

fn backup_create(config: &Config, discourse_name: &str) -> Result<()> {
    let discourse = find_discourse(config, discourse_name)
        .ok_or_else(|| anyhow!("unknown discourse {}", discourse_name))?;
    let client = DiscourseClient::new(discourse)?;
    client.create_backup()?;
    Ok(())
}

fn backup_list(config: &Config, discourse_name: &str) -> Result<()> {
    let discourse = find_discourse(config, discourse_name)
        .ok_or_else(|| anyhow!("unknown discourse {}", discourse_name))?;
    let client = DiscourseClient::new(discourse)?;
    let backups = client.list_backups()?;
    let raw = serde_json::to_string_pretty(&backups)?;
    println!("{}", raw);
    Ok(())
}

fn backup_restore(config: &Config, discourse_name: &str, backup_path: &str) -> Result<()> {
    let discourse = find_discourse(config, discourse_name)
        .ok_or_else(|| anyhow!("unknown discourse {}", discourse_name))?;
    let client = DiscourseClient::new(discourse)?;
    client.restore_backup(backup_path)?;
    Ok(())
}

fn category_pull(
    config: &Config,
    category_id: u64,
    local_path: Option<&Path>,
    discourse_name: Option<&str>,
) -> Result<()> {
    let discourse = select_discourse(config, discourse_name)?;
    let client = DiscourseClient::new(discourse)?;
    let category = client.fetch_category(category_id)?;
    let dir = match local_path {
        Some(path) => path.to_path_buf(),
        None => {
            let name = category
                .category
                .as_ref()
                .map(|c| c.slug.clone())
                .unwrap_or_else(|| format!("category-{}", category_id));
            std::env::current_dir()?.join(name)
        }
    };
    ensure_dir(&dir)?;
    for topic in category.topic_list.topics {
        let topic_detail = client.fetch_topic(topic.id, true)?;
        let raw = topic_detail
            .post_stream
            .posts
            .get(0)
            .and_then(|p| p.raw.clone())
            .unwrap_or_default();
        let filename = format!("{}.md", slugify(&topic.title));
        write_markdown(&dir.join(filename), &raw)?;
    }
    println!("{}", dir.display());
    Ok(())
}

fn category_push(
    config: &Config,
    category_id: u64,
    local_path: &Path,
    discourse_name: Option<&str>,
) -> Result<()> {
    let discourse = select_discourse(config, discourse_name)?;
    let client = DiscourseClient::new(discourse)?;
    let existing = client.fetch_category(category_id)?;
    let mut topics = existing.topic_list.topics;
    let entries =
        fs::read_dir(local_path).with_context(|| format!("reading {}", local_path.display()))?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let raw = read_markdown(&path)?;
        let title = extract_title(&raw)
            .unwrap_or_else(|| path.file_stem().unwrap().to_string_lossy().to_string());
        if let Some(topic) = find_topic_match(&topics, &title, &path) {
            let detail = client.fetch_topic(topic.id, true)?;
            let post = detail
                .post_stream
                .posts
                .get(0)
                .ok_or_else(|| anyhow!("topic has no posts"))?;
            client.update_post(post.id, &raw)?;
        } else {
            let topic_id = client.create_topic(category_id, &title, &raw)?;
            topics.push(TopicSummary {
                id: topic_id,
                title: title.clone(),
                slug: slugify(&title),
            });
        }
    }
    Ok(())
}

fn extract_title(raw: &str) -> Option<String> {
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(title) = line.strip_prefix("# ") {
            return Some(title.trim().to_string());
        }
        break;
    }
    None
}

fn find_topic_match<'a>(
    topics: &'a [TopicSummary],
    title: &str,
    path: &Path,
) -> Option<&'a TopicSummary> {
    let slug = slugify(title);
    topics.iter().find(|topic| {
        topic.slug == slug
            || topic.title.eq_ignore_ascii_case(title)
            || path
                .file_stem()
                .map(|s| s.to_string_lossy().eq_ignore_ascii_case(&topic.slug))
                .unwrap_or(false)
    })
}

fn select_discourse<'a>(
    config: &'a Config,
    discourse_name: Option<&str>,
) -> Result<&'a DiscourseConfig> {
    if let Some(name) = discourse_name {
        return find_discourse(config, name).ok_or_else(|| anyhow!("unknown discourse {}", name));
    }
    if config.discourse.len() == 1 {
        return Ok(&config.discourse[0]);
    }
    Err(anyhow!("multiple discourses configured; use --discourse"))
}

fn merge_discourse_name<'a>(positional: Option<&'a str>, flag: Option<&'a str>) -> Option<&'a str> {
    flag.or(positional)
}

fn prompt(label: &str) -> Result<String> {
    print!("{}: ", label);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

fn prompt_optional(label: &str) -> Result<Option<String>> {
    let value = prompt(label)?;
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}
