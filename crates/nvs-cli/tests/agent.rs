//! `nvs agent`, driven the way a coding agent drives it — through the built
//! binary, because `rule:tooling/an-agent-asks-the-binary`'s contract is what
//! the *command* prints, not what a function returns.
//!
//! The correspondence cases here are member-for-member against `nvs meta
//! --json` rather than against a count, which is what
//! `rule:tooling/the-index-is-one-line-per-member` asks for: completeness is the
//! property the index exists to have, and a count passes over a member swapped
//! for another.

use std::path::{Path, PathBuf};
use std::process::Command;

/// `nvs agent <args...>`, as `(stdout, stderr, success)`.
fn agent(args: &[&str]) -> (String, String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("agent")
        .args(args)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8(out.stdout).expect("the output is UTF-8"),
        String::from_utf8(out.stderr).expect("the output is UTF-8"),
        out.status.success(),
    )
}

/// The registry document the index is rendered from.
fn document() -> serde_json::Value {
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(["meta", "--json"])
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    serde_json::from_slice(&out.stdout).expect("the document is JSON")
}

/// The symbol a line carries: everything before its first `(`, `<` or space,
/// except on a line that opens with the kind of name it is. There the symbol is
/// the rest of the line after `config: `, `command: ` or `flag: `, and the code
/// after `code: `. That is the whole parser a consumer needs to get from a line
/// back to `show`.
fn symbol_of(line: &str) -> &str {
    for kind in ["config: ", "command: ", "flag: "] {
        if let Some(rest) = line.strip_prefix(kind) {
            return rest;
        }
    }
    if let Some(rest) = line.strip_prefix("code: ") {
        return rest.split(' ').next().unwrap_or(rest);
    }
    let end = line.find(['(', '<', ' ']).unwrap_or(line.len());
    &line[..end]
}

/// Every line of `nvs agent index`.
fn index() -> Vec<String> {
    let (out, _, ok) = agent(&["index"]);
    assert!(ok, "`nvs agent index` succeeds");
    out.lines().map(str::to_owned).collect()
}

/// Everything `nvs agent primer` printed.
fn primer() -> String {
    let (out, _, ok) = agent(&["primer"]);
    assert!(ok, "`nvs agent primer` succeeds");
    out
}

/// One chapter of the reference, read from the tree the binary embedded it
/// from — which is what makes the marking test a guard rather than a copy of
/// what the marking happens to be today.
fn chapter(relative: &str) -> String {
    let path = nvs_repo::path(&format!("docs/reference/{relative}"));
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{} is readable", path.display()))
}

/// The five parts `rule:tooling/a-primer-claim-is-executed` fixes, each
/// recognized by something only the section that carries it says.
// covers: tools:agents/nvs-agent-primer
#[test]
fn the_primer_carries_the_lookup_protocol_the_worked_program_and_the_refusal_table() {
    let primer = primer();
    for wanted in [
        "nvs agent find <query>",
        "So the loop is three calls and a check",
        "```nvs",
        "| `<?php` |",
        "| `function __construct(…)` |",
        "## The chapters",
        "- **programs** —",
        "- **php-differences** —",
    ] {
        assert!(primer.contains(wanted), "the primer carries `{wanted}`");
    }
    let classes = document()["classes"].as_array().expect("`classes`").len();
    assert!(
        primer.contains(&format!("{classes} `Core` classes")),
        "the primer counts the registry it renders beside the chapters"
    );
}

/// The three shapes the investigation behind `docs/decisions/0167.md` caught an
/// agent guessing wrong, all in one program that runs.
// covers: tools:agents/nvs-agent-primer
#[test]
fn the_worked_program_shows_a_typed_local_a_foreach_binding_and_an_options_bag() {
    let primer = primer();
    for wanted in [
        "array<string> $amounts = [\"3\", \"11\", \"7\"];",
        "Core\\Arr::sort($amounts, {order: Core\\Order::Asc})",
        "foreach ($ordered as string $each) {",
        "($each as int)",
        "```output",
    ] {
        assert!(
            primer.contains(wanted),
            "the worked program shows `{wanted}`"
        );
    }
}

/// A program that reads a file does not run without a grant, and the primer is
/// where an agent that has read nothing else learns it.
// covers: tools:agents/nvs-agent-primer
#[test]
fn the_primer_names_the_capability_model_and_the_smallest_grant() {
    let primer = primer();
    for wanted in [
        "default for every one of them is **denied**",
        "`fs.read`",
        "[capabilities.fs]",
        "read = [\"data\"]",
    ] {
        assert!(
            primer.contains(wanted),
            "the capability model states `{wanted}`"
        );
    }
}

/// A section is in the primer because it is marked and for no other reason, so
/// the chapter that carries the capability model contributes that section and
/// none of its neighbours.
// covers: tools:agents/nvs-agent-primer
#[test]
fn an_unmarked_chapter_section_is_not_lifted_into_the_primer() {
    let primer = primer();
    let chapter = chapter("tools/20-config.md");
    let lines: Vec<&str> = chapter.lines().collect();
    let mut marked = 0;

    for (i, line) in lines.iter().enumerate() {
        let Some(title) = line.strip_prefix("# ") else {
            continue;
        };
        let heading = format!("## {title}");
        if i > 0 && lines[i - 1].trim() == "<!-- primer -->" {
            marked += 1;
            assert!(
                primer.contains(&heading),
                "`{title}` is marked, so it is lifted"
            );
        } else {
            assert!(
                !primer.contains(&heading),
                "`{title}` is not marked, so nothing lifts it"
            );
        }
    }

    assert!(marked > 0, "the configuration chapter marks a section");
    assert!(
        !primer.contains("<!-- primer -->"),
        "the marker selects a section and is not part of it"
    );
}

/// Member for member against the document: one line each, in one direction and
/// then the other, so neither a dropped member nor an invented one passes.
#[test]
fn the_index_has_exactly_one_line_for_every_registered_member() {
    let document = document();
    let mut registered = Vec::new();
    for class in document["classes"].as_array().expect("`classes`") {
        let class_name = class["name"].as_str().expect("a class names itself");
        for member in class["members"].as_array().expect("`members`") {
            let name = member["name"].as_str().expect("a member names itself");
            registered.push(format!("{class_name}::{name}"));
        }
    }

    let printed: Vec<String> = index()
        .iter()
        .map(|line| symbol_of(line).to_owned())
        .filter(|symbol| symbol.contains("::"))
        .collect();

    let missing: Vec<&String> = registered
        .iter()
        .filter(|symbol| !printed.contains(symbol))
        .collect();
    assert!(missing.is_empty(), "members with no line: {missing:?}");

    let invented: Vec<&String> = printed
        .iter()
        .filter(|symbol| !registered.contains(symbol))
        .collect();
    assert!(invented.is_empty(), "lines naming no member: {invented:?}");

    let mut sorted = printed.clone();
    sorted.sort();
    let before = sorted.len();
    sorted.dedup();
    assert_eq!(before, sorted.len(), "no member has two lines");
}

/// The protocol is `find` a name, then `show` its card, so a line that `show`
/// cannot resolve is a dead end an agent has no way out of. Every line, not a
/// sample: the index is the complete list or it is not worth greping.
///
/// That is one process per line, which is the cost of asserting the surface
/// rather than the function behind it, so the lines are shared out across the
/// cores instead of the run being shortened.
#[test]
fn every_index_line_resolves_through_show() {
    let lines = index();
    let lanes = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);
    let chunk = lines.len().div_ceil(lanes).max(1);

    let unresolved: Vec<String> = std::thread::scope(|scope| {
        let workers: Vec<_> = lines
            .chunks(chunk)
            .map(|lines| {
                scope.spawn(move || {
                    let mut unresolved = Vec::new();
                    for line in lines {
                        let symbol = symbol_of(line);
                        let (out, _, ok) = agent(&["show", symbol]);
                        if !ok || out.is_empty() {
                            unresolved.push(symbol.to_owned());
                        }
                    }
                    unresolved
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("a lane finishes"))
            .collect()
    });

    assert!(
        unresolved.is_empty(),
        "index lines `show` does not resolve: {unresolved:?}"
    );
}

/// The capability is joined from the document's own roster at render time, so a
/// gated member wears it and an ungated one wears nothing — an empty bracket
/// would read as a gate whose name went missing.
// covers: tools:agents/nvs-agent
#[test]
fn a_gated_member_renders_its_capability_and_an_ungated_one_renders_none() {
    let lines = index();
    let line = |symbol: &str| {
        lines
            .iter()
            .find(|line| symbol_of(line) == symbol)
            .unwrap_or_else(|| panic!("{symbol} has a line"))
            .clone()
    };

    assert_eq!(
        line(r"Core\IO::read"),
        r"Core\IO::read(string $path): string  [fs.read]"
    );
    // `stdin` is a `None` row rather than an absent one, so this is the table
    // saying "needs nothing" and not the table having forgotten it.
    assert_eq!(line(r"Core\IO::stdin"), r"Core\IO::stdin(): tainted string");
}

/// The rosters beside the members are on the index too, because a name an agent
/// cannot find is a name it concludes does not exist — and `RuntimeError` is a
/// name it will reach for while writing a `catch`.
#[test]
fn the_index_names_every_enum_exception_and_attribute_beside_the_members() {
    let document = document();
    let printed: Vec<String> = index()
        .iter()
        .map(|line| symbol_of(line).to_owned())
        .collect();

    let mut wanted = Vec::new();
    for roster in ["enums", "exceptions"] {
        for row in document[roster].as_array().expect("a roster") {
            wanted.push(row["name"].as_str().expect("a row names itself").to_owned());
        }
    }
    for attribute in document["attributes"].as_array().expect("`attributes`") {
        wanted.push(
            attribute
                .as_str()
                .expect("an attribute is a name")
                .to_owned(),
        );
    }
    assert!(!wanted.is_empty(), "the rosters are populated");

    let missing: Vec<&String> = wanted
        .iter()
        .filter(|name| !printed.contains(name))
        .collect();
    assert!(missing.is_empty(), "rosters with no line: {missing:?}");
}

/// The query is written as the language spells it, backslash and all. This is
/// the whole reason `find` is a command: `grep 'Core\IO'` loses the backslash to
/// the shell, and the empty result reads as "no such class".
// covers: tools:agents/nvs-agent
#[test]
fn find_matches_a_backslashed_class_name_written_literally() {
    let (out, _, ok) = agent(&["find", r"Core\IO::read"]);
    assert!(ok, "`nvs agent find` succeeds");
    let lines: Vec<&str> = out.lines().collect();

    assert!(
        lines.contains(&r"Core\IO::read(string $path): string  [fs.read]"),
        "the exact member is among {lines:?}"
    );
    for line in &lines {
        assert!(
            symbol_of(line).contains(r"Core\IO::read"),
            "{line} matched the query literally"
        );
    }
}

/// Matching is over the symbol and without case, so a half-remembered name
/// still lands. It is the symbol and not the whole line, or a query naming a
/// type answers with every member that returns one.
#[test]
fn find_is_case_insensitive_over_the_class_and_member_name() {
    let (written, _, _) = agent(&["find", r"Core\Str::index"]);
    let (shouted, _, _) = agent(&["find", r"CORE\STR::INDEX"]);
    let (muttered, _, _) = agent(&["find", r"core\str::index"]);

    assert!(
        written.contains(r"Core\Str::indexOf"),
        "the member is found as written: {written}"
    );
    assert_eq!(shouted, written, "case does not change the answer");
    assert_eq!(muttered, written, "case does not change the answer");
}

/// `show` was asked for one specific thing, so it fails rather than printing
/// nothing — and a wrong guess is usually a near miss, which is what makes the
/// nearest matches the useful half of the refusal.
// covers: tools:agents/nvs-agent
#[test]
fn show_on_an_unknown_symbol_exits_non_zero_naming_the_nearest_matches() {
    let (out, err, ok) = agent(&["show", r"Core\Str::lenght"]);
    assert!(!ok, "an unknown symbol is a failure");
    assert!(out.is_empty(), "nothing is printed as if it were a card");
    assert!(
        err.contains(r"Core\Str::lenght"),
        "the refusal names what was asked for: {err}"
    );
    assert!(
        err.contains(r"Core\Str::length"),
        "the refusal names the symbol meant: {err}"
    );
}

/// A keyword is in the grammar and not in the registry, so no member is named for
/// it. The heading that documents it is on the index instead, and the symbol its
/// line opens with is what `show` prints the section from — the same two calls
/// that reach a member.
// covers: tools:agents/nvs-agent
#[test]
fn a_keyword_no_member_is_named_for_is_found_by_its_heading_and_shown_as_its_section() {
    let (out, _, ok) = agent(&["find", "autoload"]);
    assert!(ok, "`nvs agent find` succeeds");
    let line = out
        .lines()
        .find(|line| line.starts_with("programs#"))
        .unwrap_or_else(|| panic!("a heading of `programs` is among: {out}"));

    let (card, _, ok) = agent(&["show", symbol_of(line)]);
    assert!(ok, "the section's symbol resolves");
    assert!(
        card.contains("autoload 'App' from './src';"),
        "the section is printed as the chapter has it: {card}"
    );
    assert!(
        !card.contains("<!-- primer -->"),
        "the primer's marker is not part of a section"
    );
    assert!(
        !card.contains("# Ending a program"),
        "the section stops at the next heading of its level: {card}"
    );
}

/// A chapter's own name resolves too, and answers with where to go next instead
/// of with the whole chapter: its summary, and the line of every heading in it.
#[test]
fn show_on_a_chapter_lists_the_sections_the_index_has_for_it() {
    let (card, _, ok) = agent(&["show", "programs"]);
    assert!(ok, "a chapter's id resolves");

    let sections: Vec<String> = index()
        .into_iter()
        .filter(|line| line.starts_with("programs#"))
        .collect();
    assert!(!sections.is_empty(), "the chapter has headings");
    for section in &sections {
        assert!(card.contains(section.as_str()), "{section} is on the card");
    }
}

/// A class has a line of its own, so the class name alone resolves and lists
/// its members, and a class with no members — the carrier `Core\Html\Markup` —
/// is on the index at all rather than only ever a type in somebody else's
/// signature.
#[test]
fn a_class_name_resolves_to_a_card_listing_its_members() {
    let (card, _, ok) = agent(&["show", r"Core\Html"]);
    assert!(ok, "a class name resolves");
    assert!(
        card.starts_with("Core\\Html  class: "),
        "the card opens with the class line: {card}"
    );
    assert!(
        card.contains(r"  Core\Html::escape(string $text): Core\Html\Markup"),
        "a member's index line is on the class card: {card}"
    );

    let (card, _, ok) = agent(&["show", r"Core\Html\Markup"]);
    assert!(ok, "a memberless class resolves too");
    assert!(
        card.starts_with("Core\\Html\\Markup  class\n"),
        "a memberless class opens with its class line: {card}"
    );
}

/// A class that carries a `ClassDoc` prints that prose between its line and its
/// members, because the card is the class's help in the binary
/// (`rule:testing/feature-proofs`) and a card nothing prints helps nobody.
#[test]
fn a_class_card_prints_its_prose_above_its_members() {
    let (card, _, ok) = agent(&["show", r"Core\Process\Result"]);
    assert!(ok, "a class name resolves");
    let prose = card
        .find("The result of a program that has ended.")
        .expect("the class's own card is printed");
    let members = card.find("\nmembers:\n").expect("the members are listed");
    assert!(
        prose < members,
        "the prose comes before the members: {card}"
    );
}

/// The markup literal has no member to be named for, so the word `html` has to
/// reach it through a heading — and through the class whose value it makes —
/// or an agent concludes from six member lines that no literal exists.
#[test]
fn find_html_reaches_the_literal_and_the_markup_class() {
    let (out, _, ok) = agent(&["find", "html"]);
    assert!(ok, "`nvs agent find` succeeds");
    let lines: Vec<&str> = out.lines().collect();
    assert!(
        lines.contains(&r"Core\Html\Markup  class"),
        "the carrier is among {lines:?}"
    );
    let section = lines
        .iter()
        .find(|line| line.starts_with("types#") && line.contains("template literal"))
        .unwrap_or_else(|| panic!("the literal's heading is among {lines:?}"));

    let (card, _, ok) = agent(&["show", symbol_of(section)]);
    assert!(ok, "the section's symbol resolves");
    assert!(
        card.contains("begin with a variable") && card.contains("<?= expr ?>"),
        "both hole grammars are stated at the literal: {card}"
    );
}

/// A chapter cites this repository's rules in parentheses, and the agent that
/// reads a section through the binary has no way to resolve one, so the shown
/// section carries no such citation.
#[test]
fn a_shown_section_carries_no_rule_citation() {
    let (card, _, ok) = agent(&["show", "expressions#refused-in-expression-position"]);
    assert!(ok, "the section resolves");
    assert!(
        !card.contains("(`rule:"),
        "no parenthesised citation survives: {card}"
    );
    assert!(
        card.contains("never a command line — and the character itself"),
        "the sentence reads on across where the citation was: {card}"
    );
}

/// The chapter list is written by hand, so a chapter added to the reference and
/// not to the list is one the binary cannot show while the primer says its map
/// is all of them. Every file of the two reference directories has a line.
#[test]
fn every_chapter_of_the_reference_has_a_line_on_the_index() {
    let printed: Vec<String> = index()
        .iter()
        .map(|line| symbol_of(line).to_owned())
        .collect();

    let mut missing = Vec::new();
    for directory in ["lang", "tools"] {
        let path = nvs_repo::path(&format!("docs/reference/{directory}"));
        for file in std::fs::read_dir(&path).expect("the directory is readable") {
            let name = file.expect("an entry").file_name();
            let name = name.to_str().expect("a chapter's name is UTF-8");
            if !name.ends_with(".md") {
                continue;
            }
            let text = chapter(&format!("{directory}/{name}"));
            let id = text
                .lines()
                .find_map(|line| line.strip_prefix("id:"))
                .unwrap_or_else(|| panic!("{name} states its `id`"))
                .trim()
                .to_owned();
            if !printed.contains(&id) {
                missing.push(id);
            }
        }
    }
    assert!(missing.is_empty(), "chapters with no line: {missing:?}");
}

/// `show` compares without case, so two lines whose symbols differ only by case
/// would leave the second unreachable — which is what a chapter named for an
/// exception or an attribute would be.
#[test]
fn no_two_index_lines_open_with_the_same_symbol() {
    let mut symbols: Vec<String> = index()
        .iter()
        .map(|line| symbol_of(line).to_lowercase())
        .collect();
    symbols.sort();
    let twice: Vec<&String> = symbols
        .windows(2)
        .filter(|pair| pair[0] == pair[1])
        .map(|pair| &pair[0])
        .collect();
    assert!(twice.is_empty(), "symbols on two lines: {twice:?}");
}

/// Nothing on standard output is still the answer, so a consumer reading the
/// output sees what it always saw. What the silence covers is said beside it.
// covers: tools:agents/nvs-agent
#[test]
fn find_with_no_match_prints_nothing_and_says_what_was_searched() {
    let (out, err, ok) = agent(&["find", "strlen"]);
    assert!(ok, "an empty result is an answer, not a failure");
    assert!(
        out.is_empty(),
        "nothing is printed as if it were a line: {out}"
    );
    assert!(
        err.contains("chapter"),
        "the message names the chapters as the next place to look: {err}"
    );
    assert!(
        err.contains("a keyword may be written under a heading that does not name it"),
        "the message says why an empty result is not an absence: {err}"
    );
}

/// `nvs agent <args...> --json`, parsed, with whether it succeeded.
fn agent_json(args: &[&str]) -> (serde_json::Value, bool) {
    let mut args = args.to_vec();
    args.push("--json");
    let (out, _, ok) = agent(&args);
    let document: serde_json::Value = serde_json::from_str(&out)
        .unwrap_or_else(|error| panic!("`nvs agent {}` prints JSON: {error}", args.join(" ")));
    assert_eq!(
        document["schemaVersion"], 1,
        "the document carries its schema version"
    );
    (document, ok)
}

/// The records of an `index --json` or `find --json` document.
fn records(document: &serde_json::Value) -> &[serde_json::Value] {
    document["entries"]
        .as_array()
        .expect("the document has an `entries` array")
}

/// `--json` is the same index as another rendering: one record per line, in
/// the same order, each carrying that line, the symbol the line opens with and
/// a kind. Every key is present on every record, as `null` where the entry has
/// no such part.
// covers: tools:agents/nvs-agent
#[test]
fn index_json_has_one_record_for_every_line_of_the_index() {
    let lines = index();
    let (document, ok) = agent_json(&["index"]);
    assert!(ok, "`nvs agent index --json` succeeds");
    let records = records(&document);
    assert_eq!(records.len(), lines.len(), "one record per line");

    let kinds = [
        "class",
        "member",
        "enum",
        "exception",
        "attribute",
        "chapter",
        "section",
        "config",
        "command",
        "flag",
        "code",
    ];
    let keys = [
        "symbol",
        "kind",
        "line",
        "alias",
        "signature",
        "capability",
        "title",
        "summary",
    ];
    for (record, line) in records.iter().zip(&lines) {
        assert_eq!(record["line"], line.as_str(), "the record carries its line");
        assert_eq!(
            record["symbol"],
            symbol_of(line),
            "the symbol is the one the line opens with: {line}"
        );
        let kind = record["kind"].as_str().unwrap_or_default();
        assert!(kinds.contains(&kind), "a known kind: {record}");
        for key in keys {
            assert!(record.get(key).is_some(), "`{key}` is present: {record}");
        }
    }

    let read = records
        .iter()
        .find(|record| record["symbol"] == r"Core\IO::read")
        .expect(r"`Core\IO::read` has a record");
    assert_eq!(read["kind"], "member");
    assert_eq!(read["signature"], "read(string $path): string");
    assert_eq!(read["capability"], "fs.read");
    let key = records
        .iter()
        .find(|record| record["symbol"] == "[server] max_in_flight")
        .expect("the key has a record");
    assert_eq!(key["kind"], "config");
    assert_eq!(key["alias"], "server.max_in_flight");
}

/// `find --json` lists the records of the lines `find` prints, and a query
/// nothing matches is an empty list and a success, as the text form is.
// covers: tools:agents/nvs-agent
#[test]
fn find_json_lists_the_matching_records_and_an_empty_list_for_no_match() {
    let (text, _, _) = agent(&["find", r"Core\IO::read"]);
    let (document, ok) = agent_json(&["find", r"Core\IO::read"]);
    assert!(ok, "`nvs agent find --json` succeeds");
    let lines: Vec<&str> = records(&document)
        .iter()
        .map(|record| record["line"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(lines, text.lines().collect::<Vec<_>>(), "the same lines");
    assert!(!lines.is_empty(), "the query matches");

    let (document, ok) = agent_json(&["find", "strlen"]);
    assert!(ok, "an empty result is an answer, not a failure");
    assert!(records(&document).is_empty(), "no record: {document}");
}

/// `show --json` is the card with each of its parts as a key. The `line` is
/// the card's first line, and each kind carries the parts its text card prints.
// covers: tools:agents/nvs-agent
#[test]
fn show_json_gives_each_part_of_the_card_by_kind() {
    let shown = |symbol: &str| {
        let (text, _, ok) = agent(&["show", symbol]);
        assert!(ok, "`nvs agent show {symbol}` succeeds");
        let (card, ok) = agent_json(&["show", symbol]);
        assert!(ok, "`nvs agent show {symbol} --json` succeeds");
        assert_eq!(
            card["line"],
            text.lines().next().unwrap_or_default(),
            "the card's line is the text card's first line"
        );
        (card, text)
    };

    let (member, _) = shown(r"Core\IO::read");
    assert_eq!(member["kind"], "member");
    assert_eq!(member["params"][0]["name"], "path");
    assert!(member["returns"].is_string(), "{member}");
    let throws: Vec<&str> = member["throws"]
        .as_array()
        .expect("`throws` is a list")
        .iter()
        .filter_map(|error| error["error"].as_str())
        .collect();
    assert!(throws.contains(&"IOError"), "{throws:?}");

    let (class, _) = shown(r"Core\Str");
    assert_eq!(class["kind"], "class");
    assert!(
        class["members"]
            .as_array()
            .expect("`members` is a list")
            .iter()
            .any(|member| member["symbol"] == r"Core\Str::length"),
        "{class}"
    );

    let (section, _) = shown("programs#autoload-find-a-class-by-its-namespace");
    assert_eq!(section["kind"], "section");
    let body = section["text"].as_str().unwrap_or_default();
    assert!(body.contains("autoload 'App' from './src';"), "{body}");

    let (chapter, _) = shown("programs");
    assert_eq!(chapter["kind"], "chapter");
    assert!(chapter["summary"].is_string(), "{chapter}");
    assert!(
        chapter["sections"]
            .as_array()
            .expect("`sections` is a list")
            .iter()
            .any(|section| section["symbol"] == "programs#autoload-find-a-class-by-its-namespace"),
        "{chapter}"
    );

    let (key, text) = shown("server.max_in_flight");
    assert_eq!(key["kind"], "config");
    assert_eq!(key["symbol"], "[server] max_in_flight");
    let comment = key["text"].as_str().unwrap_or_default();
    assert!(
        text.contains(comment),
        "the text card prints the same comment"
    );

    let (code, text) = shown("E0621");
    assert_eq!(code["kind"], "code");
    let card = code["text"].as_str().unwrap_or_default();
    assert!(!card.is_empty() && text.contains(card), "{code}");
}

/// An unknown symbol fails under `--json` too. The document is still printed on
/// standard output, as `nvs check --json`'s is, and names what was asked for and
/// the records nearest to it.
// covers: tools:agents/nvs-agent
#[test]
fn show_json_on_an_unknown_symbol_fails_with_the_nearest_records() {
    let (document, ok) = agent_json(&["show", r"Core\Str::lenght"]);
    assert!(!ok, "an unknown symbol is a failure");
    let error = document["error"].as_str().unwrap_or_default();
    assert!(error.contains(r"Core\Str::lenght"), "{document}");
    assert!(
        document["nearest"]
            .as_array()
            .expect("`nearest` is a list")
            .iter()
            .any(|record| record["symbol"] == r"Core\Str::length"),
        "{document}"
    );
}

/// A syntactic form is not a `Core` member, so a heading is the only way `find`
/// can reach it — and a form with no heading reads, to an agent, as a feature
/// that does not exist. Every form here is one that was, or could have been,
/// concluded absent that way; each must answer `find` with a section line.
#[test]
fn every_syntactic_form_is_reachable_through_a_heading() {
    let forms = [
        "doc-comments",
        "inout",
        "tainted",
        "secret",
        "lateinit",
        "readonly",
        "match",
        "require",
        "autoload",
        "spawn",
        "implements",
        "class-t",
        "html",
        "any-expression",
        "closures-fn",
        "enum",
        "is",
        "yield",
        "clone",
    ];
    let mut missing = Vec::new();
    for form in forms {
        let (out, _, ok) = agent(&["find", form]);
        assert!(ok, "`nvs agent find {form}` succeeds");
        if !out.lines().any(|line| line.contains("  section: ")) {
            missing.push(form);
        }
    }
    assert!(missing.is_empty(), "forms with no heading: {missing:?}");
}

/// The lines one section of the agents chapter shows as lines the binary
/// prints: what its `text` blocks hold. `section` is the heading line.
fn shown(section: &str) -> Vec<String> {
    let text = chapter("tools/50-agents.md");
    let mut lines = Vec::new();
    let mut in_section = false;
    let mut in_fence = false;
    let mut in_sample = false;
    for line in text.lines() {
        if line.starts_with("```") {
            in_sample = !in_fence && line == "```text";
            in_fence = !in_fence;
        } else if !in_fence && line.starts_with("# ") {
            in_section = line == section;
        } else if in_section && in_sample {
            lines.push(line.to_owned());
        }
    }
    lines
}

/// The section's list of headings is a copy of what the primer printed the day
/// it was written, and a copy goes stale the day a section is marked or a
/// chapter retitles one. Each heading shown is a heading of the primer, in the
/// primer's order, from its title to its chapter map, and only a `…` line passes
/// over headings.
// covers: tools:agents/nvs-agent-primer
#[test]
fn every_heading_the_chapter_shows_is_a_heading_of_the_primer_in_that_order() {
    let shown = shown("# nvs agent primer");
    assert!(shown.len() > 2, "the section shows the headings: {shown:?}");

    let primer = primer();
    let mut in_fence = false;
    let mut headings = Vec::new();
    for line in primer.lines() {
        if line.starts_with("```") {
            in_fence = !in_fence;
        } else if !in_fence && line.starts_with('#') {
            headings.push(line);
        }
    }

    let mut next = 0;
    let mut passing = false;
    for line in &shown {
        if line == "…" {
            passing = true;
            continue;
        }
        let ahead = headings[next..]
            .iter()
            .position(|heading| heading == line)
            .unwrap_or_else(|| panic!("the primer prints no heading `{line}` at that place"));
        assert!(
            passing || ahead == 0,
            "the chapter passes over `{}` with no `…`",
            headings[next]
        );
        next += ahead + 1;
        passing = false;
    }
    assert!(
        !passing && next == headings.len(),
        "the list closes on the primer's last heading, `{}`",
        headings[headings.len() - 1]
    );
}

/// The section's sample is a copy of lines the index printed the day it was
/// written, and a copy goes stale the day a signature gains a parameter or an
/// exception moves in its tree. Each line shown is a whole line of the index,
/// its symbol resolves through `show`, and the sample has one line of every
/// kind the section then describes.
// covers: tools:agents/nvs-agent
#[test]
fn every_index_line_the_chapter_shows_is_a_line_of_the_index() {
    let shown = shown("# nvs agent");
    let index = index();
    for line in &shown {
        assert!(
            index.contains(line),
            "the chapter shows a line the index does not print: {line}"
        );
        let (_, _, ok) = agent(&["show", symbol_of(line)]);
        assert!(ok, "`show` resolves the symbol `{}`", symbol_of(line));
    }
    for (kind, mark) in [
        ("a gated member", "  ["),
        ("a generic member", "<T>("),
        ("an enum", "  enum "),
        ("an exception", "  exception "),
        ("a chapter", "  chapter: "),
        ("a section", "  section: "),
    ] {
        assert!(
            shown.iter().any(|line| line.contains(mark)),
            "the sample shows no line for {kind}: {shown:?}"
        );
    }
}

/// The check that closes the loop. A program written from the PHP name is
/// refused, and the help names what the language writes for that name — as a
/// spelling `show` resolves, so the diagnostic leads back into the same two
/// calls instead of out of them. A name that has no single replacement gets no
/// such sentence, because a guess there would be a spelling that does not
/// resolve.
// covers: tools:agents/nvs-agent
#[test]
fn the_member_a_diagnostic_names_for_a_php_function_is_one_show_resolves() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("agent-check-loop");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a private directory under the target directory");
    let calls = [
        ("count", "count($sizes)"),
        ("implode", "implode(\", \", $names)"),
        ("in_array", "in_array(3, $sizes)"),
        ("strlen", "strlen($names[0])"),
        ("tally", "tally($sizes)"),
    ];
    let mut program = String::from(
        "<?nvs\narray<int> $sizes = [3, 1, 2];\narray<string> $names = [\"Ada\", \"Grace\"];\n",
    );
    for (_, call) in calls {
        program.push_str("echo ");
        program.push_str(call);
        program.push_str(", \"\\n\";\n");
    }
    std::fs::write(dir.join("main.nvs"), program).expect("the program is written");

    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(["check", "main.nvs"])
        .current_dir(&dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    let _ = std::fs::remove_dir_all(&dir);
    assert!(!out.status.success(), "a free function does not compile");
    let err = String::from_utf8(out.stderr).expect("the output is UTF-8");

    for (name, _) in calls {
        assert!(
            err.contains(&format!("`{name}` is not a function that exists")),
            "`{name}` is refused by name: {err}"
        );
        let opening = format!("help: PHP's `{name}` is `");
        let named = err
            .lines()
            .find_map(|line| line.trim_start().strip_prefix("= ")?.strip_prefix(&opening))
            .and_then(|rest| rest.split_once("` here"))
            .map(|(member, _)| member);
        if name == "tally" {
            assert_eq!(named, None, "a name PHP does not have is told no member");
            continue;
        }
        let member = named.unwrap_or_else(|| panic!("the help names a member for `{name}`: {err}"));
        let (card, _, ok) = agent(&["show", member]);
        assert!(ok, "`show` resolves `{member}`, which the help named");
        assert!(
            card.starts_with(member),
            "the card is that member's own: {card}"
        );
    }
}

/// Every `$ nvs agent …` command one section of the agents chapter shows, as
/// the arguments after `agent` and the lines the chapter prints under the
/// command. `section` is the heading line. A lone `$` is the prompt coming back
/// and ends the command above it.
fn transcript(section: &str) -> Vec<(Vec<String>, Vec<String>)> {
    let text = chapter("tools/50-agents.md");
    let mut steps: Vec<(Vec<String>, Vec<String>)> = Vec::new();
    let mut in_section = false;
    let mut in_fence = false;
    let mut in_transcript = false;
    let mut open = false;
    for line in text.lines() {
        if line.starts_with("```") {
            in_transcript = !in_fence && line == "```text";
            in_fence = !in_fence;
            open = false;
        } else if !in_fence && line.starts_with("# ") {
            in_section = line == section;
        } else if in_section && in_transcript {
            if let Some(command) = line.strip_prefix("$ nvs agent ") {
                let args = command
                    .split_whitespace()
                    .map(|word| word.trim_matches('\'').to_owned())
                    .collect();
                steps.push((args, Vec::new()));
                open = true;
            } else if line == "$" {
                open = false;
            } else if open {
                let (_, printed) = steps.last_mut().expect("a command is open");
                printed.push(line.to_owned());
            }
        }
    }
    steps
}

/// The session is a transcript, and a transcript in a chapter is a copy of what
/// the binary printed the day it was written. Replaying each command holds the
/// copy to the binary: what `find` lists, the card `show` prints, and the
/// status each call exits with. A block that closes on `…` is an excerpt, so it
/// is matched as the opening of the output.
// covers: tools:agents/a-worked-session
#[test]
fn the_worked_session_of_the_chapter_is_what_the_binary_prints() {
    let steps = transcript("# A worked session");
    let commands: Vec<&str> = steps.iter().map(|(args, _)| args[0].as_str()).collect();
    assert_eq!(
        commands,
        ["find", "find", "show", "find", "show"],
        "the session is a miss, a hit and its card, then a keyword and its section"
    );

    for (args, printed) in &steps {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let (out, _, ok) = agent(&args);
        assert!(ok, "`nvs agent {}` succeeds", args.join(" "));
        let lines: Vec<&str> = out.lines().collect();

        match printed.last().and_then(|last| last.strip_suffix('…')) {
            Some(opening) => {
                let whole = printed.len() - 1;
                assert!(
                    lines.len() > whole,
                    "`nvs agent {}` prints past the excerpt: {out}",
                    args.join(" ")
                );
                assert_eq!(
                    lines[..whole],
                    printed[..whole],
                    "the excerpt's whole lines"
                );
                assert!(
                    lines[whole].starts_with(opening.trim_end()),
                    "the excerpt stops inside this line: {}",
                    lines[whole]
                );
            }
            None => assert_eq!(
                lines,
                *printed,
                "`nvs agent {}` prints what the chapter shows",
                args.join(" ")
            ),
        }
    }
}

/// A fresh empty directory named for the test that owns it, so two tests never
/// share a working directory.
fn tree(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("nvs-agent-init-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a private directory under the target directory");
    dir
}

/// `nvs agent init <args...>`, run *in* `dir`, as `(stdout, stderr, success)`.
fn init(dir: &Path, args: &[&str]) -> (String, String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(["agent", "init"])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    (
        String::from_utf8(out.stdout).expect("the output is UTF-8"),
        String::from_utf8(out.stderr).expect("the output is UTF-8"),
        out.status.success(),
    )
}

/// Every file under `dir`, relative to it with `/` separators and sorted, so a
/// test asserts on the whole tree rather than on the files it thought to check.
fn files(dir: &Path) -> Vec<String> {
    fn walk(root: &Path, at: &Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(at).expect("the fixture directory is readable") {
            let path = entry.expect("the entry is readable").path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .expect("the entry is under the root");
                out.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

/// One of the fixture's files, as text.
fn read(dir: &Path, relative: &str) -> String {
    std::fs::read_to_string(dir.join(relative)).expect("the file this run wrote is readable")
}

/// A tree with no harness in it gets the harness-neutral pointer and nothing
/// else, because an adapter is written for a harness that is present and this
/// tree shows none.
// covers: tools:agents/nvs-agent-init
#[test]
fn init_in_an_empty_tree_writes_the_agents_stanza_and_nothing_else() {
    let dir = tree("empty");
    let (out, err, ok) = init(&dir, &[]);
    assert!(ok, "`nvs agent init` succeeds: {err}");
    assert_eq!(files(&dir), vec!["AGENTS.md".to_owned()]);

    let stanza = read(&dir, "AGENTS.md");
    for command in [
        "nvs agent primer",
        "nvs agent index",
        "nvs agent find",
        "nvs agent show",
    ] {
        assert!(stanza.contains(command), "the stanza names `{command}`");
    }
    assert!(
        stanza.contains("nvs check"),
        "the stanza names the check loop"
    );
    assert!(out.contains("AGENTS.md"), "it says what it wrote: {out}");
}

/// The stanza is a region of a file the project owns, so a second run finds its
/// own markers and leaves everything between them — and everything outside
/// them — exactly as it was.
// covers: tools:agents/nvs-agent-init
#[test]
fn init_run_twice_changes_nothing_the_first_run_wrote() {
    let dir = tree("twice");
    let theirs = "# Working on this project\n\nOur own rules, which this does not touch.\n";
    std::fs::write(dir.join("AGENTS.md"), theirs).expect("the project's own file is written");

    let (_, err, ok) = init(&dir, &[]);
    assert!(ok, "the first run succeeds: {err}");
    let after_one = read(&dir, "AGENTS.md");
    assert!(
        after_one.starts_with(theirs),
        "the stanza lands beside the project's own instructions, not instead of them"
    );

    let (_, err, ok) = init(&dir, &[]);
    assert!(ok, "the second run succeeds: {err}");
    assert_eq!(read(&dir, "AGENTS.md"), after_one, "and wrote nothing new");
    assert_eq!(files(&dir), vec!["AGENTS.md".to_owned()]);
}

/// A checkout that turned every line ending into CRLF, as Git's `core.autocrlf`
/// does on Windows, holds the same stanza and the same pointer, so a second run
/// is `up to date` and leaves the converted files as they are.
// covers: tools:agents/nvs-agent-init
#[test]
fn init_reads_a_crlf_checkout_of_what_it_wrote_as_up_to_date() {
    let dir = tree("crlf");
    std::fs::create_dir_all(dir.join(".claude")).expect("the harness's own directory");
    let (_, err, ok) = init(&dir, &[]);
    assert!(ok, "the first run succeeds: {err}");

    for relative in ["AGENTS.md", ".claude/skills/novis/SKILL.md"] {
        let crlf = read(&dir, relative).replace('\n', "\r\n");
        std::fs::write(dir.join(relative), &crlf).expect("the converted file is written");
    }
    let converted: Vec<String> = ["AGENTS.md", ".claude/skills/novis/SKILL.md"]
        .iter()
        .map(|relative| read(&dir, relative))
        .collect();

    let (out, err, ok) = init(&dir, &[]);
    assert!(ok, "a CRLF checkout is not a refusal: {err}");
    assert!(out.contains("up to date"), "it says nothing changed: {out}");
    for (relative, before) in ["AGENTS.md", ".claude/skills/novis/SKILL.md"]
        .iter()
        .zip(&converted)
    {
        assert_eq!(
            &read(&dir, relative),
            before,
            "{relative} is left as it was"
        );
    }
}

/// Whether a changed stanza is an edit or an upgrade is not something this can
/// see, so it refuses both — overwriting a reader's own sentence is the worse of
/// the two mistakes, and the refusal names the file and the way out.
// covers: tools:agents/nvs-agent-init
#[test]
fn init_refuses_to_overwrite_a_stanza_that_has_been_edited() {
    let dir = tree("edited");
    let (_, err, ok) = init(&dir, &[]);
    assert!(ok, "the first run succeeds: {err}");

    let written = read(&dir, "AGENTS.md");
    let edited = written.replace("nvs agent index", "nvs agent index  (we run this in CI)");
    assert_ne!(edited, written, "the edit landed inside the stanza");
    std::fs::write(dir.join("AGENTS.md"), &edited).expect("the edited file is written");

    let (out, err, ok) = init(&dir, &[]);
    assert!(!ok, "an edited stanza is a refusal: {out}");
    assert!(
        err.contains("AGENTS.md"),
        "the refusal names the file: {err}"
    );
    assert_eq!(read(&dir, "AGENTS.md"), edited, "and it changed nothing");
}

/// A harness is detected by the directory it already keeps in the project, so a
/// tree with `.claude/` in it gets the skill beside the neutral stanza and a
/// tree without one does not — which is the whole of the detection.
// covers: tools:agents/nvs-agent-init
#[test]
fn init_writes_the_claude_skill_when_that_harness_is_present() {
    let dir = tree("claude");
    std::fs::create_dir_all(dir.join(".claude")).expect("the harness's own directory");

    let (out, err, ok) = init(&dir, &[]);
    assert!(ok, "`nvs agent init` succeeds: {err}");
    assert_eq!(
        files(&dir),
        vec![
            ".claude/skills/novis/SKILL.md".to_owned(),
            "AGENTS.md".to_owned(),
        ]
    );
    assert!(
        out.contains(".claude/skills/novis/SKILL.md"),
        "it says what it wrote: {out}"
    );

    let (_, err, ok) = init(&dir, &[]);
    assert!(ok, "the second run succeeds: {err}");
    assert_eq!(
        files(&dir),
        vec![
            ".claude/skills/novis/SKILL.md".to_owned(),
            "AGENTS.md".to_owned(),
        ]
    );
}

/// Every pointer `--all` writes, as `(path, text)`, which is the whole set the
/// two tests below hold to the rule — asked for by the flag rather than by
/// naming the harnesses, so a row added to the table is covered the day it lands.
fn pointers(name: &str) -> Vec<(String, String)> {
    let dir = tree(name);
    let (_, err, ok) = init(&dir, &["--all"]);
    assert!(ok, "`nvs agent init --all` succeeds: {err}");
    let paths = files(&dir);
    assert!(
        paths.len() > 1,
        "`--all` writes the adapters as well as the stanza: {paths:?}"
    );
    paths
        .into_iter()
        .map(|path| {
            let text = read(&dir, &path);
            (path, text)
        })
        .collect()
}

/// A pointer says where to ask and how to check the answer, and that is the
/// entire reason it exists — so each one names all four commands and the check
/// loop, in whatever shape its harness reads.
// covers: tools:agents/nvs-agent-init
#[test]
fn every_adapter_names_the_agent_commands_and_the_check_loop() {
    for (path, text) in pointers("names-the-commands") {
        for command in [
            "nvs agent primer",
            "nvs agent index",
            "nvs agent find",
            "nvs agent show",
            "nvs check",
        ] {
            assert!(text.contains(command), "{path} names `{command}`");
        }
    }
}

/// `rule:tooling/an-adapter-carries-protocol-and-never-language`, as the thing
/// that fails when it is broken: a signature, a type name or a refusal copied
/// into a pointer is a copy that goes stale the day the member changes, and the
/// agent reading it has no way to know it is old.
// covers: tools:agents/nvs-agent-init
#[test]
fn no_adapter_contains_a_member_signature_a_type_name_or_a_refusal() {
    for (path, text) in pointers("no-language-content") {
        for needle in [
            "Core\\",
            "$",
            "RuntimeError",
            "not granted",
            "<?nvs",
            "function ",
            "public ",
        ] {
            assert!(
                !text.contains(needle),
                "{path} states a language fact: it contains `{needle}`"
            );
        }
    }
}

/// The skill is found and summarised by its front matter, so a missing key makes
/// it invisible to the harness rather than wrong — which is a failure nothing
/// else in this suite would see.
// covers: tools:agents/nvs-agent-init
#[test]
fn the_claude_skill_carries_a_name_and_a_description_in_its_front_matter() {
    let dir = tree("front-matter");
    let (_, err, ok) = init(&dir, &["--all"]);
    assert!(ok, "`nvs agent init --all` succeeds: {err}");

    let skill = read(&dir, ".claude/skills/novis/SKILL.md");
    let mut lines = skill.lines();
    assert_eq!(
        lines.next(),
        Some("---"),
        "the file opens on its front matter"
    );
    let front: Vec<&str> = lines.take_while(|line| *line != "---").collect();
    assert!(
        !front.is_empty(),
        "the front matter is closed by a second `---`"
    );

    for key in ["name", "description"] {
        let value = front
            .iter()
            .find_map(|line| line.strip_prefix(&format!("{key}: ")))
            .unwrap_or_else(|| panic!("the front matter carries `{key}`: {front:?}"));
        assert!(!value.trim().is_empty(), "`{key}` has a value");
    }
    assert!(
        front.contains(&"name: novis"),
        "the skill is named for the language: {front:?}"
    );
}

/// The section's transcript is a copy of what three runs printed the day it was
/// written. Replaying the runs in one empty tree, in the order the chapter has
/// them, holds each line to the binary.
// covers: tools:agents/nvs-agent-init
#[test]
fn the_init_transcript_of_the_chapter_is_what_the_binary_prints() {
    let steps = transcript("# nvs agent init");
    assert!(
        steps.len() > 1,
        "the section shows a first run and a second"
    );

    let dir = tree("transcript");
    for (args, printed) in &steps {
        assert_eq!(args[0], "init", "the section's transcript runs `init` only");
        let flags: Vec<&str> = args[1..].iter().map(String::as_str).collect();
        let (out, err, ok) = init(&dir, &flags);
        assert!(ok, "`nvs agent {}` succeeds: {err}", args.join(" "));
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(
            lines,
            *printed,
            "`nvs agent {}` prints what the chapter shows",
            args.join(" ")
        );
    }
}

/// Every file under `dir` with its bytes, which is what a refusal is held to
/// having left alone.
fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
    files(dir)
        .into_iter()
        .map(|path| {
            let bytes = std::fs::read(dir.join(&path)).expect("the fixture file is readable");
            (path, bytes)
        })
        .collect()
}

/// An `AGENTS.md` written to break the install: a stanza that opens and never
/// closes, one whose markers are out of order or nested, one that is empty, text
/// that is not UTF-8, and a directory where the file belongs. Each is refused by
/// a message naming the file, with a failing status and no panic, and the tree
/// is byte for byte what it was — the adapter the `.claude/` directory would
/// have earned included.
// covers: tools:agents/nvs-agent-init
#[test]
fn init_refuses_an_agents_file_it_cannot_read_as_its_own_and_changes_nothing() {
    let open = "<!-- nvs agent: written by `nvs agent init` -->";
    let close = "<!-- /nvs agent -->";
    let hostile: [(&str, Vec<u8>); 6] = [
        (
            "unclosed",
            format!("# Ours\n\n{open}\n\n## Novis\n").into_bytes(),
        ),
        ("closed-first", format!("{close}\n{open}\n").into_bytes()),
        ("empty", format!("{open}{close}").into_bytes()),
        (
            "nested",
            format!("{open}\n{open}\n{close}\n{close}\n").into_bytes(),
        ),
        (
            "long",
            format!("{open}\n{}\n{close}\n", "x".repeat(4_000_000)).into_bytes(),
        ),
        ("not-utf8", vec![0xff, 0xfe, b'#', b' ', 0x80, b'\n']),
    ];
    for (name, bytes) in hostile {
        let dir = tree(&format!("hostile-{name}"));
        std::fs::create_dir_all(dir.join(".claude")).expect("the harness's own directory");
        std::fs::write(dir.join("AGENTS.md"), &bytes).expect("the hostile file is written");
        let before = snapshot(&dir);

        let (out, err, ok) = init(&dir, &[]);
        assert!(!ok, "`{name}` is a refusal: {out}");
        assert!(
            err.contains("AGENTS.md"),
            "`{name}`: the refusal names the file: {err}"
        );
        assert!(!err.contains("panicked"), "`{name}`: {err}");
        assert_eq!(snapshot(&dir), before, "`{name}`: and it changed nothing");
    }

    let dir = tree("hostile-directory");
    std::fs::create_dir_all(dir.join("AGENTS.md")).expect("a directory where the file belongs");
    std::fs::create_dir_all(dir.join(".claude")).expect("the harness's own directory");
    let (out, err, ok) = init(&dir, &[]);
    assert!(!ok, "a directory named `AGENTS.md` is a refusal: {out}");
    assert!(
        err.contains("AGENTS.md"),
        "the refusal names the file: {err}"
    );
    assert!(!err.contains("panicked"), "{err}");
    assert!(files(&dir).is_empty(), "and it wrote nothing");
}

/// A refusal is about the whole run, whichever file it names. The stanza is
/// judged first and the adapter second, so a tree whose adapter is somebody's
/// own file is the one where a run could write the stanza and then refuse; it
/// writes neither, and says `wrote` about nothing.
// covers: tools:agents/nvs-agent-init
#[test]
fn a_refused_adapter_leaves_the_stanza_unwritten() {
    let dir = tree("refused-adapter");
    let skill = dir.join(".claude/skills/novis/SKILL.md");
    std::fs::create_dir_all(skill.parent().expect("the skill has a directory"))
        .expect("the harness's own directory");
    std::fs::write(&skill, "our own skill\n").expect("the project's own file is written");
    let before = snapshot(&dir);

    let (out, err, ok) = init(&dir, &[]);
    assert!(!ok, "somebody's own adapter is a refusal: {out}");
    assert!(
        err.contains(".claude/skills/novis/SKILL.md"),
        "the refusal names the file: {err}"
    );
    assert!(!out.contains("wrote"), "it wrote nothing: {out}");
    assert_eq!(snapshot(&dir), before, "and `AGENTS.md` does not exist");
}

/// What every marker `init` writes opens with.
const MARK: &str = "<!-- nvs agent: written by `nvs agent init`";

/// The protocol the `init` before fingerprints wrote, verbatim, which is what a
/// project that ran that binary still carries.
const LEGACY_PROTOCOL: &str = "\
Novis is the language this project is written in, and the `nvs` binary installed on this machine is
its documentation. It answers from the registry it compiles against, so an answer can never describe
a version that is not installed — which is why nothing about the language itself is written here.

- `nvs agent primer` — read once, before writing anything. The short document that makes a coding
  agent productive.
- `nvs agent index` — one line per member of the standard library.
- `nvs agent find <query>` — the index lines whose class or member name matches. A command rather
  than a grep, because a namespaced name loses its backslash to the shell before grep sees it.
- `nvs agent show <symbol>` — one symbol's card: its signature, its prose, its parameters and what
  it throws.

Then check what you wrote. `nvs check <file>` names what is wrong and where, and `nvs test` runs the
suite. That is the loop — read the primer once, `find` a name, `show` its card, `nvs check` — and
the diagnostic is part of the documentation rather than an alternative to it.
";

/// The `AGENTS.md` and the Claude Code skill the `init` before fingerprints
/// wrote, as `(stanza file, skill)`.
fn legacy() -> (String, String) {
    let stanza = format!(
        "<!-- nvs agent: written by `nvs agent init` -->\n\n## Novis\n\n{LEGACY_PROTOCOL}\n\
         <!-- /nvs agent -->\n"
    );
    let skill = format!(
        "---\nname: novis\ndescription: Ask the installed `nvs` binary about the Novis language \
         and check what you wrote — the primer, the index, one symbol's card, then `nvs check`. \
         Use it whenever reading or writing Novis code.\n---\n\n# Novis\n\n{LEGACY_PROTOCOL}"
    );
    (stanza, skill)
}

/// `text` with its marker carrying the fingerprint of the text around it, the
/// way `init` computes it: BLAKE3 over the text without the marker line, with
/// `\r\n` read as `\n`, cut to eight hex digits. The text must be one unit, as
/// an adapter is.
fn refingerprint(text: &str) -> String {
    let start = text.find(MARK).expect("the unit carries a marker");
    let end = start + text[start..].find('\n').expect("the marker ends its line") + 1;
    let rest = format!("{}{}", &text[..start], &text[end..]).replace("\r\n", "\n");
    let print = &blake3::hash(rest.as_bytes()).to_hex()[..8];
    format!(
        "{}{MARK}, fingerprint {print} -->\n{}",
        &text[..start],
        &text[end..]
    )
}

/// The texts a fresh `init --all` writes, as `(path, text)`, each call in a
/// directory of its own because tests run in parallel.
fn fresh() -> Vec<(String, String)> {
    static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let call = CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("fresh-{call}");
    let out = pointers(&name);
    let _ = std::fs::remove_dir_all(
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("nvs-agent-init-{name}")),
    );
    out
}

/// The pointer a fresh run writes at `path`.
fn fresh_text(path: &str) -> String {
    fresh()
        .into_iter()
        .find(|(at, _)| at == path)
        .map(|(_, text)| text)
        .unwrap_or_else(|| panic!("a fresh run writes {path}"))
}

/// Every unit `init` writes opens its own text with a marker that carries a
/// fingerprint, and an adapter keeps its front matter on the first line, where
/// its harness looks for it.
// covers: tools:agents/nvs-agent-init
#[test]
fn every_unit_carries_a_fingerprint_in_its_marker() {
    for (path, text) in fresh() {
        let line = text
            .lines()
            .find(|line| line.starts_with(MARK))
            .unwrap_or_else(|| panic!("{path} carries a marker"));
        let print = line
            .strip_prefix(MARK)
            .and_then(|tail| tail.strip_prefix(", fingerprint "))
            .and_then(|tail| tail.strip_suffix(" -->"))
            .unwrap_or_else(|| panic!("{path}'s marker carries a fingerprint: {line}"));
        assert_eq!(print.len(), 8, "{path}: {line}");
        assert!(
            print.chars().all(|c| c.is_ascii_hexdigit()),
            "{path}: {line}"
        );
        if path != "AGENTS.md" {
            assert!(
                text.starts_with("---\n"),
                "{path} opens on its front matter"
            );
        }
        // The stanza ends at its close marker, and the line break after it is
        // the file's own.
        let unit = if path == "AGENTS.md" {
            text.trim_end_matches('\n')
        } else {
            &text
        };
        assert_eq!(
            refingerprint(unit),
            unit,
            "{path}: the fingerprint is BLAKE3's"
        );
    }
}

/// A unit that still matches its fingerprint was written by `init` and not
/// edited, so a re-run replaces it with this binary's text — with its line
/// endings converted to CRLF or not.
// covers: tools:agents/nvs-agent-init
#[test]
fn init_updates_a_file_it_wrote_that_nobody_edited() {
    let skill = ".claude/skills/novis/SKILL.md";
    let current = fresh_text(skill);
    for crlf in [false, true] {
        let dir = tree(&format!("outdated-{crlf}"));
        std::fs::create_dir_all(dir.join(".claude")).expect("the harness's own directory");
        let (_, err, ok) = init(&dir, &[]);
        assert!(ok, "the first run succeeds: {err}");

        let mut older = refingerprint(&read(&dir, skill).replace("# Novis", "# Novis, older"));
        if crlf {
            older = older.replace('\n', "\r\n");
        }
        std::fs::write(dir.join(skill), &older).expect("the older pointer is written");

        let (out, err, ok) = init(&dir, &["--check"]);
        assert!(!ok, "`--check` fails on an outdated file: {out}{err}");
        assert_eq!(
            out.lines().collect::<Vec<_>>(),
            [format!("outdated {skill}")]
        );

        let (out, err, ok) = init(&dir, &[]);
        assert!(ok, "an untouched outdated file is updated: {err}");
        assert_eq!(
            out.lines().collect::<Vec<_>>(),
            [format!("updated {skill}")]
        );
        assert_eq!(
            read(&dir, skill),
            current,
            "it now reads as this binary writes it"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// The stanza and the skill the `init` before fingerprints wrote carry no
/// fingerprint, and are recognised by the text that binary wrote. Both are
/// updated, in a LF and in a CRLF checkout, and the project's own text above
/// the stanza is kept.
// covers: tools:agents/nvs-agent-init
#[test]
fn init_updates_what_the_init_before_fingerprints_wrote() {
    let skill = ".claude/skills/novis/SKILL.md";
    let theirs = "# Ours\n\nOur own rules.\n\n";
    let (stanza, legacy_skill) = legacy();
    let current_stanza = fresh_text("AGENTS.md");
    for crlf in [false, true] {
        let dir = tree(&format!("legacy-{crlf}"));
        let convert = |text: String| {
            if crlf {
                text.replace('\n', "\r\n")
            } else {
                text
            }
        };
        std::fs::create_dir_all(dir.join(".claude/skills/novis")).expect("the skill directory");
        std::fs::write(dir.join("AGENTS.md"), convert(format!("{theirs}{stanza}")))
            .expect("the legacy stanza is written");
        std::fs::write(dir.join(skill), convert(legacy_skill.clone()))
            .expect("the legacy skill is written");

        let note = primer_err(&dir);
        assert!(
            note.contains("nvs agent init"),
            "the primer tells the agent the files are old: {note}"
        );

        let (out, err, ok) = init(&dir, &[]);
        assert!(ok, "a legacy install is updated, not refused: {err}");
        assert_eq!(
            out.lines().collect::<Vec<_>>(),
            ["updated AGENTS.md".to_owned(), format!("updated {skill}")]
        );
        let agents = read(&dir, "AGENTS.md");
        assert!(agents.starts_with(&convert(theirs.to_owned())), "{agents}");
        assert!(agents.contains(current_stanza.trim_end()), "{agents}");
        assert_eq!(read(&dir, skill), fresh_text(skill));
        assert_eq!(
            primer_err(&dir),
            "",
            "and the primer has nothing more to say"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// An edited file is refused and left byte for byte as it was. `--force`
/// replaces it, and `--check` names it.
// covers: tools:agents/nvs-agent-init
#[test]
fn init_force_replaces_an_edited_file_and_check_names_it() {
    let dir = tree("force");
    let (_, err, ok) = init(&dir, &[]);
    assert!(ok, "the first run succeeds: {err}");
    let written = read(&dir, "AGENTS.md");
    let edited = written.replace("## Novis", "## Novis, as we use it");
    std::fs::write(dir.join("AGENTS.md"), &edited).expect("the edited file is written");

    let (out, _, ok) = init(&dir, &["--check"]);
    assert!(!ok, "`--check` fails on an edited file");
    assert_eq!(out.trim_end(), "edited AGENTS.md");
    assert_eq!(primer_err(&dir), "", "an edited file is not called old");

    let (_, err, ok) = init(&dir, &[]);
    assert!(!ok, "an edited file is a refusal");
    assert!(
        err.contains("--force"),
        "the refusal names the way out: {err}"
    );
    assert_eq!(read(&dir, "AGENTS.md"), edited, "and it changed nothing");

    let (out, err, ok) = init(&dir, &["--force"]);
    assert!(ok, "`--force` succeeds: {err}");
    assert_eq!(out.trim_end(), "updated AGENTS.md");
    assert_eq!(read(&dir, "AGENTS.md"), written);
    let _ = std::fs::remove_dir_all(&dir);
}

/// `--check` writes nothing. It fails and names each missing file until a run
/// writes them, then says `up to date` and succeeds.
// covers: tools:agents/nvs-agent-init
#[test]
fn init_check_writes_nothing_and_fails_until_every_file_is_current() {
    let dir = tree("check");
    std::fs::create_dir_all(dir.join(".cursor")).expect("the harness's own directory");

    let (out, _, ok) = init(&dir, &["--check"]);
    assert!(!ok, "missing files fail the check");
    assert_eq!(
        out.lines().collect::<Vec<_>>(),
        ["missing AGENTS.md", "missing .cursor/rules/novis.mdc"]
    );
    assert!(files(&dir).is_empty(), "and it wrote nothing");

    let (_, err, ok) = init(&dir, &[]);
    assert!(ok, "the run succeeds: {err}");
    let (out, err, ok) = init(&dir, &["--check"]);
    assert!(ok, "a current tree passes: {err}");
    assert_eq!(out.trim_end(), "up to date");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Cursor is detected by `.cursor/`, and Copilot by its own instruction files.
/// A `.github/` directory alone is in most projects, so it shows no harness.
// covers: tools:agents/nvs-agent-init
#[test]
fn init_writes_the_cursor_and_copilot_pointers_when_those_harnesses_are_present() {
    let cursor = ".cursor/rules/novis.mdc";
    let copilot = ".github/instructions/novis.instructions.md";
    let cases: [(&str, &[&str], Option<&str>); 4] = [
        ("cursor", &[".cursor"], Some(cursor)),
        ("github-only", &[".github/workflows"], None),
        ("copilot-dir", &[".github/instructions"], Some(copilot)),
        ("copilot-file", &[".github"], Some(copilot)),
    ];
    for (name, dirs, expected) in cases {
        let dir = tree(&format!("harness-{name}"));
        for sub in dirs {
            std::fs::create_dir_all(dir.join(sub)).expect("the harness's own directory");
        }
        if name == "copilot-file" {
            std::fs::write(dir.join(".github/copilot-instructions.md"), "Ours.\n")
                .expect("the project's own instructions");
        }
        let (out, err, ok) = init(&dir, &[]);
        assert!(ok, "`{name}`: {err}");
        let mut wrote: Vec<&str> = vec!["wrote AGENTS.md"];
        let line = expected.map(|path| format!("wrote {path}"));
        if let Some(line) = &line {
            wrote.push(line);
        }
        assert_eq!(out.lines().collect::<Vec<_>>(), wrote, "`{name}`");
        if let Some(path) = expected {
            assert_eq!(read(&dir, path), fresh_text(path), "`{name}`");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    let mdc = fresh_text(cursor);
    assert!(mdc.contains("\nglobs: **/*.nvs\n"), "{mdc}");
    assert!(mdc.contains("\nalwaysApply: false\n"), "{mdc}");
    assert!(mdc.contains("\ndescription: "), "{mdc}");
    let instructions = fresh_text(copilot);
    assert!(
        instructions.starts_with("---\napplyTo: \"**/*.nvs\"\n---\n"),
        "{instructions}"
    );
}

/// A project that ran `init` and now shows a harness this binary has a pointer
/// for lacks a file, and the primer says so.
// covers: tools:agents/nvs-agent-init
#[test]
fn the_primer_notes_a_harness_whose_pointer_is_missing_and_nothing_else() {
    let dir = tree("primer-note");
    assert_eq!(primer_err(&dir), "", "a project with no files has no note");
    let (_, err, ok) = init(&dir, &[]);
    assert!(ok, "the run succeeds: {err}");
    assert_eq!(primer_err(&dir), "", "a current project has no note");

    std::fs::create_dir_all(dir.join(".cursor")).expect("the harness's own directory");
    let note = primer_err(&dir);
    assert_eq!(note.lines().count(), 1, "the note is one line: {note}");
    assert!(note.contains("nvs agent init"), "{note}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// `nvs agent primer`, run in `dir`, as its standard error.
fn primer_err(dir: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .args(["agent", "primer"])
        .current_dir(dir)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    assert!(out.status.success(), "the primer succeeds");
    String::from_utf8(out.stderr).expect("the output is UTF-8")
}
