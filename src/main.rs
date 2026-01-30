use anyhow::{anyhow, Context, Result};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};
use dsc::config::{find_discourse, load_config, save_config, Config, DiscourseConfig};
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
        discourse: String,
        emoji_path: PathBuf,
        emoji_name: String,
    },
}

#[derive(Subcommand)]
enum TopicCommand {
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
enum CategoryCommand {
    List {
        discourse: String,
        #[arg(long)]
        tree: bool,
    },
    Copy {
        discourse: String,
        category_id: u64,
    },
    Pull {
        discourse: String,
        category_id: u64,
        local_path: Option<PathBuf>,
    },
    Push {
        discourse: String,
        local_path: PathBuf,
        category_id: u64,
    },
}

#[derive(Subcommand)]
enum GroupCommand {
    List {
        discourse: String,
    },
    Info {
        discourse: String,
        group: u64,
    },
    Copy {
        discourse: String,
        #[arg(long, short = 't')]
        target: Option<String>,
        group: u64,
    },
}

#[derive(Subcommand)]
enum BackupCommand {
    Create {
        discourse: String,
    },
    List {
        discourse: String,
    },
    Restore {
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
                return Err(anyhow!("--concurrent/--max only apply to 'dsc update all'"));
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
                discourse,
                emoji_path,
                emoji_name,
            } => add_emoji(&config, &discourse, &emoji_path, &emoji_name)?,
        },
        Commands::Topic { command } => match command {
            TopicCommand::Pull {
                discourse,
                topic_id,
                local_path,
            } => topic_pull(&config, &discourse, topic_id, local_path.as_deref())?,
            TopicCommand::Push {
                discourse,
                local_path,
                topic_id,
            } => topic_push(&config, &discourse, topic_id, &local_path)?,
            TopicCommand::Sync {
                discourse,
                topic_id,
                local_path,
                yes,
            } => topic_sync(&config, &discourse, topic_id, &local_path, yes)?,
        },
        Commands::Category { command } => match command {
            CategoryCommand::List { discourse, tree } => category_list(&config, &discourse, tree)?,
            CategoryCommand::Copy {
                discourse,
                category_id,
            } => category_copy(&config, &discourse, category_id)?,
            CategoryCommand::Pull {
                discourse,
                category_id,
                local_path,
            } => category_pull(&config, &discourse, category_id, local_path.as_deref())?,
            CategoryCommand::Push {
                discourse,
                local_path,
                category_id,
            } => category_push(&config, &discourse, category_id, &local_path)?,
        },
        Commands::Group { command } => match command {
            GroupCommand::List { discourse } => group_list(&config, &discourse)?,
            GroupCommand::Info { discourse, group } => group_info(&config, &discourse, group)?,
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

        if !interactive {
            entry.apikey = Some("".to_string());
            entry.api_username = Some("".to_string());
            entry.changelog_path = Some("".to_string());
            entry.tags = Some(Vec::new());
            entry.changelog_topic_id = Some(0);
            entry.ssh_host = Some("".to_string());
        }
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
        let tags = record.get(2).map(parse_tags).filter(|t| !t.is_empty());
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
    if concurrent {
        return Err(anyhow!(
            "--concurrent is disabled for 'dsc update all' because it stops on first failure"
        ));
    }
    if !concurrent {
        let date = chrono::Utc::now().format("%Y.%m.%d");
        let log_path = format!("{}-dsc-update-all.log", date);
        println!("==> Logging update progress to {}", log_path);
        let mut log_file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .with_context(|| format!("opening update log at {}", log_path))?;
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
                if let Err(err) = post_changelog_update(discourse, Some(&metadata)) {
                    writeln!(
                        log_file,
                        "{} failed {} (changelog): {}",
                        chrono::Utc::now().to_rfc3339(),
                        discourse.name,
                        err
                    )?;
                    return Err(err);
                }
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

    match run_ssh_command(&target, &os_update_cmd) {
        Ok(_) => {
            os_updated = true;
            if run_ssh_command(&target, &reboot_cmd).is_ok() {
                server_rebooted = true;
                if std::env::var("DSC_SSH_OS_UPDATE_CMD").unwrap_or_default()
                    != "echo OS packages updated"
                {
                    std::thread::sleep(std::time::Duration::from_secs(30));
                    let mut attempts = 0;
                    let max_attempts = 12;
                    while attempts < max_attempts {
                        match std::process::Command::new("ssh")
                            .arg("-o")
                            .arg("BatchMode=yes")
                            .arg("-o")
                            .arg("ConnectTimeout=10")
                            .arg(&target)
                            .arg("echo 'server is up'")
                            .output()
                        {
                            Ok(output) if output.status.success() => {
                                break;
                            }
                            _ => {
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
        }
        Err(_) => {}
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
    let payload = body.join("\n");
    let post_id = client.create_post(topic_id, &payload)?;
    if test_marker.is_some() {
        println!("DSC_TEST_POST_ID={}", post_id);
    }
    Ok(())
}

fn add_emoji(
    config: &Config,
    discourse_name: &str,
    emoji_path: &Path,
    emoji_name: &str,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    let client = DiscourseClient::new(discourse)?;
    client.upload_emoji(emoji_path, emoji_name)?;
    Ok(())
}

fn topic_pull(
    config: &Config,
    discourse_name: &str,
    topic_id: u64,
    local_path: Option<&Path>,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
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
    discourse_name: &str,
    topic_id: u64,
    local_path: &Path,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
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
    discourse_name: &str,
    topic_id: u64,
    local_path: &Path,
    assume_yes: bool,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
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

fn category_list(config: &Config, discourse_name: &str, tree: bool) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    let client = DiscourseClient::new(discourse)?;
    let categories = client.fetch_categories()?;
    let mut flat = Vec::new();
    for category in categories {
        flatten_categories(&category, &mut flat);
    }
    if tree {
        print_category_tree(&flat);
    } else {
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
    }
    Ok(())
}

fn flatten_categories(category: &CategoryInfo, out: &mut Vec<CategoryInfo>) {
    out.push(category.clone());
    for sub in &category.subcategory_list {
        flatten_categories(sub, out);
    }
}

fn print_category_tree(categories: &[CategoryInfo]) {
    let mut ordered_ids = Vec::new();
    let mut map = std::collections::HashMap::new();
    for category in categories {
        if let Some(id) = category.id {
            if !map.contains_key(&id) {
                ordered_ids.push(id);
                map.insert(id, category.clone());
            }
        }
    }

    let mut children: std::collections::HashMap<u64, Vec<u64>> = std::collections::HashMap::new();
    for category in map.values() {
        if let (Some(id), Some(parent_id)) = (category.id, category.parent_category_id) {
            if map.contains_key(&parent_id) {
                let entry = children.entry(parent_id).or_default();
                if !entry.contains(&id) {
                    entry.push(id);
                }
            }
        }
    }

    let mut roots = Vec::new();
    for id in &ordered_ids {
        if let Some(category) = map.get(id) {
            match category.parent_category_id {
                Some(parent_id) if map.contains_key(&parent_id) => {}
                _ => roots.push(*id),
            }
        }
    }

    let mut seen = std::collections::HashSet::new();
    let last_index = roots.len().saturating_sub(1);
    for (idx, id) in roots.into_iter().enumerate() {
        let is_last = idx == last_index;
        print_category_node(&map, &children, id, "", is_last, &mut seen);
    }
}

fn print_category_node(
    map: &std::collections::HashMap<u64, CategoryInfo>,
    children: &std::collections::HashMap<u64, Vec<u64>>,
    id: u64,
    prefix: &str,
    is_last: bool,
    seen: &mut std::collections::HashSet<u64>,
) {
    if !seen.insert(id) {
        return;
    }
    if let Some(category) = map.get(&id) {
        let branch = if is_last {
            "└── ".to_string()
        } else {
            "├── ".to_string()
        };
        println!("{}{}{} - {}", prefix, branch, id, category.name);
        if let Some(child_ids) = children.get(&id) {
            let new_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };
            let last_index = child_ids.len().saturating_sub(1);
            for (idx, child_id) in child_ids.iter().enumerate() {
                let child_last = idx == last_index;
                print_category_node(map, children, *child_id, &new_prefix, child_last, seen);
            }
        }
    }
}

fn category_copy(config: &Config, discourse_name: &str, category_id: u64) -> Result<()> {
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
    discourse_name: &str,
    category_id: u64,
    local_path: Option<&Path>,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
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
    discourse_name: &str,
    category_id: u64,
    local_path: &Path,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
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
    Err(anyhow!("discourse name is required"))
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
