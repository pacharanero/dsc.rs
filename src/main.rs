use anyhow::{anyhow, Result};
use clap::Parser;
use dsc::cli::*;
use dsc::commands;
use dsc::commands::user::{ActivityFormat, Role};
use dsc::config::{load_config, resolve_default_config_path, save_config};

fn map_role(role: RoleArg) -> Role {
    match role {
        RoleArg::Admin => Role::Admin,
        RoleArg::Moderator => Role::Moderator,
    }
}

fn map_activity_format(f: ActivityFormatArg) -> ActivityFormat {
    match f {
        ActivityFormatArg::Text => ActivityFormat::Text,
        ActivityFormatArg::Json => ActivityFormat::Json,
        ActivityFormatArg::Yaml => ActivityFormat::Yaml,
        ActivityFormatArg::Markdown => ActivityFormat::Markdown,
        ActivityFormatArg::Csv => ActivityFormat::Csv,
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_path = cli.config.unwrap_or_else(resolve_default_config_path);
    let mut config = load_config(&config_path)?;
    let dry_run = cli.dry_run;

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
            } => commands::topic::topic_push(&config, &discourse, topic_id, &local_path, dry_run),

            TopicCommand::Sync {
                discourse,
                topic_id,
                local_path,
                yes,
            } => commands::topic::topic_sync(&config, &discourse, topic_id, &local_path, yes),

            TopicCommand::Reply {
                discourse,
                topic_id,
                local_path,
            } => commands::topic::topic_reply(
                &config,
                &discourse,
                topic_id,
                local_path.as_deref(),
            ),

            TopicCommand::New {
                discourse,
                category_id,
                title,
                local_path,
            } => commands::topic::topic_new(
                &config,
                &discourse,
                category_id,
                &title,
                local_path.as_deref(),
                dry_run,
            ),
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
            } => commands::category::category_copy(
                &config,
                &discourse,
                target.as_deref(),
                &category,
                dry_run,
            ),

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
            } => commands::group::group_copy(
                &config,
                &discourse,
                target.as_deref(),
                group,
                dry_run,
            ),

            GroupCommand::Add {
                discourse,
                group,
                local_path,
                notify,
            } => commands::group::group_add(
                &config,
                &discourse,
                group,
                local_path.as_deref(),
                notify,
                dry_run,
            ),
        },

        Commands::Pm { command } => match command {
            PmCommand::Send {
                discourse,
                recipients,
                title,
                local_path,
            } => commands::pm::pm_send(
                &config,
                &discourse,
                &recipients,
                &title,
                local_path.as_deref(),
                dry_run,
            ),
            PmCommand::List {
                discourse,
                username,
                direction,
                format,
            } => commands::pm::pm_list(&config, &discourse, &username, &direction, format),
        },

        Commands::ApiKey { command } => match command {
            ApiKeyCommand::List { discourse, format } => {
                commands::api_key::api_key_list(&config, &discourse, format)
            }
            ApiKeyCommand::Create {
                discourse,
                description,
                username,
                format,
            } => commands::api_key::api_key_create(
                &config,
                &discourse,
                &description,
                username.as_deref(),
                format,
                dry_run,
            ),
            ApiKeyCommand::Revoke { discourse, key_id } => {
                commands::api_key::api_key_revoke(&config, &discourse, key_id, dry_run)
            }
        },

        Commands::Invite { command } => match command {
            InviteCommand::Send {
                discourse,
                email,
                group,
                topic,
                message,
            } => commands::invite::invite_one(
                &config,
                &discourse,
                &email,
                &group,
                topic,
                message.as_deref(),
                dry_run,
            ),
            InviteCommand::Bulk {
                discourse,
                local_path,
                group,
                topic,
                message,
            } => commands::invite::invite_bulk(
                &config,
                &discourse,
                local_path.as_deref(),
                &group,
                topic,
                message.as_deref(),
                dry_run,
            ),
        },

        Commands::User { command } => match command {
            UserCommand::List {
                discourse,
                listing,
                page,
                format,
            } => commands::user::user_list(&config, &discourse, &listing, page, format),
            UserCommand::Info {
                discourse,
                username,
                format,
            } => commands::user::user_info(&config, &discourse, &username, format),
            UserCommand::Suspend {
                discourse,
                username,
                until,
                reason,
            } => commands::user::user_suspend(
                &config,
                &discourse,
                &username,
                &until,
                &reason,
                dry_run,
            ),
            UserCommand::Unsuspend {
                discourse,
                username,
            } => commands::user::user_unsuspend(&config, &discourse, &username, dry_run),
            UserCommand::Silence {
                discourse,
                username,
                until,
                reason,
            } => commands::user::user_silence(
                &config,
                &discourse,
                &username,
                &until,
                &reason,
                dry_run,
            ),
            UserCommand::Unsilence {
                discourse,
                username,
            } => commands::user::user_unsilence(&config, &discourse, &username, dry_run),
            UserCommand::Promote {
                discourse,
                username,
                role,
            } => commands::user::user_promote(
                &config,
                &discourse,
                &username,
                map_role(role),
                dry_run,
            ),
            UserCommand::Demote {
                discourse,
                username,
                role,
            } => commands::user::user_demote(
                &config,
                &discourse,
                &username,
                map_role(role),
                dry_run,
            ),
            UserCommand::Create {
                discourse,
                email,
                username,
                name,
                password_stdin,
                approve,
            } => commands::user::user_create(
                &config,
                &discourse,
                &email,
                &username,
                name.as_deref(),
                password_stdin,
                approve,
                dry_run,
            ),
            UserCommand::PasswordReset {
                discourse,
                username,
            } => commands::user::user_password_reset(&config, &discourse, &username, dry_run),
            UserCommand::EmailSet {
                discourse,
                username,
                email,
            } => commands::user::user_email_set(&config, &discourse, &username, &email, dry_run),
            UserCommand::Activity {
                discourse,
                username,
                since,
                types,
                limit,
                format,
            } => {
                let names: Vec<String> = vec![types];
                commands::user::user_activity(
                    &config,
                    &discourse,
                    &username,
                    &names,
                    since.as_deref(),
                    limit,
                    map_activity_format(format),
                )
            }
            UserCommand::Groups { command } => match command {
                UserGroupsCommand::List {
                    discourse,
                    username,
                    format,
                } => commands::user::user_groups_list(&config, &discourse, &username, format),
                UserGroupsCommand::Add {
                    discourse,
                    username,
                    group_id,
                    notify,
                } => commands::user::user_groups_add(
                    &config,
                    &discourse,
                    &username,
                    group_id,
                    notify,
                    dry_run,
                ),
                UserGroupsCommand::Remove {
                    discourse,
                    username,
                    group_id,
                } => commands::user::user_groups_remove(
                    &config,
                    &discourse,
                    &username,
                    group_id,
                    dry_run,
                ),
            },
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
            } => commands::backup::backup_restore(&config, &discourse, &backup_path, dry_run),
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
                commands::plugin::plugin_install(&config, &discourse, &url, dry_run)
            }
            PluginCommand::Remove { discourse, name } => {
                commands::plugin::plugin_remove(&config, &discourse, &name, dry_run)
            }
        },

        Commands::Theme { command } => match command {
            ThemeCommand::List {
                discourse,
                format,
                verbose,
            } => commands::theme::theme_list(&config, &discourse, format, verbose),
            ThemeCommand::Install { discourse, url } => {
                commands::theme::theme_install(&config, &discourse, &url, dry_run)
            }
            ThemeCommand::Remove { discourse, name } => {
                commands::theme::theme_remove(&config, &discourse, &name, dry_run)
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
            dry_run,
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

        Commands::Open { discourse } => commands::open::open_discourse(&config, &discourse),

        Commands::Harden {
            host,
            ssh_user,
            new_user,
            ssh_port,
            pubkey_file,
        } => commands::harden::harden(
            &config.harden,
            &host,
            &ssh_user,
            new_user.as_deref(),
            ssh_port,
            &pubkey_file,
            dry_run,
        ),

        Commands::Search {
            discourse,
            query,
            format,
        } => commands::search::search(&config, &discourse, &query, format),

        Commands::Upload {
            discourse,
            file,
            upload_type,
            format,
        } => commands::upload::upload(&config, &discourse, &file, &upload_type, format),

        Commands::Post { command } => match command {
            PostCommand::Edit {
                discourse,
                post_id,
                local_path,
            } => commands::post::post_edit(
                &config,
                &discourse,
                post_id,
                local_path.as_deref(),
                dry_run,
            ),
            PostCommand::Delete { discourse, post_id } => {
                commands::post::post_delete(&config, &discourse, post_id, dry_run)
            }
            PostCommand::Move {
                discourse,
                post_id,
                to_topic,
            } => commands::post::post_move(&config, &discourse, post_id, to_topic, dry_run),
        },

        Commands::Tag { command } => match command {
            TagCommand::List { discourse, format } => {
                commands::tag::tag_list(&config, &discourse, format)
            }
            TagCommand::Apply {
                discourse,
                topic_id,
                tag,
            } => commands::tag::tag_apply(&config, &discourse, topic_id, &tag, dry_run),
            TagCommand::Remove {
                discourse,
                topic_id,
                tag,
            } => commands::tag::tag_remove(&config, &discourse, topic_id, &tag, dry_run),
        },

        Commands::Config {
            command: ConfigCommand::Check { format, skip_ssh },
        } => commands::config::config_check(&config, format, skip_ssh),

        Commands::Completions { shell, dir } => {
            commands::completions::write_completions(shell, dir.as_deref())
        }

        Commands::Version => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}
