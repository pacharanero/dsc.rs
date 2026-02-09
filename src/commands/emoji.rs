use crate::commands::common::{ensure_api_credentials, select_discourse};
use crate::config::Config;
use crate::api::DiscourseClient;
use crate::utils::slugify;
use anyhow::{anyhow, Context, Result};
use base64::Engine;
use std::fs;
use std::path::Path;

pub fn add_emoji(
    config: &Config,
    discourse_name: &str,
    emoji_path: &Path,
    emoji_name: Option<&str>,
) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;
    if emoji_path.is_dir() {
        if emoji_name.is_some() {
            return Err(anyhow!(
                "emoji name is not allowed when uploading a directory"
            ));
        }
        let mut files = Vec::new();
        for entry in
            fs::read_dir(emoji_path).with_context(|| format!("reading {}", emoji_path.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if !is_emoji_file(&path) {
                continue;
            }
            files.push(path);
        }
        files.sort();
        if files.is_empty() {
            return Err(anyhow!("no emoji image files found in directory"));
        }
        for path in files {
            let name = emoji_name_from_path(&path)?;
            client.upload_emoji(&path, &name)?;
            println!("uploaded {} from {}", name, path.display());
        }
        return Ok(());
    }

    let name = match emoji_name {
        Some(name) => name.to_string(),
        None => emoji_name_from_path(emoji_path)?,
    };
    client.upload_emoji(emoji_path, &name)?;
    Ok(())
}

pub fn list_emojis(config: &Config, discourse_name: &str, inline: bool) -> Result<()> {
    let discourse = select_discourse(config, Some(discourse_name))?;
    ensure_api_credentials(discourse)?;
    let client = DiscourseClient::new(discourse)?;
    let mut emojis = client.list_custom_emojis()?;
    emojis.sort_by(|a, b| a.name.cmp(&b.name));

    if emojis.is_empty() {
        println!("No custom emojis found");
        return Ok(());
    }

    if inline {
        if let Some(protocol) = detect_inline_protocol() {
            print_inline_emojis(&emojis, protocol)?;
        } else {
            print_emojis_table(&emojis);
        }
    } else {
        print_emojis_table(&emojis);
    }
    Ok(())
}

fn print_emojis_table(emojis: &[crate::api::CustomEmoji]) {
    println!("name\turl");
    for emoji in emojis {
        println!("{}\t{}", emoji.name, emoji.url);
    }
}

#[derive(Clone, Copy)]
enum InlineProtocol {
    Iterm2,
    Kitty,
}

fn detect_inline_protocol() -> Option<InlineProtocol> {
    if let Ok(value) = std::env::var("DSC_EMOJI_INLINE_PROTOCOL") {
        let value = value.trim().to_ascii_lowercase();
        if value == "iterm2" || value == "iterm" {
            return Some(InlineProtocol::Iterm2);
        }
        if value == "kitty" {
            return Some(InlineProtocol::Kitty);
        }
        if value == "off" || value == "0" {
            return None;
        }
    }
    if let Ok(term_program) = std::env::var("TERM_PROGRAM") {
        if term_program == "iTerm.app" || term_program == "WezTerm" {
            return Some(InlineProtocol::Iterm2);
        }
    }
    if std::env::var("KITTY_WINDOW_ID").is_ok()
        || std::env::var("KITTY_SESSION_ID").is_ok()
        || std::env::var("TERM")
            .map(|t| t.contains("kitty"))
            .unwrap_or(false)
    {
        return Some(InlineProtocol::Kitty);
    }
    None
}

fn print_inline_emojis(
    emojis: &[crate::api::CustomEmoji],
    protocol: InlineProtocol,
) -> Result<()> {
    let client = reqwest::blocking::Client::new();
    for emoji in emojis {
        let image = client.get(&emoji.url).send();
        let image = match image {
            Ok(response) if response.status().is_success() => response.bytes(),
            _ => {
                println!("{}\t{}", emoji.name, emoji.url);
                continue;
            }
        };
        let image = match image {
            Ok(bytes) => bytes,
            Err(_) => {
                println!("{}\t{}", emoji.name, emoji.url);
                continue;
            }
        };
        let encoded = base64::engine::general_purpose::STANDARD.encode(&image);
        match protocol {
            InlineProtocol::Iterm2 => {
                let sequence = format!(
                    "\u{1b}]1337;File=inline=1;width=1;height=1;preserveAspectRatio=1:{}\u{7}",
                    encoded
                );
                println!("{} {}", emoji.name, sequence);
            }
            InlineProtocol::Kitty => {
                let sequence = format!("\u{1b}_Gf=100,t=d;{}\u{1b}\\", encoded);
                println!("{} {}", emoji.name, sequence);
            }
        }
    }
    Ok(())
}

fn emoji_name_from_path(path: &Path) -> Result<String> {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("emoji path missing filename: {}", path.display()))?;
    let slug = slugify(stem);
    let name = slug.replace('-', "_");
    if name.is_empty() {
        return Err(anyhow!("emoji name is empty for {}", path.display()));
    }
    Ok(name)
}

fn is_emoji_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
        return false;
    };
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "svg"
    )
}
