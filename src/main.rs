use anyhow::{anyhow, Result};
use clap::Parser;
use dsc::cli::{
    BackupCommand, CategoryCommand, Cli, Commands, EmojiCommand, GroupCommand, ListCommand,
    PaletteCommand, SettingCommand, TopicCommand,
};
use dsc::commands;
use dsc::config::{load_config, save_config};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = load_config(&cli.config)?;

    match cli.command {
        Commands::List {
            format,
            tags,
            command,
        } => match command {
            Some(ListCommand::Tidy) => {
                if tags.is_some() {
                    return Err(anyhow!("--tags is not supported with 'dsc list tidy'"));
                }
                commands::list::list_tidy(&cli.config, &mut config)?;
            }
            None => commands::list::list_discourses(&config, format, tags.as_deref())?,
        },
        Commands::Add { names, interactive } => {
            commands::add::add_discourses(&mut config, &names, interactive)?;
            save_config(&cli.config, &config)?;
        }
        Commands::Import { path } => {
            commands::import::import_discourses(&mut config, path.as_deref())?;
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
                commands::update::update_all(&config, concurrent, max, post_changelog)?;
            } else {
                commands::update::update_one(&config, &name, post_changelog)?;
            }
        }
        Commands::Emoji { command } => match command {
            EmojiCommand::Add {
                discourse,
                emoji_path,
                emoji_name,
            } => {
                commands::emoji::add_emoji(&config, &discourse, &emoji_path, emoji_name.as_deref())?
            }

            EmojiCommand::List { discourse } => {
                commands::emoji::list_emojis(&config, &discourse)?;
            }
        },
        Commands::Topic { command } => match command {
            TopicCommand::Pull {
                discourse,
                topic_id,
                local_path,
            } => commands::topic::topic_pull(&config, &discourse, topic_id, local_path.as_deref())?,
            TopicCommand::Push {
                discourse,
                local_path,
                topic_id,
            } => commands::topic::topic_push(&config, &discourse, topic_id, &local_path)?,
            TopicCommand::Sync {
                discourse,
                topic_id,
                local_path,
                yes,
            } => commands::topic::topic_sync(&config, &discourse, topic_id, &local_path, yes)?,
        },
        Commands::Category { command } => match command {
            CategoryCommand::List { discourse, tree } => {
                commands::category::category_list(&config, &discourse, tree)?;
            }
            CategoryCommand::Copy {
                discourse,
                category_id,
            } => commands::category::category_copy(&config, &discourse, category_id)?,
            CategoryCommand::Pull {
                discourse,
                category_id,
                local_path,
            } => commands::category::category_pull(
                &config,
                &discourse,
                category_id,
                local_path.as_deref(),
            )?,
            CategoryCommand::Push {
                discourse,
                local_path,
                category_id,
            } => commands::category::category_push(&config, &discourse, category_id, &local_path)?,
        },
        Commands::Group { command } => match command {
            GroupCommand::List { discourse } => commands::group::group_list(&config, &discourse)?,
            GroupCommand::Info {
                discourse,
                group,
                format,
            } => {
                commands::group::group_info(&config, &discourse, group, format)?;
            }
            GroupCommand::Copy {
                discourse,
                target,
                group,
            } => commands::group::group_copy(&config, &discourse, target.as_deref(), group)?,
        },
        Commands::Backup { command } => match command {
            BackupCommand::Create { discourse } => {
                commands::backup::backup_create(&config, &discourse)?;
            }
            BackupCommand::List { discourse, format } => {
                commands::backup::backup_list(&config, &discourse, format)?;
            }
            BackupCommand::Restore {
                discourse,
                backup_path,
            } => commands::backup::backup_restore(&config, &discourse, &backup_path)?,
        },
        Commands::Palette { command } => match command {
            PaletteCommand::List { discourse } => {
                commands::palette::palette_list(&config, &discourse)?;
            }
            PaletteCommand::Pull {
                discourse,
                palette_id,
                local_path,
            } => commands::palette::palette_pull(
                &config,
                &discourse,
                palette_id,
                local_path.as_deref(),
            )?,
            PaletteCommand::Push {
                discourse,
                local_path,
                palette_id,
            } => commands::palette::palette_push(&config, &discourse, &local_path, palette_id)?,
        },
        Commands::Setting { command } => match command {
            SettingCommand::Set {
                setting,
                value,
                tags,
            } => commands::setting::set_site_setting(&config, &setting, &value, tags.as_deref())?,
        },
        Commands::Completions { shell, dir } => {
            commands::completions::write_completions(shell, dir.as_deref())?;
        }
    }

    Ok(())
}
