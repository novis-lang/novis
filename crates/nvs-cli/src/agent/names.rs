//! The index lines for what a user types outside a program: configuration keys,
//! commands, flags and diagnostic codes.
//!
//! `rule:tooling/the-index-names-every-key-command-and-code` is what these lines
//! are. Each kind is derived at the call from the table the binary already runs
//! on, and kept nowhere:
//!
//! * a **key** from [`nvs_config::default_file`], the shipped `nvs.toml` that
//!   spells every key the parser reads and is held to that parser by
//!   `bun nv directives --check-template`. Its line is
//!   `config: [server] max_in_flight`; its card is the comment above the key
//!   and the key's own line as the file writes it, which is what an operator
//!   reads there. The dotted spelling, `server.max_in_flight`, reaches it too.
//! * a **command** and a **flag** from [`crate::Cli`]'s `clap` definition,
//!   hidden ones left out: `command: nvs serve` and `flag: nvs serve --port`,
//!   carded by the same text `--help` prints. Positional arguments are not
//!   flags and have no line.
//! * a **code** from [`nvs_diagnostics::code::ALL`]: `code: E0621`, and after
//!   two spaces the first sentence of its card once it carries one
//!   (`rule:tooling/a-diagnostic-code-carries-its-card`). Its `show` card is
//!   [`nvs_diagnostics::code::card`]'s text whole.
//!
//! These lines do not open with their symbol, as a member's does. The symbol is
//! everything after the `config: `, `command: ` or `flag: `, and the code alone
//! after `code: `. They are exact names and never a search of the text under
//! them, so an empty `find` still means that the name does not exist.

use clap::CommandFactory as _;

use super::{Entry, without_rule_citations};

/// A block header in the shipped file, live or commented out — `[limits]`,
/// `#[db.main]`, `#[[server.mount]]` — as the header is written, and the dotted
/// path under it.
fn header(line: &str) -> Option<(&str, &str)> {
    let written = line.strip_prefix('#').unwrap_or(line).trim_start();
    let inner = written.strip_prefix('[')?.strip_suffix(']')?;
    let inner = inner.strip_prefix('[').unwrap_or(inner);
    let inner = inner.strip_suffix(']').unwrap_or(inner);
    let legal = |c: char| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-');
    (!inner.is_empty() && inner.chars().all(legal)).then_some((written, inner))
}

/// The key a setting line spells, live or commented out: `#max = 16 # default`
/// spells `max`. A comment is prose unless its first character begins a key.
fn setting(line: &str) -> Option<&str> {
    let rest = line.strip_prefix('#').unwrap_or(line);
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')))
        .unwrap_or(rest.len());
    let key = &rest[..end];
    let segment = |part: &str| part.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_');
    (key.split('.').all(segment) && rest[end..].trim_start().starts_with('=')).then_some(key)
}

/// One line per key the shipped configuration file spells, under the header it
/// sits below, carded by the comment block directly above it. A run of keys
/// with no comment between them shares the block above the first of them; a
/// blank line or a header ends the block, and so does the next comment after a
/// key.
pub(super) fn config_keys() -> Vec<Entry> {
    let mut out = Vec::new();
    let mut block = ("", "");
    let mut prose: Vec<&str> = Vec::new();
    let mut after_key = false;
    for line in nvs_config::default_file().lines() {
        let line = line.trim();
        let was_after_key = std::mem::replace(&mut after_key, false);
        if line.is_empty() {
            prose.clear();
        } else if let Some(found) = header(line) {
            block = found;
            prose.clear();
        } else if let Some(key) = setting(line) {
            after_key = true;
            let (written, dotted) = block;
            let (symbol, also) = if dotted.is_empty() {
                (key.to_owned(), None)
            } else {
                (format!("{written} {key}"), Some(format!("{dotted}.{key}")))
            };
            // The comment is joined into one paragraph before the citations
            // come out, so a citation the file wrapped onto a line of its own
            // leaves no stray full stop behind.
            let mut body = without_rule_citations(&prose.join(" "));
            if !body.is_empty() {
                body.push_str("\n\n");
            }
            body.push_str(line.strip_prefix('#').unwrap_or(line));
            let mut entry = Entry::new(symbol.clone(), format!("config: {symbol}"));
            entry.also = also;
            entry.body = body;
            out.push(entry);
        } else if let Some(comment) = line.strip_prefix('#') {
            if was_after_key {
                prose.clear();
            }
            prose.push(comment.trim_start_matches('#').trim());
        } else {
            prose.clear();
        }
    }
    out
}

/// One line per command and subcommand `nvs` parses, each followed by one per
/// flag it takes; the root's own flags come first, as `nvs --config`.
pub(super) fn commands() -> Vec<Entry> {
    let mut out = Vec::new();
    walk(&crate::Cli::command(), "nvs", &mut out);
    out
}

fn walk(command: &clap::Command, path: &str, out: &mut Vec<Entry>) {
    for arg in command.get_arguments() {
        if arg.is_hide_set() || arg.is_positional() {
            continue;
        }
        let name = match (arg.get_long(), arg.get_short()) {
            (Some(long), _) => format!("--{long}"),
            (None, Some(short)) => format!("-{short}"),
            (None, None) => continue,
        };
        let symbol = format!("{path} {name}");
        let mut entry = Entry::new(symbol.clone(), format!("flag: {symbol}"));
        entry.body = arg
            .get_long_help()
            .or_else(|| arg.get_help())
            .map(ToString::to_string)
            .unwrap_or_default();
        out.push(entry);
    }
    for sub in command.get_subcommands() {
        if sub.is_hide_set() {
            continue;
        }
        let symbol = format!("{path} {}", sub.get_name());
        let mut entry = Entry::new(symbol.clone(), format!("command: {symbol}"));
        entry.body = sub
            .get_long_about()
            .or_else(|| sub.get_about())
            .map(ToString::to_string)
            .unwrap_or_default();
        out.push(entry);
        walk(sub, &symbol, out);
    }
}

/// One line per diagnostic code, in the order the diagnostics crate declares
/// them.
pub(super) fn codes() -> Vec<Entry> {
    nvs_diagnostics::code::ALL
        .iter()
        .map(|&code| {
            let card = nvs_diagnostics::code::card(code).unwrap_or_default();
            let mut line = format!("code: {code}");
            if !card.is_empty() {
                line.push_str("  ");
                line.push_str(first_sentence(card));
            }
            let mut entry = Entry::new(code.as_str().to_owned(), line);
            entry.body = card.to_owned();
            entry
        })
        .collect()
}

/// `card` up to and including the first full stop that ends a sentence — one
/// followed by whitespace or by nothing — so a `.` inside a name such as
/// `nvs.toml` does not cut it short.
fn first_sentence(card: &str) -> &str {
    card.match_indices('.')
        .map(|(at, _)| at + 1)
        .find(|&end| card[end..].chars().next().is_none_or(char::is_whitespace))
        .map_or(card, |end| &card[..end])
}

#[cfg(test)]
mod tests {
    use super::super::{entries, show_target};
    use clap::CommandFactory as _;

    /// Every name in `table` has a line of `kind` on the index, and every line
    /// of that kind names one of them: a correspondence, never a count.
    fn corresponds(kind: &str, table: &[String]) {
        let document = crate::meta::document();
        let printed: Vec<String> = entries(&document)
            .iter()
            .filter_map(|entry| entry.line.strip_prefix(kind))
            .filter_map(|rest| rest.split("  ").next().map(str::to_owned))
            .collect();
        assert!(!table.is_empty(), "the {kind} table is populated");
        let missing: Vec<&String> = table
            .iter()
            .filter(|name| !printed.contains(name))
            .collect();
        assert!(missing.is_empty(), "{kind} names with no line: {missing:?}");
        let invented: Vec<&String> = printed
            .iter()
            .filter(|name| !table.contains(name))
            .collect();
        assert!(
            invented.is_empty(),
            "{kind} lines naming nothing: {invented:?}"
        );
        for name in table {
            assert!(
                show_target(&document, name).is_some(),
                "`show` reaches `{name}` by its exact name"
            );
        }
    }

    /// Every configuration key, command, flag and diagnostic code is an index
    /// entry that `show` reaches by name
    /// (`rule:tooling/the-index-names-every-key-command-and-code`). Each table
    /// is read here without the renderer: the codes out of the diagnostics
    /// crate's own source, the keys from the shipped file's lines, the commands
    /// and flags from the parser.
    #[test]
    fn every_key_command_flag_and_code_is_an_index_entry() {
        let source = include_str!("../../../nvs-diagnostics/src/lib.rs");
        let codes: Vec<String> = source
            .split("Code::new(\"")
            .skip(1)
            .filter_map(|rest| rest.split_once('"').map(|(code, _)| code.to_owned()))
            .filter(|code| code.len() == 5)
            .collect();
        corresponds("code: ", &codes);

        let mut keys = Vec::new();
        let mut block = String::new();
        for line in nvs_config::default_file().lines().map(str::trim) {
            if let Some((written, _)) = super::header(line) {
                block = written.to_owned();
            } else if let Some(key) = super::setting(line) {
                keys.push(if block.is_empty() {
                    key.to_owned()
                } else {
                    format!("{block} {key}")
                });
            }
        }
        corresponds("config: ", &keys);

        let mut commands = Vec::new();
        let mut flags = Vec::new();
        let mut pending = vec![(crate::Cli::command(), "nvs".to_owned())];
        while let Some((command, path)) = pending.pop() {
            for arg in command.get_arguments().filter(|arg| !arg.is_hide_set()) {
                if let Some(long) = arg.get_long() {
                    flags.push(format!("{path} --{long}"));
                } else if let Some(short) = arg.get_short() {
                    flags.push(format!("{path} -{short}"));
                }
            }
            for sub in command.get_subcommands().filter(|sub| !sub.is_hide_set()) {
                let path = format!("{path} {}", sub.get_name());
                commands.push(path.clone());
                pending.push((sub.clone(), path));
            }
        }
        corresponds("command: ", &commands);
        corresponds("flag: ", &flags);
    }

    /// A key is reached by the dotted name a program and an error message write
    /// it with, and `find` on a flag's own spelling lands on the flag.
    #[test]
    fn a_key_is_reached_by_its_dotted_name_and_a_flag_by_its_spelling() {
        let document = crate::meta::document();
        let key = show_target(&document, "server.max_in_flight").expect("the dotted key resolves");
        assert_eq!(key.line, "config: [server] max_in_flight");
        assert!(
            key.body.contains("max_in_flight ="),
            "the card writes the key's line: {}",
            key.body
        );
        assert!(
            entries(&document)
                .iter()
                .any(|entry| entry.line == "flag: nvs serve --port"
                    && entry.named(|name| name.contains("--port"))),
            "`find --port` lands on the flag"
        );
    }

    /// A code with a card has its first sentence on the index line and the
    /// whole card under it on `show`; a code still owing one is its line alone
    /// (`rule:tooling/a-diagnostic-code-carries-its-card`).
    #[test]
    fn a_codes_line_carries_its_cards_first_sentence_and_show_the_whole_card() {
        let document = crate::meta::document();
        let carded = nvs_diagnostics::code::E_BAD_DURATION_LITERAL;
        let card = nvs_diagnostics::code::card(carded).expect("the lexer's codes carry cards");
        let entry = show_target(&document, "E0007").expect("the code resolves");
        assert_eq!(
            entry.line,
            "code: E0007  This duration is not written correctly, or it is too long to store."
        );
        assert_eq!(entry.body, card);

        let owing = nvs_diagnostics::code::ALL
            .iter()
            .find(|&&code| nvs_diagnostics::code::card(code).is_none());
        if let Some(owing) = owing {
            let entry = show_target(&document, owing.as_str()).expect("the code resolves");
            assert_eq!(entry.line, format!("code: {owing}"));
            assert!(entry.body.is_empty());
        }
        assert_eq!(
            super::first_sentence("Set `nvs.toml`. Then run."),
            "Set `nvs.toml`."
        );
        assert_eq!(super::first_sentence("No full stop"), "No full stop");
    }
}
