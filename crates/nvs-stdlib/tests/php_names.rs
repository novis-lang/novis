//! The PHP-name table, held against the three inputs it is joined from.
//!
//! `build.rs` reads two documents and emits
//! [`php_names::CANDIDATES`](nvs_stdlib::php_names::CANDIDATES); the registry
//! is the third input, and a build script cannot reach it, so what a build
//! script would otherwise have refused is refused here — under
//! `nv verify`, which stops at the first failing step, a red
//! `cargo test -p nvs-stdlib` is a build that does not finish.
//!
//! Three claims about the table, one per test, and they are the three halves
//! of `rule:php-migration/every-php-builtin-is-a-completion-candidate` and
//! `rule:ide/three-of-four-item-shapes-insert-nothing`:
//!
//! - the candidate list is the oracle inventory **whole**, so coverage of the
//!   migration table never decides whether the feature works;
//! - a destination spelling a registered class does not declare is a typo in
//!   the migration table's Novis cell, and fails here rather than reaching a
//!   developer as a suggestion that does not resolve;
//! - three of the four item shapes insert nothing, asserted as the absence of
//!   an edit rather than as an empty string.
//!
//! A fourth test holds the table's second reader, `php_names::became`, which
//! is what a diagnostic says of a PHP function: it names a spelling only where
//! the row opens on one.
//!
//! # What the second one can and cannot see
//!
//! A destination whose **class** the registry does not declare at all is a
//! class nobody has built yet, or the one interface shape
//! `nvs_stdlib::php_names`'s own docs describe; it is the second item shape and
//! not a failure. A destination whose class *is* registered and whose member is
//! not is the failure this exists for — nothing else in the tree distinguishes
//! `Core\Str::trimEnd` from `Core\Str::trimEndd`.
//!
//! The exemptions come from
//! [`tests/migration-members-outstanding.txt`](migration-members-outstanding.txt),
//! which `tests/spec_registry_coverage.rs` owns and this file only reads. That
//! file's own gate is what fails on a stale line, so this one asserts in one
//! direction only: an exemption nobody needs any more is reported there, once,
//! rather than here as well.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use nvs_stdlib::php_names::{
    CANDIDATES, Candidate, Destination, Item, Kind, Outcome, became, function,
};
use nvs_stdlib::registry;

/// One path under the workspace root, read.
fn read(relative: &str) -> String {
    let path = nvs_repo::path(relative);
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

/// Every name the oracle inventory lists, with the section that listed it.
///
/// The inventory's own parser, written a second time on purpose: the whole
/// claim below is that `build.rs` read this file correctly, and a test sharing
/// its reader could only assert that it is self-consistent.
fn inventory() -> BTreeSet<(String, Kind)> {
    let text = read("tools/data/php-builtins.txt");
    let mut section = None;
    let mut names = BTreeSet::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        match line {
            "[functions]" => section = Some(Kind::Function),
            "[types]" => section = Some(Kind::Type),
            name => {
                if let Some(kind) = section {
                    names.insert((name.to_owned(), kind));
                }
            }
        }
    }
    names
}

/// The `# functions: N  types: M` line the inventory's header carries.
///
/// `tools/dump-php-builtins.php` writes it from the same build it dumped, so it
/// is the one count in this test that comes from PHP rather than from a parse
/// of PHP's output.
fn header_counts() -> BTreeMap<Kind, usize> {
    let text = read("tools/data/php-builtins.txt");
    let line = text
        .lines()
        .find(|line| line.starts_with("# functions:"))
        .expect("the inventory's header states its own counts");
    let mut counts = BTreeMap::new();
    let mut fields = line.trim_start_matches('#').split_whitespace();
    while let (Some(label), Some(count)) = (fields.next(), fields.next()) {
        let kind = match label {
            "functions:" => Kind::Function,
            "types:" => Kind::Type,
            _ => continue,
        };
        counts.insert(kind, count.parse().expect("a count"));
    }
    counts
}

/// The keys of `tests/migration-members-outstanding.txt`, as `(class, member)`.
///
/// A line is `<key>  # <owner>`; the owner column is that file's own gate's
/// business, not this one's.
fn outstanding() -> BTreeSet<(String, String)> {
    let text = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/migration-members-outstanding.txt"),
    )
    .expect("tests/migration-members-outstanding.txt");
    text.lines()
        .map(|line| line.split('#').next().unwrap_or_default().trim())
        .filter(|key| !key.is_empty())
        .filter_map(|key| key.split_once("::"))
        .map(|(class, member)| (class.to_owned(), member.to_owned()))
        .collect()
}

/// Every name in the differential oracle's inventory is a candidate, whatever
/// the migration table says about it or does not say at all.
///
/// The list is complete by construction and the table is filled in one PHP
/// domain at a time — `rule:php-migration/every-php-builtin-is-a-completion-candidate`
/// is that split, and this is the half of it a test can hold.
#[test]
fn every_php_builtin_in_the_oracle_inventory_is_a_candidate() {
    let want = inventory();
    let counts = header_counts();
    for (kind, count) in &counts {
        let read = want.iter().filter(|(_, found)| found == kind).count();
        assert_eq!(
            read, *count,
            "tools/data/php-builtins.txt lists {read} {kind:?} name(s) where its own header says \
             {count} — the file's shape has changed under this parser"
        );
    }

    let have: BTreeSet<(String, Kind)> = CANDIDATES
        .iter()
        .map(|candidate| (candidate.php.to_owned(), candidate.kind))
        .collect();
    let missing: Vec<_> = want.difference(&have).collect();
    assert!(
        missing.is_empty(),
        "{} inventory name(s) are not candidates: {missing:?}",
        missing.len()
    );
    let invented: Vec<_> = have.difference(&want).collect();
    assert!(
        invented.is_empty(),
        "{} candidate(s) are in no section of the inventory: {invented:?}",
        invented.len()
    );

    let sorted: Vec<_> = CANDIDATES.iter().map(|candidate| candidate.php).collect();
    assert!(
        sorted.windows(2).all(|pair| pair[0] <= pair[1]),
        "the table is sorted by its PHP spelling — `php_names::starting_with` binary-searches it"
    );
}

/// A `member` row naming a member of a registered class that the class does not
/// declare is a spelling the migration table gets wrong, and it fails here
/// rather than reaching a developer.
///
/// `rule:php-migration/an-item-inserts-only-a-registered-member` is the rule,
/// and the module doc above owns why a destination whose *class* is unregistered
/// is the second item shape instead of a failure.
#[test]
fn a_destination_spelling_that_matches_no_registry_member_fails_the_build() {
    // The refusal itself, asked of a spelling no document holds: a member on a
    // class the registry certainly declares. A refusal nothing exercises can
    // stop refusing without anything going red.
    let typo = Destination {
        class: "Core\\Str",
        member: "trimEndd",
    };
    assert!(
        registry::class(typo.class).is_some(),
        "`Core\\Str` is registered, so this case tests the member half"
    );
    assert!(!typo.is_registered(), "`Core\\Str` declares no `trimEndd`");
    let mistyped = Candidate {
        php: "chop",
        kind: Kind::Function,
        outcome: Outcome::Member,
        cell: "`Core\\Str::trimEndd`",
        destinations: &[Destination {
            class: "Core\\Str",
            member: "trimEndd",
        }],
    };
    assert_eq!(mistyped.items(), vec![Item::NotRegistered(Some(typo))]);
    assert!(mistyped.items()[0].insertion().is_none());

    // And the table as it is on disk.
    let exempt = outstanding();
    let mut resolved = 0usize;
    let mut wrong = BTreeSet::new();
    for candidate in CANDIDATES {
        for destination in candidate.destinations {
            if destination.is_registered() {
                resolved += 1;
                continue;
            }
            if registry::class(destination.class).is_none() {
                continue;
            }
            let key = (destination.class.to_owned(), destination.member.to_owned());
            if !exempt.contains(&key) {
                wrong.insert((candidate.php, destination.spelling()));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "docs/spec/02-php-migration.md names {} destination(s) on a class the registry declares \
         that the class does not: {wrong:?} — either the spelling is wrong, or the member is owed \
         and belongs in tests/migration-members-outstanding.txt with the goal that will build it",
        wrong.len()
    );
    assert!(
        resolved > 200,
        "only {resolved} destination(s) resolved against the registry — the join has stopped \
         reading the table rather than found it clean"
    );
}

/// `rule:ide/three-of-four-item-shapes-insert-nothing`, over the generated
/// table's own rows.
///
/// Asserted here rather than over a rendered completion item because the table
/// is what says whether a row has an insertable destination at all; what an
/// editor writes beside the insertion is `nvs-lsp`'s.
#[test]
fn three_of_the_four_item_shapes_insert_nothing() {
    let mut inserts = 0usize;
    let mut not_registered = 0usize;
    let mut dropped = 0usize;
    let mut undecided = 0usize;

    for candidate in CANDIDATES {
        let items = candidate.items();
        assert!(
            !items.is_empty(),
            "`{}` offers no item at all, so the name would not appear",
            candidate.php
        );
        for item in items {
            match item {
                Item::Inserts(destination) => {
                    inserts += 1;
                    let text = item
                        .insertion()
                        .expect("a registered destination is the one shape that inserts");
                    assert_eq!(text, destination.spelling());
                    assert!(
                        text.starts_with("Core\\"),
                        "`{}` would insert `{text}`, which is not a `Core` spelling",
                        candidate.php
                    );
                    assert_ne!(
                        text, candidate.php,
                        "the PHP spelling never reaches a file — `rule:statements/nothing-gets-a-second-name`"
                    );
                    assert!(destination.is_registered());
                }
                Item::NotRegistered(_) | Item::Dropped | Item::Undecided => {
                    match item {
                        Item::NotRegistered(_) => not_registered += 1,
                        Item::Dropped => dropped += 1,
                        _ => undecided += 1,
                    }
                    assert!(
                        item.insertion().is_none(),
                        "`{}`'s {item:?} item inserts text, and three of the four shapes insert \
                         nothing — the absence of an edit, not an empty string",
                        candidate.php
                    );
                }
            }
        }
    }

    for (shape, count) in [
        ("inserts", inserts),
        ("not registered", not_registered),
        ("dropped", dropped),
        ("undecided", undecided),
    ] {
        assert!(
            count > 0,
            "no row produced the `{shape}` shape, so its half of the assertion above passed \
             vacuously"
        );
    }
}

/// What a diagnostic says of a PHP function is read off that function's own
/// row: the code span the cell opens on for a `member` or a `language` row, and
/// that nothing does the job for a `dropped` one.
///
/// The sweep is the claim a handful of names cannot make. No answer is a
/// spelling the row does not open on, and a row that opens on prose — "the
/// same pair" — answers nothing, because its words mean something only under
/// the row above it.
#[test]
fn a_php_function_is_told_the_spelling_its_row_opens_on() {
    assert_eq!(
        became("count").as_deref(),
        Some("is `Core\\Arr::count` here")
    );
    assert_eq!(became("intval").as_deref(), Some("is `$x as int` here"));
    assert_eq!(
        became("addslashes").as_deref(),
        Some("has no counterpart here")
    );

    // PHP resolves a function without regard to case, and a fully-qualified
    // call writes one leading `\`.
    assert_eq!(became("\\COUNT"), became("count"));
    // A name PHP does not have, and a PHP name under a namespace, are nobody's
    // built-in.
    assert_eq!(became("tally"), None);
    assert_eq!(became("Core\\count"), None);
    // A type is not called, so its name is no function's.
    let ty = CANDIDATES
        .iter()
        .find(|candidate| candidate.kind == Kind::Type)
        .expect("the inventory lists types");
    assert!(function(ty.php).is_none_or(|found| found.kind == Kind::Function));

    let mut spelled = 0usize;
    let mut silent = 0usize;
    let functions = CANDIDATES
        .iter()
        .filter(|candidate| candidate.kind == Kind::Function);
    for candidate in functions {
        let answer = became(candidate.php);
        match candidate.outcome {
            Outcome::Member | Outcome::Language => {
                let Some(clause) = answer else {
                    silent += 1;
                    continue;
                };
                let written = clause
                    .strip_prefix("is `")
                    .and_then(|rest| rest.strip_suffix("` here"))
                    .unwrap_or_else(|| panic!("`{}` answers `{clause}`", candidate.php));
                assert!(
                    !written.is_empty()
                        && candidate
                            .cell
                            .trim_start()
                            .starts_with(&format!("`{written}`")),
                    "`{}` is told `{written}`, which its row does not open on: {}",
                    candidate.php,
                    candidate.cell
                );
                assert!(
                    candidate
                        .destinations
                        .iter()
                        .all(Destination::is_registered),
                    "`{}` is told a row naming a member the registry does not hold",
                    candidate.php
                );
                spelled += 1;
            }
            Outcome::Dropped => assert_eq!(
                answer.as_deref(),
                Some("has no counterpart here"),
                "`{}` is a dropped row",
                candidate.php
            ),
            Outcome::Open => assert_eq!(answer, None, "`{}` is undecided", candidate.php),
        }
    }
    assert!(
        spelled > 0 && silent > 0,
        "{spelled} row(s) named a spelling and {silent} named none — one half of the sweep \
         passed vacuously"
    );
}
