//! `nvs/directives` against the default file it reads and the parser that file
//! is held to (`crate::directives`'s module doc).

use lsp_types::{CompletionItem, CompletionTextEdit, Position};
use nvs_config::Config;
use nvs_diagnostics::{PositionEncoding, SourceMap};
use nvs_lsp::directives::{NAMED, complete, table};

/// The labels offered at `cursor` (a `|`) in `text`.
fn labels(text: &str) -> Vec<String> {
    items(text).into_iter().map(|item| item.label).collect()
}

fn items(text: &str) -> Vec<CompletionItem> {
    let at = text.find('|').expect("the case marks its cursor");
    let before = &text[..at];
    let line = u32::try_from(before.matches('\n').count()).expect("a short case");
    let column = before.len() - before.rfind('\n').map_or(0, |n| n + 1);
    let character = u32::try_from(column).expect("a short line");
    let text = text.replacen('|', "", 1);
    complete(&text, Position { line, character }, PositionEncoding::Utf16)
}

fn header(name: &str) -> String {
    if table().headers.iter().any(|h| h.name == name && h.array) {
        format!("[[{name}]]")
    } else {
        format!("[{name}]")
    }
}

/// Whether `text` deserializes as a whole `nvs.toml`.
fn parses(text: &str) -> Result<(), String> {
    let mut sources = SourceMap::new();
    let (_, parsed) = nvs_config::file::parse::<Config>(&mut sources, "nvs.toml", text);
    parsed.map(|_| ()).map_err(|err| err.message)
}

/// The array blocks, under which a key cannot be written alone: an entry of
/// one needs the entry's own header first.
fn under_an_array(block: &str) -> bool {
    table()
        .headers
        .iter()
        .any(|h| h.array && (block == h.name || block.starts_with(&format!("{}.", h.name))))
}

#[test]
fn every_setting_line_of_the_default_file_is_an_entry() {
    let lines = nvs_config::default_file()
        .lines()
        .filter(|line| {
            line.strip_prefix('#').is_some_and(|rest| {
                rest.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_')
                    && rest.contains('=')
            })
        })
        .count();
    assert_eq!(table().entries.len(), lines);
    assert!(
        table().entries.iter().all(|entry| !entry.doc.is_empty()),
        "a key with no comment above it: {:?}",
        table().entries.iter().find(|entry| entry.doc.is_empty())
    );
}

#[test]
fn each_block_offers_exactly_its_own_keys() {
    for header in &table().headers {
        let expected: Vec<&str> = table()
            .entries
            .iter()
            .filter(|entry| entry.block == header.name)
            .map(|entry| entry.key)
            .collect();
        let offered = labels(&format!("{}\n|\n", self::header(header.name)));
        assert_eq!(offered, expected, "under [{}]", header.name);
    }
}

#[test]
fn every_offered_key_parses_under_its_block() {
    // The other half of the check `bun nv directives --check-template` makes:
    // that one fails on a key the parser has and the file lacks, and this one
    // on a key the file offers and the parser refuses. `[db.pool]`'s keys are
    // refused when written there, and the default file says so above them.
    for entry in &table().entries {
        if under_an_array(entry.block) || entry.block == "db.pool" {
            continue;
        }
        let text = format!("[{}]\n{} = {}\n", entry.block, entry.key, entry.value);
        parses(&text).unwrap_or_else(|err| panic!("{text}was refused: {err}"));
    }
}

#[test]
fn a_block_the_operator_names_offers_the_example_blocks_keys() {
    for (prefix, example) in NAMED {
        let block = format!("{prefix}.{example}");
        let expected: Vec<&str> = table()
            .entries
            .iter()
            .filter(|entry| entry.block == block)
            .map(|entry| entry.key)
            .collect();
        assert!(!expected.is_empty(), "the default file writes no [{block}]");
        assert_eq!(labels(&format!("[{prefix}.reports]\n|\n")), expected);
        for entry in table().entries.iter().filter(|entry| entry.block == block) {
            let text = format!("[{prefix}.reports]\n{} = {}\n", entry.key, entry.value);
            parses(&text).unwrap_or_else(|err| panic!("{text}was refused: {err}"));
        }
    }
    assert!(labels("[db.reports.pool]\n|\n").contains(&"max".to_owned()));
}

#[test]
fn a_key_the_section_already_sets_is_not_offered_again() {
    let offered = labels("[limits]\nmemory = \"1G\"\n|\nwall_time = \"5s\"\n[mode]\n");
    assert!(offered.contains(&"cpu_time".to_owned()));
    assert!(!offered.contains(&"memory".to_owned()));
    assert!(!offered.contains(&"wall_time".to_owned()));
}

#[test]
fn a_partial_key_is_replaced_by_the_whole_setting() {
    let found = items("[limits]\ncpu|\n");
    let item = found
        .iter()
        .find(|item| item.label == "cpu_time")
        .expect("cpu_time is offered");
    let Some(CompletionTextEdit::Edit(edit)) = &item.text_edit else {
        panic!("no edit on {item:?}");
    };
    assert_eq!(
        (edit.range.start.character, edit.range.end.character),
        (0, 3)
    );
    assert_eq!(edit.new_text, "cpu_time = ${1:\"30s\"}");
    assert!(
        item.detail
            .as_deref()
            .is_some_and(|detail| detail.contains("default"))
    );
}

#[test]
fn headers_are_offered_after_a_bracket() {
    let tables = labels("[|\n");
    assert!(tables.contains(&"limits".to_owned()));
    assert!(!tables.contains(&"app".to_owned()));
    let arrays = labels("[[|\n");
    assert!(arrays.contains(&"app".to_owned()));
    assert!(!arrays.contains(&"limits".to_owned()));
}

#[test]
fn nothing_is_offered_in_a_comment_after_an_equals_or_above_every_header() {
    assert!(labels("[limits]\n# cpu|\n").is_empty());
    assert!(labels("[limits]\nmemory = |\n").is_empty());
    assert!(labels("|\n[limits]\n").is_empty());
    assert!(labels("[nothing.here]\n|\n").is_empty());
}
