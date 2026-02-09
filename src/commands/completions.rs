use crate::cli::{Cli, CompletionShell};
use crate::utils::ensure_dir;
use anyhow::{Context, Result};
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::fs;
use std::io;
use std::path::Path;

pub fn write_completions(shell: CompletionShell, dir: Option<&Path>) -> Result<()> {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    match dir {
        Some(dir) => {
            ensure_dir(dir)?;
            let filename = match shell {
                CompletionShell::Bash => "dsc.bash",
                CompletionShell::Zsh => "_dsc",
                CompletionShell::Fish => "dsc.fish",
            };
            let path = dir.join(filename);
            let generator: Shell = shell.into();
            if matches!(shell, CompletionShell::Zsh) {
                let mut buffer = Vec::new();
                generate(generator, &mut cmd, name, &mut buffer);
                let content = String::from_utf8(buffer).context("decoding zsh completions")?;
                let content = inject_zsh_sort_style(content);
                let content = inject_zsh_dynamic_discourse_completion(content);
                fs::write(&path, content).with_context(|| format!("writing {}", path.display()))?;
            } else {
                let mut file = fs::File::create(&path)
                    .with_context(|| format!("creating {}", path.display()))?;
                generate(generator, &mut cmd, name, &mut file);
            }
            println!("{}", path.display());
        }
        None => {
            let generator: Shell = shell.into();
            if matches!(shell, CompletionShell::Zsh) {
                let mut buffer = Vec::new();
                generate(generator, &mut cmd, name, &mut buffer);
                let content = String::from_utf8(buffer).context("decoding zsh completions")?;
                let content = inject_zsh_sort_style(content);
                let content = inject_zsh_dynamic_discourse_completion(content);
                print!("{}", content);
            } else {
                let mut stdout = io::stdout();
                generate(generator, &mut cmd, name, &mut stdout);
            }
        }
    }
    Ok(())
}

fn inject_zsh_sort_style(mut content: String) -> String {
    let style = "zstyle ':completion:*:dsc:*' sort false";
    if content.contains(style) {
        return content;
    }
    let marker = "autoload -U is-at-least\n";
    if let Some(pos) = content.find(marker) {
        let insert_at = pos + marker.len();
        content.insert_str(insert_at, &format!("\n{}\n", style));
        return content;
    }
    format!("{}\n\n{}", style, content)
}

fn inject_zsh_dynamic_discourse_completion(mut content: String) -> String {
    if !content.contains("_dsc_discourse_names()") {
        let marker = "autoload -U is-at-least\n";
        let function = "\n_dsc_discourse_names() {\n\
    local config_path\n\
    local i\n\
    for i in {1..$#words}; do\n\
        if [[ ${words[$i]} == -c || ${words[$i]} == --config ]]; then\n\
            config_path=${words[$((i+1))]}\n\
        elif [[ ${words[$i]} == --config=* ]]; then\n\
            config_path=${words[$i]#--config=}\n\
        fi\n\
    done\n\
\n\
    local cmd=(dsc list --format plaintext)\n\
    if [[ -n ${config_path:-} ]]; then\n\
        cmd+=(-c \"$config_path\")\n\
    fi\n\
\n\
    local -a names\n\
    names=(${(f)\"$(command ${cmd[@]} 2>/dev/null | sed 's/ - .*//')\"})\n\
    _describe -t discourses 'discourses' names\n\
}\n";
        if let Some(pos) = content.find(marker) {
            let insert_at = pos + marker.len();
            content.insert_str(insert_at, function);
        } else {
            content = format!("{}{}", function.trim_start(), content);
        }
    }

    replace_update_name_completion(content)
}

fn replace_update_name_completion(content: String) -> String {
    let mut output = String::with_capacity(content.len());
    let mut remaining = content.as_str();
    let update_marker = "(update)\n_arguments";
    while let Some(pos) = remaining.find(update_marker) {
        let (before, rest) = remaining.split_at(pos);
        output.push_str(before);

        let rest = &rest[update_marker.len()..];
        output.push_str(update_marker);

        if let Some(name_pos) = rest.find("':name:_default'") {
            let (mid, tail) = rest.split_at(name_pos);
            output.push_str(mid);
            output.push_str("':name:_dsc_discourse_names'");
            output.push_str(&tail["':name:_default'".len()..]);
            remaining = "";
            output.push_str(remaining);
            return output;
        }

        remaining = rest;
    }

    output.push_str(remaining);
    output
}
