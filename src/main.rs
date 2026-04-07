use anyhow::{anyhow, Result};
use clap::Parser;
use dsc::cli::*;
use dsc::commands;
use dsc::config::{load_config, resolve_default_config_path, save_config};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_path = cli.config.unwrap_or_else(resolve_default_config_path);
    let mut config = load_config(&config_path)?;

    match cli.command {
        Commands::List {
            command: Some(ListCommand::Tidy),
            tags,
            open,
            verbose,
            ..
        } => {
            if verbose {
                return Err(anyhow!("--verbose is not supported with 'dsc list tidy'"));
            }
            if open {
                return Err(anyhow!("--open is not supported with 'dsc list tidy'"));
            }
            match tags {
                Some(_) => Err(anyhow!("--tags is not supported with 'dsc list tidy'")),
                None => commands::list::list_tidy(&config_path, &mut config),
            }
        }

        Commands::List {
            format,
            tags,
            open,
            verbose,
            ..
        } => commands::list::list_discourses(&config, format, tags.as_deref(), open, verbose),

        Commands::Add { names, interactive } => {
            commands::add::add_discourses(&mut config, &names, interactive)?;
            save_config(&config_path, &config)
        }

        Commands::Import { path } => {
            commands::import::import_discourses(&mut config, path.as_deref())?;
            save_config(&config_path, &config)
        }

        Commands::Update {
            name,
            parallel,
            max,
            post_changelog,
            yes,
        } => match name.as_str() {
            "all" if max.is_some() && !parallel => Err(anyhow!("--max requires --parallel")),
            "all" if max == Some(0) => Err(anyhow!("--max must be at least 1")),
            "all" => commands::update::update_all(&config, parallel, max, post_changelog, yes),
            _ if parallel || max.is_some() => {
                Err(anyhow!("--parallel/--max only apply to 'dsc update all'"))
            }
            _ => commands::update::update_one(&config, &name, post_changelog, yes),
        },

        Commands::Emoji {
            command:
                EmojiCommand::Add {
                    discourse,
                    emoji_path,
                    emoji_name,
                },
        } => commands::emoji::add_emoji(&config, &discourse, &emoji_path, emoji_name.as_deref()),

        Commands::Emoji {
            command:
                EmojiCommand::List {
                    discourse,
                    format,
                    verbose,
                    inline,
                },
        } => commands::emoji::list_emojis(&config, &discourse, format, verbose, inline),

        Commands::Topic { command } => match command {
            TopicCommand::Pull {
                discourse,
                topic_id,
                local_path,
            } => commands::topic::topic_pull(&config, &discourse, topic_id, local_path.as_deref()),

            TopicCommand::Push {
                discourse,
                local_path,
                topic_id,
            } => commands::topic::topic_push(&config, &discourse, topic_id, &local_path),

            TopicCommand::Sync {
                discourse,
                topic_id,
                local_path,
                yes,
            } => commands::topic::topic_sync(&config, &discourse, topic_id, &local_path, yes),
        },

        Commands::Category { command } => match command {
            CategoryCommand::List {
                discourse,
                format,
                verbose,
                tree,
            } => commands::category::category_list(&config, &discourse, format, verbose, tree),

            CategoryCommand::Copy {
                discourse,
                target,
                category,
            } => {
                commands::category::category_copy(&config, &discourse, target.as_deref(), &category)
            }

            CategoryCommand::Pull {
                discourse,
                category,
                local_path,
            } => commands::category::category_pull(
                &config,
                &discourse,
                &category,
                local_path.as_deref(),
            ),

            CategoryCommand::Push {
                discourse,
                local_path,
                category,
            } => commands::category::category_push(&config, &discourse, &category, &local_path),
        },

        Commands::Group { command } => match command {
            GroupCommand::List {
                discourse,
                format,
                verbose,
            } => commands::group::group_list(&config, &discourse, format, verbose),
            GroupCommand::Info {
                discourse,
                group,
                format,
            } => commands::group::group_info(&config, &discourse, group, format),
            GroupCommand::Members {
                discourse,
                group,
                format,
            } => commands::group::group_members(&config, &discourse, group, format),

            GroupCommand::Copy {
                discourse,
                target,
                group,
            } => commands::group::group_copy(&config, &discourse, target.as_deref(), group),
        },

        Commands::Backup { command } => match command {
            BackupCommand::Create { discourse } => {
                commands::backup::backup_create(&config, &discourse)
            }

            BackupCommand::List {
                discourse,
                format,
                verbose,
            } => commands::backup::backup_list(&config, &discourse, format, verbose),

            BackupCommand::Restore {
                discourse,
                backup_path,
            } => commands::backup::backup_restore(&config, &discourse, &backup_path),
        },

        Commands::Palette { command } => match command {
            PaletteCommand::List {
                discourse,
                format,
                verbose,
            } => commands::palette::palette_list(&config, &discourse, format, verbose),

            PaletteCommand::Pull {
                discourse,
                palette_id,
                local_path,
            } => commands::palette::palette_pull(
                &config,
                &discourse,
                palette_id,
                local_path.as_deref(),
            ),

            PaletteCommand::Push {
                discourse,
                local_path,
                palette_id,
            } => commands::palette::palette_push(&config, &discourse, &local_path, palette_id),
        },

        Commands::Plugin { command } => match command {
            PluginCommand::List {
                discourse,
                format,
                verbose,
            } => commands::plugin::plugin_list(&config, &discourse, format, verbose),
            PluginCommand::Install { discourse, url } => {
                commands::plugin::plugin_install(&config, &discourse, &url)
            }
            PluginCommand::Remove { discourse, name } => {
                commands::plugin::plugin_remove(&config, &discourse, &name)
            }
        },

        Commands::Theme { command } => match command {
            ThemeCommand::List {
                discourse,
                format,
                verbose,
            } => commands::theme::theme_list(&config, &discourse, format, verbose),
            ThemeCommand::Install { discourse, url } => {
                commands::theme::theme_install(&config, &discourse, &url)
            }
            ThemeCommand::Remove { discourse, name } => {
                commands::theme::theme_remove(&config, &discourse, &name)
            }
            ThemeCommand::Pull {
                discourse,
                theme_id,
                local_path,
            } => commands::theme::theme_pull(
                &config,
                &discourse,
                theme_id,
                local_path.as_deref(),
            ),
            ThemeCommand::Push {
                discourse,
                local_path,
                theme_id,
            } => commands::theme::theme_push(&config, &discourse, &local_path, theme_id),
            ThemeCommand::Duplicate {
                discourse,
                theme_id,
            } => commands::theme::theme_duplicate(&config, &discourse, theme_id),
        },

        Commands::Setting {
            command:
                SettingCommand::Set {
                    discourse,
                    setting,
                    value,
                    tags,
                },
        } => commands::setting::set_site_setting(
            &config,
            Some(discourse.as_str()),
            &setting,
            &value,
            tags.as_deref(),
        ),

        Commands::Setting {
            command: SettingCommand::Get { discourse, setting },
        } => commands::setting::get_site_setting(&config, &discourse, &setting),

        Commands::Setting {
            command:
                SettingCommand::List {
                    discourse,
                    format,
                    verbose,
                },
        } => commands::setting::list_site_settings(&config, &discourse, format, verbose),

        Commands::Completions { shell, dir } => {
            commands::completions::write_completions(shell, dir.as_deref())
        }

        Commands::Version => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}
