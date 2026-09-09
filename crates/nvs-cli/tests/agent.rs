//! `nvs agent`, driven the way a coding agent drives it — through the built
//! binary, because `rule:tooling/an-agent-asks-the-binary`'s contract is what
//! the *command* prints, not what a function returns.
//!
//! The correspondence cases here are member-for-member against `nvs meta
//! --json` rather than against a count, which is what
//! `rule:tooling/the-index-is-one-line-per-member` asks for: completeness is the
//! property the index exists to have, and a count passes over a member swapped
//! for another.

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

/// The symbol a line opens with: everything before its first `(`, `<` or space,
/// which is the whole parser a consumer needs to get from a line back to `show`.
fn symbol_of(line: &str) -> &str {
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
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/reference")
        .join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{} is readable", path.display()))
}

/// The five parts `rule:tooling/a-primer-claim-is-executed` fixes, each
/// recognized by something only the section that carries it says.
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
