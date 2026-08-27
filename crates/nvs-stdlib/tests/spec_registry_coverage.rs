//! The gate `docs/agent/loop-goal.md` names as this loop's own definition of
//! done: every member
//! [docs/spec/01-core-library.md](../../../docs/spec/01-core-library.md) §§ 1-12
//! writes as a **table row** is a member some class in [`registry::CLASSES`]
//! declares.
//!
//! This is the mirror of `conformance_coverage.rs`, which walks the registry
//! and asks the repository for a case. This walks the *spec* and asks the
//! registry for a row, so the two together close the loop: a member cannot be
//! specified without being registered, and cannot be registered without being
//! run.
//!
//! # The outstanding list is a file, and it only shrinks
//!
//! §§ 1-12 are not on disk yet — `crates/nvs-stdlib`'s own known gap 1 and the
//! plan's `Open now` say how much is owed — so the honest reading of "fails
//! naming every one with no registry entry" is a test that is red for the
//! whole of the loop that exists to make it green. That trade was refused:
//! `tools/verify.py` stops at the first failing step, so a permanently red
//! `cargo test` costs every later session its clippy and fmt signal, which is
//! a much larger loss than the one it buys. The acceptance gate withholds
//! *done* on `loop-goal.toml`'s `min_passing = 600` regardless, and that
//! number is untouched by anything here.
//!
//! So the outstanding members live in
//! [`tests/spec-members-outstanding.txt`](spec-members-outstanding.txt), a
//! checked-in ratchet, and this test fails in **both** directions:
//!
//! - a spec row with no registry entry that the file does not list — which is
//!   a row added to the spec without an implementation, the regression this
//!   guards;
//! - a line in the file whose member *is* now registered — so implementing a
//!   member and striking its line are one edit, and the file cannot drift into
//!   a list of things that were true once.
//!
//! When the file is empty, its emptiness *is* the sentence at the top of this
//! doc, and the loop is done with §§ 1-12.
//!
//! # What a row is, and what is deliberately not checked
//!
//! Only a `| Member | Signature | … |` table is read. Three things in §§ 1-12
//! are therefore out of scope on purpose, each because the spec states it as
//! prose rather than as a row: `Core\Bytes`'s twelve members (§ 7 lists them
//! in a sentence), § 9's three collections (a `| Type | Members |` table of
//! bare names), and the members a Notes cell mentions in passing —
//! `toEpochMicros`, `Duration::hours`, `Core\Time\Date::at`. Widening the
//! parser to reach them would mean reading English, not a table.
//!
//! A member **that writes no class of its own** resolves against every class
//! the section's own heading names,
//! plus every registered class inside one of their namespaces — so § 5's
//! `Match` rows are answered by `Core\Regex\Match` without the parser having
//! to read the sentence that introduces it, and § 4's `plus` is answered by
//! `Core\Time\Instant` on behalf of `Core\Time\DateTime`'s identical row. That
//! is looser than a checker would be, and for the same reason
//! `conformance_coverage.rs` accepts any receiver's `->text(`: the alternative
//! is a second checker rather than a coverage gate.
//!
//! A row that *does* write its own class — `Uri::parse`, `Csv::parse` — is
//! resolved against that class alone. § 12 is where the difference shows: its
//! heading names four classes, two of them write a `parse`, and the loose
//! reading would let either one strike the other's line. See [`scoped`].

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use nvs_stdlib::registry;

/// Every `` `code` `` span in `text`, in order, without their backticks.
fn spans(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('`') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('`') else { break };
        found.push(&after[..close]);
        rest = &after[close + 1..];
    }
    found
}

/// The section number of a `## 7. …` heading, or `None` for a heading that
/// does not open a numbered section.
fn section_number(line: &str) -> Option<u32> {
    let rest = line.strip_prefix("## ")?;
    let (digits, _) = rest.split_once(". ")?;
    digits.parse().ok()
}

/// The bare member name a Member cell's code span writes, with any receiver
/// (`$match->group`) or class qualifier (`Random::int`) dropped — `None` for a
/// span that is not a member name at all.
fn member_name(span: &str) -> Option<&str> {
    let after_receiver = span.split_once("->").map_or(span, |(_, rest)| rest);
    let name = after_receiver.rsplit("::").next()?;
    let plain = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    plain.then_some(name)
}

/// Every registered class a section heading reaches: the `Core\X` names the
/// heading itself writes, plus everything registered inside one of their
/// namespaces.
fn classes_in(heading: &str) -> Vec<&'static registry::CoreClass> {
    let named: Vec<&str> = spans(heading)
        .into_iter()
        .filter(|span| span.starts_with(r"Core\"))
        .collect();
    registry::CLASSES
        .iter()
        .filter(|class| {
            named
                .iter()
                .any(|name| class.name == *name || class.name.starts_with(&format!(r"{name}\")))
        })
        .collect()
}

/// The class a Member cell's span qualifies its member with — `Uri` in
/// `Uri::parse` — or `None` for a bare name or a receiver form, neither of
/// which says which class it belongs to.
fn qualifier(span: &str) -> Option<&str> {
    if span.contains("->") {
        return None;
    }
    let (class, _) = span.rsplit_once("::")?;
    (!class.is_empty()).then_some(class)
}

/// `candidates`, narrowed to the one class a span names for itself.
///
/// § 12's heading names four classes and two of them write a `parse` row, so
/// the section-wide reading alone would let `Core\Csv::parse` answer on
/// `Core\Uri::parse`'s behalf and strike a member nobody has written. A span
/// that writes its own qualifier is therefore resolved against that class and
/// no other. A qualifier naming a class this section has not registered at all
/// falls back to the whole list, which is what keeps the deliberate looseness
/// this file's own docs describe for every row that writes a bare name.
fn scoped<'a>(candidates: &[&'a registry::CoreClass], span: &str) -> Vec<&'a registry::CoreClass> {
    let Some(class) = qualifier(span) else {
        return candidates.to_vec();
    };
    let suffix = format!(r"\{class}");
    let narrowed: Vec<&'a registry::CoreClass> = candidates
        .iter()
        .copied()
        .filter(|found| found.name == class || found.name.ends_with(&suffix))
        .collect();
    if narrowed.is_empty() {
        candidates.to_vec()
    } else {
        narrowed
    }
}

/// Whether any of `candidates` declares `name`, as a static member, an
/// instance member or a constant.
fn registered(candidates: &[&'static registry::CoreClass], name: &str) -> bool {
    candidates
        .iter()
        .any(|class| class.members().any(|m| m.name == name) || class.constant(name).is_some())
}

/// The keys `spec-members-outstanding.txt` lists, blank lines and `#` comments
/// dropped.
fn outstanding_file() -> (std::path::PathBuf, BTreeSet<String>) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/spec-members-outstanding.txt");
    let text = fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    let keys = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect();
    (path, keys)
}

#[test]
fn every_part_one_spec_member_is_registered() {
    let spec = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/spec/01-core-library.md");
    let text = fs::read_to_string(&spec).unwrap_or_else(|err| panic!("{}: {err}", spec.display()));

    let mut section = None;
    let mut candidates = Vec::new();
    let mut in_members = false;
    let mut seen = 0usize;
    let mut outstanding = BTreeSet::new();

    for line in text.lines() {
        if let Some(number) = section_number(line) {
            // §§ 13-19 are Part II and later — `loop-goal.md` § *Standing
            // decisions* puts them out of scope, so the walk stops at 12.
            section = (1..=12).contains(&number).then_some(number);
            candidates = classes_in(line);
            in_members = false;
            continue;
        }
        let Some(number) = section else { continue };
        if !line.starts_with('|') {
            in_members = false;
            continue;
        }
        let cell = line
            .trim_start_matches('|')
            .split('|')
            .next()
            .unwrap_or("")
            .trim();
        if cell == "Member" {
            in_members = true;
            continue;
        }
        if cell.starts_with("---") || !in_members {
            continue;
        }
        // One cell can write a pair or a triple — `` `plus` / `minus` ``,
        // `` `Uri::encodeComponent` / `decodeComponent` `` — and each span in
        // it is its own member.
        for span in spans(cell) {
            let Some(name) = member_name(span) else {
                continue;
            };
            seen += 1;
            if !registered(&scoped(&candidates, span), name) {
                outstanding.insert(format!("§{number} {span}"));
            }
        }
    }

    assert!(
        seen > 100,
        "{} yielded only {seen} member row(s), which is too few to be §§ 1-12 — \
         the table parser has stopped matching the spec's own shape",
        spec.display()
    );

    let (path, listed) = outstanding_file();
    let unlisted: Vec<&String> = outstanding.difference(&listed).collect();
    assert!(
        unlisted.is_empty(),
        "{} spec member(s) in §§ 1-12 have no `registry::CLASSES` row and are not listed in {}: {}\n\
         Register the member (four things — see docs/agent/conventions.md), or add its key to \
         that file if it is genuinely still owed.",
        unlisted.len(),
        path.display(),
        unlisted
            .iter()
            .map(|key| key.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    let stale: Vec<&String> = listed.difference(&outstanding).collect();
    assert!(
        stale.is_empty(),
        "{} line(s) in {} name a member that is registered now, or a key no spec row produces: {}\n\
         Delete those lines — the list only shrinks, and striking a line is part of the slice \
         that registers the member.",
        stale.len(),
        path.display(),
        stale
            .iter()
            .map(|key| key.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
}
