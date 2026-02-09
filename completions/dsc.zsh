#compdef dsc

autoload -U is-at-least

_dsc_discourse_names() {
    local config_path
    local i
    for i in {1..$#words}; do
        if [[ ${words[$i]} == -c || ${words[$i]} == --config ]]; then
            config_path=${words[$((i+1))]}
        elif [[ ${words[$i]} == --config=* ]]; then
            config_path=${words[$i]#--config=}
        fi
    done

    local cmd=(dsc list --format plaintext)
    if [[ -n ${config_path:-} ]]; then
        cmd+=(-c "$config_path")
    fi

    local -a names
    names=(${(f)"$(command ${cmd[@]} 2>/dev/null | sed 's/ - .*//')"})
    _describe -t discourses 'discourses' names
}

_dsc() {
    typeset -A opt_args
    typeset -a _arguments_options
    local ret=1

    if is-at-least 5.2; then
        _arguments_options=(-s -S -C)
    else
        _arguments_options=(-s -C)
    fi

    local context curcontext="$curcontext" state line
    _arguments "${_arguments_options[@]}" : \
'-c+[]:CONFIG:_files' \
'--config=[]:CONFIG:_files' \
'-h[Print help]' \
'--help[Print help]' \
":: :_dsc_commands" \
"*::: :->dsc" \
&& ret=0
    case $state in
    (dsc)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:dsc-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
'-f+[]:FORMAT:(plaintext markdown markdown-table json yaml csv)' \
'--format=[]:FORMAT:(plaintext markdown markdown-table json yaml csv)' \
'-h[Print help]' \
'--help[Print help]' \
&& ret=0
;;
(add)
_arguments "${_arguments_options[@]}" : \
'-i[]' \
'--interactive[]' \
'-h[Print help]' \
'--help[Print help]' \
':names:_default' \
&& ret=0
;;
(import)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
'::path:_files' \
&& ret=0
;;
(update)
_arguments "${_arguments_options[@]}" : \
'-m+[]:MAX:_default' \
'--max=[]:MAX:_default' \
'-C[]' \
'--concurrent[]' \
'-p[]' \
'--post-changelog[]' \
'-h[Print help]' \
'--help[Print help]' \
':name:_dsc_discourse_names' \
&& ret=0
;;
(emoji)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
":: :_dsc__emoji_commands" \
"*::: :->emoji" \
&& ret=0

    case $state in
    (emoji)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:dsc-emoji-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
':discourse:_default' \
':emoji_path:_files' \
':emoji_name:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_dsc__emoji__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:dsc-emoji-help-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(topic)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
":: :_dsc__topic_commands" \
"*::: :->topic" \
&& ret=0

    case $state in
    (topic)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:dsc-topic-command-$line[1]:"
        case $line[1] in
            (pull)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
':discourse:_default' \
':topic_id:_default' \
'::local_path:_files' \
&& ret=0
;;
(push)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
':discourse:_default' \
':local_path:_files' \
':topic_id:_default' \
&& ret=0
;;
(sync)
_arguments "${_arguments_options[@]}" : \
'-y[]' \
'--yes[]' \
'-h[Print help]' \
'--help[Print help]' \
':discourse:_default' \
':topic_id:_default' \
':local_path:_files' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_dsc__topic__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:dsc-topic-help-command-$line[1]:"
        case $line[1] in
            (pull)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(push)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(sync)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(category)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
":: :_dsc__category_commands" \
"*::: :->category" \
&& ret=0

    case $state in
    (category)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:dsc-category-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
'--tree[]' \
'-h[Print help]' \
'--help[Print help]' \
':discourse:_default' \
&& ret=0
;;
(copy)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
':discourse:_default' \
':category_id:_default' \
&& ret=0
;;
(pull)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
':discourse:_default' \
':category_id:_default' \
'::local_path:_files' \
&& ret=0
;;
(push)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
':discourse:_default' \
':local_path:_files' \
':category_id:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_dsc__category__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:dsc-category-help-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(copy)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(pull)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(push)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(group)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
":: :_dsc__group_commands" \
"*::: :->group" \
&& ret=0

    case $state in
    (group)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:dsc-group-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
':discourse:_default' \
&& ret=0
;;
(info)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
':discourse:_default' \
':group:_default' \
&& ret=0
;;
(copy)
_arguments "${_arguments_options[@]}" : \
'-t+[]:TARGET:_default' \
'--target=[]:TARGET:_default' \
'-h[Print help]' \
'--help[Print help]' \
':discourse:_default' \
':group:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_dsc__group__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:dsc-group-help-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(info)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(copy)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(backup)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
":: :_dsc__backup_commands" \
"*::: :->backup" \
&& ret=0

    case $state in
    (backup)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:dsc-backup-command-$line[1]:"
        case $line[1] in
            (create)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
':discourse:_default' \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
':discourse:_default' \
&& ret=0
;;
(restore)
_arguments "${_arguments_options[@]}" : \
'-h[Print help]' \
'--help[Print help]' \
':discourse:_default' \
':backup_path:_default' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_dsc__backup__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:dsc-backup-help-command-$line[1]:"
        case $line[1] in
            (create)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(restore)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
;;
(completions)
_arguments "${_arguments_options[@]}" : \
'-d+[]:DIR:_files' \
'--dir=[]:DIR:_files' \
'-h[Print help]' \
'--help[Print help]' \
':shell:(bash zsh fish)' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_dsc__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:dsc-help-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(add)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(import)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(update)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(emoji)
_arguments "${_arguments_options[@]}" : \
":: :_dsc__help__emoji_commands" \
"*::: :->emoji" \
&& ret=0

    case $state in
    (emoji)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:dsc-help-emoji-command-$line[1]:"
        case $line[1] in
            (add)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(topic)
_arguments "${_arguments_options[@]}" : \
":: :_dsc__help__topic_commands" \
"*::: :->topic" \
&& ret=0

    case $state in
    (topic)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:dsc-help-topic-command-$line[1]:"
        case $line[1] in
            (pull)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(push)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(sync)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(category)
_arguments "${_arguments_options[@]}" : \
":: :_dsc__help__category_commands" \
"*::: :->category" \
&& ret=0

    case $state in
    (category)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:dsc-help-category-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(copy)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(pull)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(push)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(group)
_arguments "${_arguments_options[@]}" : \
":: :_dsc__help__group_commands" \
"*::: :->group" \
&& ret=0

    case $state in
    (group)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:dsc-help-group-command-$line[1]:"
        case $line[1] in
            (list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(info)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(copy)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(backup)
_arguments "${_arguments_options[@]}" : \
":: :_dsc__help__backup_commands" \
"*::: :->backup" \
&& ret=0

    case $state in
    (backup)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:dsc-help-backup-command-$line[1]:"
        case $line[1] in
            (create)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(list)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(restore)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
(completions)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
}

(( $+functions[_dsc_commands] )) ||
_dsc_commands() {
    local commands; commands=(
'list:' \
'add:' \
'import:' \
'update:' \
'emoji:' \
'topic:' \
'category:' \
'group:' \
'backup:' \
'completions:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'dsc commands' commands "$@"
}
(( $+functions[_dsc__add_commands] )) ||
_dsc__add_commands() {
    local commands; commands=()
    _describe -t commands 'dsc add commands' commands "$@"
}
(( $+functions[_dsc__backup_commands] )) ||
_dsc__backup_commands() {
    local commands; commands=(
'create:' \
'list:' \
'restore:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'dsc backup commands' commands "$@"
}
(( $+functions[_dsc__backup__create_commands] )) ||
_dsc__backup__create_commands() {
    local commands; commands=()
    _describe -t commands 'dsc backup create commands' commands "$@"
}
(( $+functions[_dsc__backup__help_commands] )) ||
_dsc__backup__help_commands() {
    local commands; commands=(
'create:' \
'list:' \
'restore:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'dsc backup help commands' commands "$@"
}
(( $+functions[_dsc__backup__help__create_commands] )) ||
_dsc__backup__help__create_commands() {
    local commands; commands=()
    _describe -t commands 'dsc backup help create commands' commands "$@"
}
(( $+functions[_dsc__backup__help__help_commands] )) ||
_dsc__backup__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'dsc backup help help commands' commands "$@"
}
(( $+functions[_dsc__backup__help__list_commands] )) ||
_dsc__backup__help__list_commands() {
    local commands; commands=()
    _describe -t commands 'dsc backup help list commands' commands "$@"
}
(( $+functions[_dsc__backup__help__restore_commands] )) ||
_dsc__backup__help__restore_commands() {
    local commands; commands=()
    _describe -t commands 'dsc backup help restore commands' commands "$@"
}
(( $+functions[_dsc__backup__list_commands] )) ||
_dsc__backup__list_commands() {
    local commands; commands=()
    _describe -t commands 'dsc backup list commands' commands "$@"
}
(( $+functions[_dsc__backup__restore_commands] )) ||
_dsc__backup__restore_commands() {
    local commands; commands=()
    _describe -t commands 'dsc backup restore commands' commands "$@"
}
(( $+functions[_dsc__category_commands] )) ||
_dsc__category_commands() {
    local commands; commands=(
'list:' \
'copy:' \
'pull:' \
'push:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'dsc category commands' commands "$@"
}
(( $+functions[_dsc__category__copy_commands] )) ||
_dsc__category__copy_commands() {
    local commands; commands=()
    _describe -t commands 'dsc category copy commands' commands "$@"
}
(( $+functions[_dsc__category__help_commands] )) ||
_dsc__category__help_commands() {
    local commands; commands=(
'list:' \
'copy:' \
'pull:' \
'push:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'dsc category help commands' commands "$@"
}
(( $+functions[_dsc__category__help__copy_commands] )) ||
_dsc__category__help__copy_commands() {
    local commands; commands=()
    _describe -t commands 'dsc category help copy commands' commands "$@"
}
(( $+functions[_dsc__category__help__help_commands] )) ||
_dsc__category__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'dsc category help help commands' commands "$@"
}
(( $+functions[_dsc__category__help__list_commands] )) ||
_dsc__category__help__list_commands() {
    local commands; commands=()
    _describe -t commands 'dsc category help list commands' commands "$@"
}
(( $+functions[_dsc__category__help__pull_commands] )) ||
_dsc__category__help__pull_commands() {
    local commands; commands=()
    _describe -t commands 'dsc category help pull commands' commands "$@"
}
(( $+functions[_dsc__category__help__push_commands] )) ||
_dsc__category__help__push_commands() {
    local commands; commands=()
    _describe -t commands 'dsc category help push commands' commands "$@"
}
(( $+functions[_dsc__category__list_commands] )) ||
_dsc__category__list_commands() {
    local commands; commands=()
    _describe -t commands 'dsc category list commands' commands "$@"
}
(( $+functions[_dsc__category__pull_commands] )) ||
_dsc__category__pull_commands() {
    local commands; commands=()
    _describe -t commands 'dsc category pull commands' commands "$@"
}
(( $+functions[_dsc__category__push_commands] )) ||
_dsc__category__push_commands() {
    local commands; commands=()
    _describe -t commands 'dsc category push commands' commands "$@"
}
(( $+functions[_dsc__completions_commands] )) ||
_dsc__completions_commands() {
    local commands; commands=()
    _describe -t commands 'dsc completions commands' commands "$@"
}
(( $+functions[_dsc__emoji_commands] )) ||
_dsc__emoji_commands() {
    local commands; commands=(
'add:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'dsc emoji commands' commands "$@"
}
(( $+functions[_dsc__emoji__add_commands] )) ||
_dsc__emoji__add_commands() {
    local commands; commands=()
    _describe -t commands 'dsc emoji add commands' commands "$@"
}
(( $+functions[_dsc__emoji__help_commands] )) ||
_dsc__emoji__help_commands() {
    local commands; commands=(
'add:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'dsc emoji help commands' commands "$@"
}
(( $+functions[_dsc__emoji__help__add_commands] )) ||
_dsc__emoji__help__add_commands() {
    local commands; commands=()
    _describe -t commands 'dsc emoji help add commands' commands "$@"
}
(( $+functions[_dsc__emoji__help__help_commands] )) ||
_dsc__emoji__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'dsc emoji help help commands' commands "$@"
}
(( $+functions[_dsc__group_commands] )) ||
_dsc__group_commands() {
    local commands; commands=(
'list:' \
'info:' \
'copy:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'dsc group commands' commands "$@"
}
(( $+functions[_dsc__group__copy_commands] )) ||
_dsc__group__copy_commands() {
    local commands; commands=()
    _describe -t commands 'dsc group copy commands' commands "$@"
}
(( $+functions[_dsc__group__help_commands] )) ||
_dsc__group__help_commands() {
    local commands; commands=(
'list:' \
'info:' \
'copy:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'dsc group help commands' commands "$@"
}
(( $+functions[_dsc__group__help__copy_commands] )) ||
_dsc__group__help__copy_commands() {
    local commands; commands=()
    _describe -t commands 'dsc group help copy commands' commands "$@"
}
(( $+functions[_dsc__group__help__help_commands] )) ||
_dsc__group__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'dsc group help help commands' commands "$@"
}
(( $+functions[_dsc__group__help__info_commands] )) ||
_dsc__group__help__info_commands() {
    local commands; commands=()
    _describe -t commands 'dsc group help info commands' commands "$@"
}
(( $+functions[_dsc__group__help__list_commands] )) ||
_dsc__group__help__list_commands() {
    local commands; commands=()
    _describe -t commands 'dsc group help list commands' commands "$@"
}
(( $+functions[_dsc__group__info_commands] )) ||
_dsc__group__info_commands() {
    local commands; commands=()
    _describe -t commands 'dsc group info commands' commands "$@"
}
(( $+functions[_dsc__group__list_commands] )) ||
_dsc__group__list_commands() {
    local commands; commands=()
    _describe -t commands 'dsc group list commands' commands "$@"
}
(( $+functions[_dsc__help_commands] )) ||
_dsc__help_commands() {
    local commands; commands=(
'list:' \
'add:' \
'import:' \
'update:' \
'emoji:' \
'topic:' \
'category:' \
'group:' \
'backup:' \
'completions:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'dsc help commands' commands "$@"
}
(( $+functions[_dsc__help__add_commands] )) ||
_dsc__help__add_commands() {
    local commands; commands=()
    _describe -t commands 'dsc help add commands' commands "$@"
}
(( $+functions[_dsc__help__backup_commands] )) ||
_dsc__help__backup_commands() {
    local commands; commands=(
'create:' \
'list:' \
'restore:' \
    )
    _describe -t commands 'dsc help backup commands' commands "$@"
}
(( $+functions[_dsc__help__backup__create_commands] )) ||
_dsc__help__backup__create_commands() {
    local commands; commands=()
    _describe -t commands 'dsc help backup create commands' commands "$@"
}
(( $+functions[_dsc__help__backup__list_commands] )) ||
_dsc__help__backup__list_commands() {
    local commands; commands=()
    _describe -t commands 'dsc help backup list commands' commands "$@"
}
(( $+functions[_dsc__help__backup__restore_commands] )) ||
_dsc__help__backup__restore_commands() {
    local commands; commands=()
    _describe -t commands 'dsc help backup restore commands' commands "$@"
}
(( $+functions[_dsc__help__category_commands] )) ||
_dsc__help__category_commands() {
    local commands; commands=(
'list:' \
'copy:' \
'pull:' \
'push:' \
    )
    _describe -t commands 'dsc help category commands' commands "$@"
}
(( $+functions[_dsc__help__category__copy_commands] )) ||
_dsc__help__category__copy_commands() {
    local commands; commands=()
    _describe -t commands 'dsc help category copy commands' commands "$@"
}
(( $+functions[_dsc__help__category__list_commands] )) ||
_dsc__help__category__list_commands() {
    local commands; commands=()
    _describe -t commands 'dsc help category list commands' commands "$@"
}
(( $+functions[_dsc__help__category__pull_commands] )) ||
_dsc__help__category__pull_commands() {
    local commands; commands=()
    _describe -t commands 'dsc help category pull commands' commands "$@"
}
(( $+functions[_dsc__help__category__push_commands] )) ||
_dsc__help__category__push_commands() {
    local commands; commands=()
    _describe -t commands 'dsc help category push commands' commands "$@"
}
(( $+functions[_dsc__help__completions_commands] )) ||
_dsc__help__completions_commands() {
    local commands; commands=()
    _describe -t commands 'dsc help completions commands' commands "$@"
}
(( $+functions[_dsc__help__emoji_commands] )) ||
_dsc__help__emoji_commands() {
    local commands; commands=(
'add:' \
    )
    _describe -t commands 'dsc help emoji commands' commands "$@"
}
(( $+functions[_dsc__help__emoji__add_commands] )) ||
_dsc__help__emoji__add_commands() {
    local commands; commands=()
    _describe -t commands 'dsc help emoji add commands' commands "$@"
}
(( $+functions[_dsc__help__group_commands] )) ||
_dsc__help__group_commands() {
    local commands; commands=(
'list:' \
'info:' \
'copy:' \
    )
    _describe -t commands 'dsc help group commands' commands "$@"
}
(( $+functions[_dsc__help__group__copy_commands] )) ||
_dsc__help__group__copy_commands() {
    local commands; commands=()
    _describe -t commands 'dsc help group copy commands' commands "$@"
}
(( $+functions[_dsc__help__group__info_commands] )) ||
_dsc__help__group__info_commands() {
    local commands; commands=()
    _describe -t commands 'dsc help group info commands' commands "$@"
}
(( $+functions[_dsc__help__group__list_commands] )) ||
_dsc__help__group__list_commands() {
    local commands; commands=()
    _describe -t commands 'dsc help group list commands' commands "$@"
}
(( $+functions[_dsc__help__help_commands] )) ||
_dsc__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'dsc help help commands' commands "$@"
}
(( $+functions[_dsc__help__import_commands] )) ||
_dsc__help__import_commands() {
    local commands; commands=()
    _describe -t commands 'dsc help import commands' commands "$@"
}
(( $+functions[_dsc__help__list_commands] )) ||
_dsc__help__list_commands() {
    local commands; commands=()
    _describe -t commands 'dsc help list commands' commands "$@"
}
(( $+functions[_dsc__help__topic_commands] )) ||
_dsc__help__topic_commands() {
    local commands; commands=(
'pull:' \
'push:' \
'sync:' \
    )
    _describe -t commands 'dsc help topic commands' commands "$@"
}
(( $+functions[_dsc__help__topic__pull_commands] )) ||
_dsc__help__topic__pull_commands() {
    local commands; commands=()
    _describe -t commands 'dsc help topic pull commands' commands "$@"
}
(( $+functions[_dsc__help__topic__push_commands] )) ||
_dsc__help__topic__push_commands() {
    local commands; commands=()
    _describe -t commands 'dsc help topic push commands' commands "$@"
}
(( $+functions[_dsc__help__topic__sync_commands] )) ||
_dsc__help__topic__sync_commands() {
    local commands; commands=()
    _describe -t commands 'dsc help topic sync commands' commands "$@"
}
(( $+functions[_dsc__help__update_commands] )) ||
_dsc__help__update_commands() {
    local commands; commands=()
    _describe -t commands 'dsc help update commands' commands "$@"
}
(( $+functions[_dsc__import_commands] )) ||
_dsc__import_commands() {
    local commands; commands=()
    _describe -t commands 'dsc import commands' commands "$@"
}
(( $+functions[_dsc__list_commands] )) ||
_dsc__list_commands() {
    local commands; commands=()
    _describe -t commands 'dsc list commands' commands "$@"
}
(( $+functions[_dsc__topic_commands] )) ||
_dsc__topic_commands() {
    local commands; commands=(
'pull:' \
'push:' \
'sync:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'dsc topic commands' commands "$@"
}
(( $+functions[_dsc__topic__help_commands] )) ||
_dsc__topic__help_commands() {
    local commands; commands=(
'pull:' \
'push:' \
'sync:' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'dsc topic help commands' commands "$@"
}
(( $+functions[_dsc__topic__help__help_commands] )) ||
_dsc__topic__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'dsc topic help help commands' commands "$@"
}
(( $+functions[_dsc__topic__help__pull_commands] )) ||
_dsc__topic__help__pull_commands() {
    local commands; commands=()
    _describe -t commands 'dsc topic help pull commands' commands "$@"
}
(( $+functions[_dsc__topic__help__push_commands] )) ||
_dsc__topic__help__push_commands() {
    local commands; commands=()
    _describe -t commands 'dsc topic help push commands' commands "$@"
}
(( $+functions[_dsc__topic__help__sync_commands] )) ||
_dsc__topic__help__sync_commands() {
    local commands; commands=()
    _describe -t commands 'dsc topic help sync commands' commands "$@"
}
(( $+functions[_dsc__topic__pull_commands] )) ||
_dsc__topic__pull_commands() {
    local commands; commands=()
    _describe -t commands 'dsc topic pull commands' commands "$@"
}
(( $+functions[_dsc__topic__push_commands] )) ||
_dsc__topic__push_commands() {
    local commands; commands=()
    _describe -t commands 'dsc topic push commands' commands "$@"
}
(( $+functions[_dsc__topic__sync_commands] )) ||
_dsc__topic__sync_commands() {
    local commands; commands=()
    _describe -t commands 'dsc topic sync commands' commands "$@"
}
(( $+functions[_dsc__update_commands] )) ||
_dsc__update_commands() {
    local commands; commands=()
    _describe -t commands 'dsc update commands' commands "$@"
}

if [ "$funcstack[1]" = "_dsc" ]; then
    _dsc "$@"
else
    compdef _dsc dsc
fi
