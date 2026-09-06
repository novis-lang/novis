//! The gate `docs/agent/loop-goal.md` names as this loop's own definition of
//! done: every member
//! [docs/spec/01-core-library.md](/docs/spec/01-core-library.md) §§ 1-12
//! writes as a **table row** is a member some class in [`registry::CLASSES`]
//! declares.
//!
//! [`every_part_two_spec_member_is_registered`] asks the same question of
//! §§ 14-19, which Part II writes as bullets rather than as tables — its own
//! doc owns that difference, and the two sections' exclusions. Those two,
//! §§ 16 and 17, get [`every_part_two_spec_class_is_registered`] instead: their
//! members are English but their `Class` column is not, so the roster is
//! checked where the signatures cannot be.
//!
//! This is the mirror of `conformance_coverage.rs`, which walks the registry
//! and asks the repository for a case. This walks the *spec* and asks the
//! registry for a row, so the two together close the loop: a member cannot be
//! specified without being registered, and cannot be registered without being
//! run.
//!
//! # The third walk reads the other spec file
//!
//! [docs/spec/02-php-migration.md](/docs/spec/02-php-migration.md) accounts for
//! every PHP name rather than for every Novis one, and its own header states
//! the rule this file enforces: *"Every `member` spelling in this file is
//! checked against `nvs-stdlib`'s registry … so a spelling this file gets wrong
//! fails a build instead of reaching anyone."*
//! [`every_migration_member_row_names_a_registered_member`] is that build
//! failure and [`every_migration_member_row_has_a_conformance_case`] asks the
//! corpus the same question one step later, so a row promising a migrating
//! program a member cannot be promising it a name that does not resolve or a
//! member nothing ever ran. The two are the parity program's stop condition
//! stated as a test: when
//! [`tests/migration-members-outstanding.txt`](migration-members-outstanding.txt)
//! is empty, every member 1,151 PHP functions were pointed at exists.
//!
//! They are here rather than in `conformance_coverage.rs` because the walk is a
//! spec walk — the table is the enumerable set and the registry is what it is
//! held against, which is this file's direction and not that one's.
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
//! # An outstanding key names its owner, in a column
//!
//! Every key in every one of these files carries `# <owner>` after it: the goal
//! from [chain.toml](/docs/agent/goals/chain.toml) that will strike the line, or
//! the word `unowned` for a key that is nobody's yet and is a scheduling
//! question for the user. [`every_outstanding_key_names_an_owner`] is what
//! makes that a field rather than a note — it reads the chain and fails on an
//! owner no entry there answers for, so a goal renamed or dropped cannot leave a
//! key pointing at nothing.
//!
//! The column exists because these facts were header prose, where one paragraph
//! owned eight keys and could not say which was which.
//! [docs/agent/carried-gaps.md](/docs/agent/carried-gaps.md) § *The contract* is
//! the rule this is the ratchet-file spelling of, and the failure it exists to
//! stop is on record in its own opening: `§18 stream` read as "goal 5's" for six
//! goals after goal 5 closed.
//!
//! What the gate deliberately does not check is whether an owner is still
//! *ahead*. A chain entry that goes green without striking its key is the more
//! interesting failure and it is a reader's to catch, because the chain file
//! holds the order and not the position — nothing on disk says where the loop
//! is. `unowned` is the honest answer once it happens, and striking the owner
//! rather than the key is that same contract's second rule.
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

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use nvs_stdlib::registry;
use regex::Regex;

mod corpus;

use corpus::{Attribution, mentions, sources};

/// A table row's cells, split on the pipes that are *not* escaped.
///
/// A union type inside a cell writes `int\|string`, because a bare `|` would
/// end the cell — so a naive `split('|')` tears exactly the rows that declare
/// a union into fragments, and a Signature cell torn that way loses its
/// closing backtick and reads as no signature at all. That is invisible to
/// [`every_part_one_spec_member_is_registered`], whose Member cell never holds
/// a union, and was silently dropping a fifth of the rows from
/// [`every_registry_rows_names_are_the_specs_signature_column`].
fn cells(line: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut escaped = false;
    for c in line.trim().trim_matches('|').chars() {
        match c {
            '|' if !escaped => out.push(String::new()),
            _ => {
                if escaped && c != '|' {
                    out.last_mut().expect("a cell").push('\\');
                }
                out.last_mut().expect("a cell").push(c);
            }
        }
        escaped = c == '\\' && !escaped;
    }
    out.iter().map(|cell| cell.trim().to_owned()).collect()
}

/// The `$name` each parameter of a spec signature is written with, in order,
/// with a trailing options shape folded to the one name
/// [`registry::OPTIONS_NAME`] — `None` for a span that is not a signature.
///
/// This is [`registry::CoreMethod::names`]'s source of truth, read live rather
/// than copied, which is the whole point of the test below: `rule:core-api/shape-rules` R2 makes
/// a parameter's name compatibility surface versioned in the spec, so the spec
/// is where it is *written* and the registry only mirrors it.
fn signature_names(sig: &str) -> Option<Vec<String>> {
    let open = sig.find('(')?;
    let mut depth = 0usize;
    let mut close = None;
    for (index, c) in sig.char_indices().skip(open) {
        match c {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(index);
                    break;
                }
            }
            _ => {}
        }
    }
    let mut names = Vec::new();
    for item in top_level_items(&sig[open + 1..close?]) {
        // A shape-typed parameter is written `{name: callable, …} $tasks`, so a
        // leading `{` does not make an item the trailing bag — having no
        // `$name` of its own does. `Core\Task::all` is where the two meet.
        let Some((_, rest)) = item.split_once('$') else {
            if item.starts_with('{') {
                names.push(registry::OPTIONS_NAME.to_owned());
                continue;
            }
            return None;
        };
        let end = rest
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .unwrap_or(rest.len());
        names.push(rest[..end].to_owned());
    }
    Some(names)
}

/// A parameter list split on its top-level commas — a nested one belongs to a
/// generic argument (`array<int\|string>`) or to an options shape.
fn top_level_items(args: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut cur = String::new();
    for c in args.chars() {
        match c {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => depth -= 1,
            ',' if depth == 0 => {
                out.push(cur.trim().to_owned());
                cur = String::new();
                continue;
            }
            _ => {}
        }
        cur.push(c);
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_owned());
    }
    out
}

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

/// Every ratchet file this module owns, named once so a gate written over all of
/// them cannot quietly miss one.
///
/// There is one per walk — §§ 1-12's, §§ 14-19's, §§ 16-17's classes and the
/// migration table's — because the halves are finished by different loops and a
/// single file would make a Part I regression indistinguishable from a Part II
/// member nobody has reached yet.
const RATCHETS: [&str; 4] = [
    "spec-members-outstanding.txt",
    "spec-members-part-two-outstanding.txt",
    "spec-classes-part-two-outstanding.txt",
    "migration-members-outstanding.txt",
];

/// One ratchet file, read.
struct Ratchet {
    /// Where it is, so a failure can name the file to edit.
    path: std::path::PathBuf,
    /// Every key it lists, owner column dropped. This is what each gate's two
    /// set comparisons are written against, so giving a line an owner cannot
    /// change what its gate reads.
    keys: BTreeSet<String>,
    /// The owner each key names, by key, and absent for a key that names none.
    /// [`every_outstanding_key_names_an_owner`] is the reader.
    owners: BTreeMap<String, String>,
}

/// Read one: blank lines and whole-line `#` comments dropped, and every
/// remaining line split into its key and its owner column.
///
/// A line is `<key>  # <owner>`. The module doc above owns why the owner is a
/// column rather than a sentence in the header.
fn outstanding_file(name: &str) -> Ratchet {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(name);
    let text = fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    let mut keys = BTreeSet::new();
    let mut owners = BTreeMap::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, owner) = match line.split_once('#') {
            Some((key, owner)) => (key.trim(), owner.trim()),
            None => (line, ""),
        };
        assert!(
            !line.contains('#') || !owner.is_empty(),
            "{}: `{line}` opens an owner column and writes nothing in it",
            path.display()
        );
        if !owner.is_empty() {
            owners.insert(key.to_owned(), owner.to_owned());
        }
        keys.insert(key.to_owned());
    }
    Ratchet { path, keys, owners }
}

/// [docs/agent/goals/chain.toml](/docs/agent/goals/chain.toml), read for the one
/// thing an owner column is checked against: which goal numbers the chain still
/// lists.
///
/// The text is taken by the caller so that a `&str` key can borrow from it —
/// and a goal's *number* is its name up to the first space, because
/// `AGENTS.md`'s own rule is that a goal is said as "goal 19" and a milestone
/// tag says nothing about order.
fn chain_goals(text: &str) -> BTreeSet<&str> {
    text.lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix("name = \""))
        .map(|name| name.split(' ').next().unwrap_or(name))
        .collect()
}

/// Read the chain, for [`chain_goals`] to walk.
fn chain_text() -> String {
    let chain = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/agent/goals/chain.toml");
    fs::read_to_string(&chain).unwrap_or_else(|err| panic!("{}: {err}", chain.display()))
}

/// What is wrong with one key's owner column, or `None` if nothing is.
///
/// Two kinds of owner pass where
/// [carried-gaps.md](/docs/agent/carried-gaps.md) § *The contract* allows three.
/// A milestone tag is an owner for a *gap*, which a plan can cover; a key here
/// is struck by a session, and only a chain entry runs sessions. A milestone
/// nobody has cut into goals therefore reads here as `unowned`, which is what it
/// is — that is the correction spec § 17's four classes needed, having been
/// filed under an M9 whose plan carries none of them.
///
/// It is a function rather than a `match` inside the gate so that
/// [`an_owner_that_is_not_a_live_chain_entry_fails`] can ask it about an owner
/// no file on disk writes: a refusal nothing ever exercises is a refusal that
/// can stop refusing without anything going red.
fn owner_problem(owner: Option<&String>, goals: &BTreeSet<&str>) -> Option<String> {
    match owner {
        Some(owner) if owner == "unowned" || goals.contains(owner.as_str()) => None,
        Some(owner) => Some(format!(
            "names goal {owner}, which is no `[[goal]]` on the chain"
        )),
        None => Some("names no owner".to_owned()),
    }
}

/// Every key in every [`RATCHETS`] file names an owner a reader can act on: a
/// `[[goal]]` [chain.toml](/docs/agent/goals/chain.toml) still lists, or the
/// word `unowned`.
///
/// [`owner_problem`] owns which two those are, and the module doc owns why the
/// owner is a column rather than a header sentence.
#[test]
fn every_outstanding_key_names_an_owner() {
    let text = chain_text();
    let goals = chain_goals(&text);
    assert!(
        goals.len() > 10,
        "docs/agent/goals/chain.toml yielded only {} `[[goal]]` name(s) — the chain's shape has \
         changed under this walk, and every owner below is being accepted against almost nothing",
        goals.len()
    );

    let mut wrong = Vec::new();
    let mut checked = 0usize;
    for name in RATCHETS {
        let ratchet = outstanding_file(name);
        for key in &ratchet.keys {
            checked += 1;
            if let Some(problem) = owner_problem(ratchet.owners.get(key), &goals) {
                wrong.push(format!("{name}: `{key}` {problem}"));
            }
        }
    }

    assert!(
        wrong.is_empty(),
        "{} outstanding key(s) name an owner nobody can act on:\n  {}\n\
         Write `# <goal>` after the key, taking the goal from docs/agent/goals/chain.toml, or \
         `# unowned` with a bullet in docs/agent/carried-gaps.md § Unowned saying why it is \
         nobody's. An owner that went green without striking its key is struck, not renamed.",
        wrong.len(),
        wrong.join("\n  ")
    );
    assert!(
        checked > 0,
        "no ratchet file holds a key, which makes this gate vacuous — either the parity program \
         is finished, in which case delete it, or `outstanding_file` has stopped reading a line"
    );
}

/// The refusal [`every_outstanding_key_names_an_owner`] is written around, asked
/// of the three columns no file on disk writes: a goal the chain does not list,
/// and no column at all.
#[test]
fn an_owner_that_is_not_a_live_chain_entry_fails() {
    let text = chain_text();
    let goals = chain_goals(&text);
    let live = "21".to_owned();
    let unowned = "unowned".to_owned();
    let orphan = "99".to_owned();

    assert!(
        goals.contains(live.as_str()),
        "goal {live} is not on the chain, so this case is asserting nothing — take a live entry \
         from docs/agent/goals/chain.toml"
    );
    assert!(
        !goals.contains(orphan.as_str()),
        "goal {orphan} is on the chain now, so it is no longer an orphan — pick a number no \
         `[[goal]]` uses"
    );

    assert_eq!(owner_problem(Some(&live), &goals), None);
    assert_eq!(owner_problem(Some(&unowned), &goals), None);
    assert!(
        owner_problem(Some(&orphan), &goals).is_some(),
        "an owner naming goal {orphan}, which the chain does not list, was accepted — a key can \
         point at nothing again"
    );
    assert!(
        owner_problem(None, &goals).is_some(),
        "a key with no owner column at all was accepted"
    );
}

/// The other direction every ratchet gate asserts, asked of a key that would be
/// stale: a member §§ 14-19 names and [`registry::CLASSES`] declares.
///
/// The walk does not produce a key for a registered member, so a line naming one
/// is a line a slice forgot to strike — the failure that keeps the list
/// shrinking. Asserting it here means the arithmetic is exercised even in the
/// state the files are usually in, which is one where no line is stale.
#[test]
fn a_key_whose_member_is_now_registered_fails_as_a_stale_line() {
    let outstanding: BTreeSet<String> = part_two_members()
        .into_iter()
        .filter(|member| !registered(&member.candidates, &member.name))
        .map(|member| member.key)
        .collect();

    let struck = "§15 Session::get".to_owned();
    assert!(
        !outstanding.contains(&struck),
        "`{struck}` has no `registry::CLASSES` row, so this case's premise has gone — name \
         another member §§ 14-19 writes and the registry declares"
    );

    let listed: BTreeSet<String> = outstanding
        .iter()
        .cloned()
        .chain([struck.clone()])
        .collect();
    let stale: Vec<&String> = listed.difference(&outstanding).collect();
    assert_eq!(
        stale,
        vec![&struck],
        "a listed key whose member is registered was not reported as stale, so striking a line \
         has stopped being part of the slice that registers the member"
    );
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
            // § 13 is Part I's compiler-facing tail, whose table has no Member
            // column at all — it writes `| Class | Owns | ADR |` and names the
            // members inside an English Owns cell — and §§ 14-19 are Part II,
            // walked by [`every_part_two_spec_member_is_registered`] below.
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

    let Ratchet {
        path, keys: listed, ..
    } = outstanding_file("spec-members-outstanding.txt");
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

/// `rule:core-api/shape-rules` R2's names, held against the one place they are written.
///
/// R2 makes every `Core` parameter callable by "the `$name` the signature in
/// [01-core-library.md] writes", which makes that column the *source* and
/// [`registry::CoreMethod::names`] a mirror of it — and a mirror nothing
/// compares is a copy that drifts. So this walks the same §§ 1-12 tables
/// [`every_part_one_spec_member_is_registered`] does, parses the **Signature**
/// cell rather than the Member cell, and asserts that every registered member
/// the row reaches spells its parameters exactly as the spec does.
///
/// # What is compared, and what is skipped
///
/// A row is resolved as loosely as the coverage test resolves one — the
/// section's classes, narrowed by a span's own qualifier — and then narrowed
/// once more by **arity**, which is what keeps the looseness honest here:
/// § 4's one `at` row is `Core\Time\DateTime`'s five-parameter constructor,
/// and `Date::at`/`TimeOfDay::at` take three, so they are not compared against
/// it rather than compared and failed. A member the spec writes with a
/// different arity than the registry builds is `docs/agent/handoff.md`'s
/// Backlog, not a failure here: `docs/agent/loop-goal.md` § *Standing
/// decisions* says the spec is authoritative for the names and the registry
/// for what is built, and nothing in Stage 0b changes a member's shape.
///
/// The floor on the comparison count is the real assertion about the parser —
/// it makes 234 comparisons today, and one that stops matching the spec's
/// shape fails here rather than passing vacuously. [`cells`] is why that
/// number is not much smaller: splitting a row on every `|` tears the rows
/// declaring a union type into fragments, and a Signature cell torn that way
/// has no closing backtick and reads as no signature at all.
#[test]
fn every_registry_rows_names_are_the_specs_signature_column() {
    let spec = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/spec/01-core-library.md");
    let text = fs::read_to_string(&spec).unwrap_or_else(|err| panic!("{}: {err}", spec.display()));

    let mut section = None;
    let mut candidates = Vec::new();
    let mut in_members = false;
    let mut compared = 0usize;
    let mut wrong = Vec::new();

    for line in text.lines() {
        if let Some(number) = section_number(line) {
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
        let row = cells(line);
        let head = row.first().map_or("", String::as_str);
        if head == "Member" {
            in_members = true;
            continue;
        }
        if head.starts_with("---") || !in_members || row.len() < 2 {
            continue;
        }
        for sig in spans(&row[1]) {
            let Some(want) = signature_names(sig) else {
                continue;
            };
            let Some(name) = member_name(&sig[..sig.find('(').unwrap_or(0)]) else {
                continue;
            };
            for class in scoped(&candidates, sig) {
                for method in class.members().filter(|m| m.name == name) {
                    if method.params.len() != want.len() {
                        continue;
                    }
                    let mut found: Vec<String> =
                        method.names.iter().map(|n| (*n).to_owned()).collect();
                    if method.options().is_some() {
                        found.push(registry::OPTIONS_NAME.to_owned());
                    }
                    compared += 1;
                    if found != want {
                        wrong.push(format!(
                            "§{number} {}::{name} has {found:?}, the spec writes {want:?}",
                            class.name
                        ));
                    }
                }
            }
        }
    }

    assert!(
        compared > 200,
        "{} yielded only {compared} comparison(s), which is too few to be §§ 1-12 — \
         the Signature-column parser has stopped matching the spec's own shape",
        spec.display()
    );
    assert!(
        wrong.is_empty(),
        "{} registered member(s) do not spell their parameters as \
         docs/spec/01-core-library.md does:\n  {}\n\
         The spec's signature column is the source (`rule:core-api/shape-rules` R2) — change the row's `names`, \
         or change the spec and accept that renaming a parameter is a breaking change.",
        wrong.len(),
        wrong.join("\n  ")
    );
}

/// The part of a Part II bullet that is a member list: everything before the
/// first em dash.
///
/// Part II is written "at one line per member" rather than as tables, and the
/// em dash is where every one of those lines turns from *what the class owns*
/// to prose about it — `` `clientIp`, `scheme`, `host` — replacing `$_GET` ``.
/// Cutting there is a typographic rule, not an English one, which is the same
/// line this file's own docs draw for §§ 1-12: a bullet that states its members
/// *after* its dash (§ 15's `Core\Server` and `Core\Cli`) is read only up to
/// it, and undercounts. That direction is the safe one — a member missed here
/// is a member the registry is not asked about, while a fragment of prose
/// mistaken for one would be a ratchet key no session could ever strike.
fn member_list(bullet: &str) -> &str {
    bullet.split_once('—').map_or(bullet, |(list, _)| list)
}

/// `text` with every parenthesised aside that is outside a code span removed.
///
/// Part II writes a member's PHP twin in parentheses directly after the
/// signature — `` `canonicalize(string $path): string` (`realpath`) `` — where
/// §§ 1-12 had a *Replaces* column to put it in, so without this the twin reads
/// as a second member of the same class and becomes a ratchet key nobody can
/// ever strike. A signature's own parentheses are inside its backticks and are
/// never touched, and a markdown link's target goes the same way as a twin,
/// which costs nothing because it holds no code span. An aside carrying an
/// unbalanced `(` *inside* a code span would swallow the rest of the bullet;
/// none does, and the `seen` floor below is what would say so.
fn without_asides(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_span = false;
    let mut depth = 0usize;
    for ch in text.chars() {
        match ch {
            '`' if depth == 0 => {
                in_span = !in_span;
                out.push(ch);
            }
            '(' if !in_span => depth += 1,
            ')' if !in_span && depth > 0 => depth -= 1,
            _ if depth > 0 => {}
            _ => out.push(ch),
        }
    }
    out
}

/// Every registered class inside `name`, itself included — empty when nothing
/// registers that name at all, which is what makes a bullet led by a class no
/// one has written yet outstanding member by member rather than silently
/// answered by a sibling.
fn classes_named(name: &str) -> Vec<&'static registry::CoreClass> {
    let inner = format!(r"{name}\");
    registry::CLASSES
        .iter()
        .filter(|class| class.name == name || class.name.starts_with(&inner))
        .collect()
}

/// One member docs/spec/01-core-library.md §§ 14-19 names, resolved as far as
/// the spec's own shape allows.
struct PartTwoMember {
    /// The classes that may answer for it: the bullet's own `Core\X` and every
    /// class registered inside that namespace, or the section heading's
    /// classes for a bullet that leads with no class, narrowed by [`scoped`]
    /// where the span writes its own qualifier.
    candidates: Vec<&'static registry::CoreClass>,
    /// The bare member name, receiver and qualifier dropped.
    name: String,
    /// `§15 Session::get` — the spelling both gates below report a member
    /// under, and the one the ratchet file lists.
    key: String,
}

/// Every member [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
/// §§ 14-19 names, in the order the file writes them.
///
/// Part II is the capability-bearing half, and it is written in two shapes
/// rather than one. § 19 carries an ordinary `| Member | Signature | … |`
/// table, read exactly as §§ 1-12's are. Everything else states a class and its
/// members in a bullet — `` - `Core\Session`: `get`, `set`, … `` — because the
/// semantics live in each subsystem's own ADR and this file fixes only the
/// roster. So the walk reads both, and [`member_list`] owns where a bullet
/// stops being a roster.
///
/// A bullet that opens with its own `` `Core\X` `` span resolves against that
/// class and the classes inside its namespace **and no others**, which matters
/// far more here than the section-wide looseness §§ 1-12 could afford: § 15
/// puts `Core\Session`, `Core\Env` and `Core\Request` under one heading, and
/// all three own a `get`. The loose reading would let `Core\Env::get` strike
/// two members nobody has written.
///
/// **§§ 16 and 17 are out of scope, on the same line § 13 is.** Both write
/// `| Class | Surface | ADR |` — one row per class, its members inside an
/// English cell beside the prose about them — and a "Replaces `fsockopen`"
/// clause sits in that cell with no dash or column separating it, so reading
/// them means reading English rather than a shape. What that costs is worth
/// naming rather than leaving implicit: nothing checks that `Core\Http\Client`,
/// `Core\RateLimit`, `Core\Metrics`, `Core\Net`, `Core\Crypto`, `Core\Html`,
/// `Core\Xml`, `Core\Compress`, `Core\Zip` or `Core\Mime` has a row for every
/// member the spec gives it — the registry-side gates walk the registry, so
/// they can only ask about the members that *are* registered. The four this
/// walk does read are § 14's and § 15's bullets and § 18's and § 19's Member
/// tables.
fn part_two_members() -> Vec<PartTwoMember> {
    let spec = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/spec/01-core-library.md");
    let text = fs::read_to_string(&spec).unwrap_or_else(|err| panic!("{}: {err}", spec.display()));

    let mut section = None;
    let mut heading: Vec<&'static registry::CoreClass> = Vec::new();
    let mut in_members = false;
    let mut bullet = String::new();
    let mut found: Vec<PartTwoMember> = Vec::new();

    let mut record =
        |number: u32, candidates: &[&'static registry::CoreClass], prefix: &str, span: &str| {
            let head = span[..span.find('(').unwrap_or(span.len())].trim();
            let Some(name) = member_name(head) else {
                return;
            };
            // A Part II bullet names its class's types beside its members —
            // `FileMode` is an enum, `Env\Mode`'s cases are `Production` and
            // `Development` — and `rule:core-api/shape-rules`'s naming is what tells the two apart
            // without reading the sentence they sit in: a member is lowerCamelCase
            // and a constant is SCREAMING_CASE, so an initial capital followed by
            // any lower-case letter is a type and never a member.
            let mut rest = name.chars();
            if rest.next().is_some_and(|first| first.is_ascii_uppercase())
                && rest.any(|later| later.is_ascii_lowercase())
            {
                return;
            }
            found.push(PartTwoMember {
                candidates: scoped(candidates, head),
                name: name.to_owned(),
                key: format!("§{number} {prefix}{head}"),
            });
        };

    for line in text.lines() {
        // A bullet runs until the next one, a blank line, a table or a heading,
        // so every line that is not an indented continuation flushes it first —
        // while `section` and `heading` still hold the ones it was written
        // under.
        let continues = !bullet.is_empty() && line.starts_with("  ") && !line.trim().is_empty();
        if !continues && !bullet.is_empty() {
            if let Some(number) = section {
                let cleaned = without_asides(&bullet);
                let list = member_list(&cleaned).to_owned();
                let led = spans(&list)
                    .first()
                    .copied()
                    .filter(|span| span.starts_with(r"Core\"))
                    .map(str::to_owned);
                let candidates = led
                    .as_deref()
                    .map_or_else(|| heading.clone(), classes_named);
                let prefix = led.as_deref().map_or_else(String::new, |class| {
                    format!("{}::", class.trim_start_matches(r"Core\"))
                });
                for span in spans(&list) {
                    record(number, &candidates, &prefix, span);
                }
            }
            bullet.clear();
        }
        if continues {
            bullet.push(' ');
            bullet.push_str(line.trim());
            continue;
        }
        if let Some(number) = section_number(line) {
            section = (14..=19).contains(&number).then_some(number);
            heading = classes_in(line);
            in_members = false;
            continue;
        }
        let Some(number) = section else { continue };
        if line.starts_with("- ") {
            bullet.push_str(line);
            in_members = false;
            continue;
        }
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
        for span in spans(cell) {
            record(number, &heading, "", span);
        }
    }

    assert!(
        found.len() > 60,
        "{} yielded only {} Part II member(s), which is too few to be §§ 14-19 — \
         the bullet and table parsers have stopped matching the spec's own shape",
        spec.display(),
        found.len()
    );
    found
}

/// [`every_part_one_spec_member_is_registered`]'s other half: every member
/// [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
/// §§ 14-19 names is a member some class in [`registry::CLASSES`] declares.
///
/// [`part_two_members`] owns the walk and what it can and cannot read. This
/// owns the ratchet: `tests/spec-members-part-two-outstanding.txt`, whose keys
/// carry the bullet's class where there is one — `§15 Session::get` — since a
/// bare `§15 get` would name three different members. The module doc above
/// owns why there is a file at all.
#[test]
fn every_part_two_spec_member_is_registered() {
    let outstanding: BTreeSet<String> = part_two_members()
        .into_iter()
        .filter(|member| !registered(&member.candidates, &member.name))
        .map(|member| member.key)
        .collect();

    let Ratchet {
        path, keys: listed, ..
    } = outstanding_file("spec-members-part-two-outstanding.txt");
    let unlisted: Vec<&String> = outstanding.difference(&listed).collect();
    assert!(
        unlisted.is_empty(),
        "{} spec member(s) in §§ 14-19 have no `registry::CLASSES` row and are not listed in {}: {}\n\
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
        "{} line(s) in {} name a member that is registered now, or a key no spec bullet or row \
         produces: {}\n\
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

/// Every class [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
/// §§ 16-17 name in the `Class` column of their tables, keyed by section.
///
/// These are the two sections [`part_two_members`] excludes, and its doc owns
/// why: their members sit inside an English cell that a shape cannot be read
/// off. The **class** column is not English — it is one or more `` `Core\X` ``
/// code spans, one row per class — so a class-level walk is available here
/// where a member-level one is not.
fn part_two_classes() -> BTreeSet<String> {
    let spec = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/spec/01-core-library.md");
    let text = fs::read_to_string(&spec).unwrap_or_else(|err| panic!("{}: {err}", spec.display()));

    let mut section = None;
    let mut found = BTreeSet::new();
    for line in text.lines() {
        if let Some(number) = section_number(line) {
            section = (16..=17).contains(&number).then_some(number);
            continue;
        }
        let Some(number) = section else { continue };
        // The first cell of a table row, which the header row and the `|---|`
        // separator both have too — neither writes a code span, so filtering
        // for one is all the row-shape reading this needs.
        let Some(cell) = line
            .strip_prefix('|')
            .and_then(|rest| rest.split('|').next())
        else {
            continue;
        };
        for span in spans(cell)
            .into_iter()
            .filter(|span| span.starts_with(r"Core\"))
        {
            found.insert(format!("§{number} {span}"));
        }
    }
    found
}

/// The class-level half of [`every_part_two_spec_member_is_registered`], over
/// the two sections that one cannot read: every class §§ 16-17 name has a
/// [`registry::CLASSES`] row, or a key in
/// `tests/spec-classes-part-two-outstanding.txt`.
///
/// This is a weaker claim than the member walk and deliberately so — it cannot
/// see that `Core\Crypto` is missing a member, only that `Core\Crypto` is
/// missing altogether. That is still the failure the two sections were
/// otherwise open to: `conformance_coverage.rs` walks the registry, so a class
/// the spec names and nobody has written is invisible to every other gate in
/// this crate. Ten classes had no gate of any kind before this one.
///
/// The match is **exact**, unlike [`classes_named`]'s: a row for `Core\Xml`
/// asks for a class called `Core\Xml`, and something registered inside its
/// namespace does not answer on its behalf. A class-level roster question is
/// the one place in this file where the looseness the member walks need would
/// cost the claim its meaning.
#[test]
fn every_part_two_spec_class_is_registered() {
    let outstanding: BTreeSet<String> = part_two_classes()
        .into_iter()
        .filter(|key| {
            let name = key.split_once(' ').map_or("", |(_, name)| name);
            !registry::CLASSES.iter().any(|class| class.name == name)
        })
        .collect();

    let Ratchet {
        path, keys: listed, ..
    } = outstanding_file("spec-classes-part-two-outstanding.txt");
    let unlisted: Vec<&String> = outstanding.difference(&listed).collect();
    assert!(
        unlisted.is_empty(),
        "{} class(es) in §§ 16-17 have no `registry::CLASSES` row and are not listed in {}: {}\n\
         Register the class, or add its key to that file if it is genuinely still owed.",
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
        "{} line(s) in {} name a class that is registered now, or a key no spec row produces: \
         {}\n\
         Delete those lines — the list only shrinks, and striking a line is part of the slice \
         that registers the class.",
        stale.len(),
        path.display(),
        stale
            .iter()
            .map(|key| key.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
}

/// The same walk asked of the repository instead of the registry: every member
/// §§ 14-19 names **and [`registry::CLASSES`] declares** is called by a case
/// under `tests/conformance/`.
///
/// This is the third side of a triangle whose other two are already gates, and
/// saying so is cheaper than letting a later session rediscover it:
/// [`every_part_two_spec_member_is_registered`] takes the spec to the registry,
/// `conformance_coverage.rs`'s own two take the registry to the corpus, and a
/// member cannot fail here while passing both. What it holds that neither of
/// them does is the *direction*: those two are anchored on
/// [`registry::CLASSES`], a roster this goal is still adding to, while this one
/// is anchored on the spec, which is the thing being implemented. A member
/// struck off `spec-members-part-two-outstanding.txt` by a session that
/// registered it and stopped fails here **named by its spec section**, which is
/// the sentence that says what to do about it; the registry-side twins would
/// name it by a class and leave the section to be looked up.
///
/// Unregistered members are skipped rather than failed, because the ratchet
/// above is where a member the spec still owes is tracked, and a second list of
/// the same keys would be a second thing to strike.
///
/// `corpus::Attribution::asked` is the rule for "a case calls this", shared
/// with the floor gate so the two cannot drift apart — and it is what lets an
/// instance member be asked for at all: a case reaches `Core\IO\File::read`
/// through whatever `Core\IO::open` answered, naming neither the class nor a
/// receiver type, and the attribution is what carries the class across that. A
/// constant is *named*, not called, so it is checked the way
/// `conformance_coverage.rs` checks one — the whole `Class::NAME`, with the
/// boundary after it.
#[test]
fn every_part_two_member_has_a_conformance_case() {
    let texts = sources();
    let rules = Attribution::new();
    let index: Vec<BTreeMap<&'static str, BTreeSet<&str>>> =
        texts.iter().map(|text| rules.asked(text)).collect();

    let mut uncovered = BTreeSet::new();
    let mut checked = 0usize;
    for member in part_two_members() {
        if !registered(&member.candidates, &member.name) {
            continue;
        }
        checked += 1;
        let covered = member.candidates.iter().any(|class| {
            if class.constant(&member.name).is_some() {
                let write = format!("{}::{}", class.name, member.name);
                texts.iter().any(|text| mentions(text, &write))
            } else {
                index.iter().any(|case| {
                    case.get(class.name)
                        .is_some_and(|named| named.contains(member.name.as_str()))
                })
            }
        });
        if !covered {
            uncovered.insert(member.key);
        }
    }

    assert!(
        checked > 20,
        "only {checked} of §§ 14-19's members are registered, and the ratchet lists {}, so          this gate has stopped reading most of what it should — `registered` or the walk has          regressed, since the ratchet only ever shrinks",
        outstanding_file("spec-members-part-two-outstanding.txt")
            .keys
            .len()
    );
    assert!(
        uncovered.is_empty(),
        "{} spec member(s) in §§ 14-19 are registered and no conformance case calls them: {}\n\
         Write one under tests/conformance/ — never with an `--ORACLE--` section, which makes \
         the Linux leg skip the case entirely. A case reaching the member through a value it \
         was handed counts; one that only names the class does not.",
        uncovered.len(),
        uncovered
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    );
}

/// The registered classes a migration cell's `Core\X` may be answered by.
///
/// The class of that exact name, and nothing else — except for one shape.
/// **A nested name nothing registers is resolved against its own namespace**:
/// `Core\Db\Queryable` is the *interface* § 11 declares, and the spec's own
/// sentence says `Core\Db\Connection` implements it and `Core\Db\Transaction`
/// implements it by delegation, so its members are registered on those two and
/// there is no row of its own for them to be on. Widening to the namespace
/// answers that without this file learning to read which spec headings are
/// interfaces, and it is the same trade [`scoped`] and
/// `conformance_coverage.rs`'s `->text(` already make: looser than a checker,
/// which is what a coverage gate is.
///
/// **A top-level `Core\X` gets no widening**, because its namespace is the
/// whole library — `Core\Process::spawn` would be answered by any class that
/// happens to declare a `spawn`, which is not a resolution but a coincidence.
fn classes_spelled(name: &str) -> Vec<&'static registry::CoreClass> {
    let exact: Vec<_> = registry::CLASSES
        .iter()
        .filter(|class| class.name == name)
        .collect();
    if !exact.is_empty() {
        return exact;
    }
    let Some((namespace, _)) = name.rsplit_once('\\') else {
        return Vec::new();
    };
    if !namespace.contains('\\') {
        return Vec::new();
    }
    let inner = format!(r"{namespace}\");
    registry::CLASSES
        .iter()
        .filter(|class| class.name.starts_with(&inner))
        .collect()
}

/// Every `Core\X::member` spelling the `member` rows of
/// [docs/spec/02-php-migration.md](/docs/spec/02-php-migration.md) name, and
/// how many rows produced them.
///
/// **Every spelling in the cell, not the cell.** That file's *How to read a
/// row* says a cell holding exactly one member is the rename `nvs convert`
/// applies while a cell naming two is prose the converter may not act on — but
/// [ADR 0111](/docs/adr/0111-a-php-builtin-completes-to-its-novis-destination.md)
/// makes the second one *completion items* in an editor, one per member, so
/// both shapes reach a person and both have to resolve. A cell that names a
/// member in passing to say what it is *not* (`array_map`'s zip has no member —
/// a `foreach` over `Core\Arr::keys`) is held to the same rule for the same
/// reason: the name is shown either way.
///
/// The row count comes back with them because the walk is over a document
/// nothing else in this crate parses: a table whose shape drifts would read as
/// zero rows and pass every assertion below vacuously.
fn migration_member_refs() -> (usize, BTreeSet<(String, String)>) {
    let table = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/spec/02-php-migration.md");
    let text =
        fs::read_to_string(&table).unwrap_or_else(|err| panic!("{}: {err}", table.display()));
    // `python tools/check-migration.py`'s own `ROW`, transcribed: a backticked
    // PHP name, a bare outcome word, and the rest of the line up to the closing
    // pipe. Deliberately not [`cells`], which reads a `\` as an escape and so
    // doubles every one in `Core\Str::trim` — that function serves
    // 01-core-library.md's union types, where the escape is real, and this
    // table's Novis cell is the one place a backslash means itself.
    let row = Regex::new(r"^\|\s*`([^`]+)`\s*\|\s*([a-z]+)\s*\|(.*)\|\s*$")
        .expect("the migration table's row");
    // The class half is one or more `\`-joined segments, so `Core\Db\Queryable`
    // is captured whole rather than as `Core\Db` with a stray tail.
    let spelling = Regex::new(r"(Core(?:\\[A-Za-z][A-Za-z0-9]*)+)::([A-Za-z][A-Za-z0-9]*)")
        .expect("the migration table's member spelling");

    let mut rows = 0usize;
    let mut refs = BTreeSet::new();
    for line in text.lines() {
        let Some(found) = row.captures(line.trim_end()) else {
            continue;
        };
        if &found[2] != "member" {
            continue;
        }
        rows += 1;
        for named in spelling.captures_iter(found.get(3).expect("the Novis cell").as_str()) {
            refs.insert((named[1].to_owned(), named[2].to_owned()));
        }
    }
    (rows, refs)
}

/// The count below which the walk has stopped reading the table rather than
/// found it clean — the file carried 520 `member` rows when this landed, and
/// half of that is a shape change nobody meant.
const MIGRATION_ROW_FLOOR: usize = 260;

#[test]
fn every_migration_member_row_names_a_registered_member() {
    let (rows, refs) = migration_member_refs();
    assert!(
        rows > MIGRATION_ROW_FLOOR,
        "docs/spec/02-php-migration.md produced {rows} `member` row(s), under the floor of \
         {MIGRATION_ROW_FLOOR} — the table's shape has changed under `cells`, and every \
         assertion here is passing vacuously rather than passing"
    );

    let outstanding: BTreeSet<String> = refs
        .iter()
        .filter(|(class, member)| !registered(&classes_spelled(class), member))
        .map(|(class, member)| format!("{class}::{member}"))
        .collect();

    let Ratchet {
        path, keys: listed, ..
    } = outstanding_file("migration-members-outstanding.txt");
    let unlisted: Vec<&String> = outstanding.difference(&listed).collect();
    assert!(
        unlisted.is_empty(),
        "{} member spelling(s) in docs/spec/02-php-migration.md name nothing \
         `registry::CLASSES` holds and are not listed in {}: {}\n\
         Register the member (five things — see docs/agent/conventions.md), correct the cell, \
         or add the spelling to that file if the member is genuinely still owed.",
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
        "{} line(s) in {} name a member that is registered now, or a spelling no `member` row \
         produces: {}\n\
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

#[test]
fn every_migration_member_row_has_a_conformance_case() {
    let (_, refs) = migration_member_refs();
    let texts = sources();
    let rules = Attribution::new();
    let index: Vec<BTreeMap<&'static str, BTreeSet<&str>>> =
        texts.iter().map(|text| rules.asked(text)).collect();

    let mut uncovered = BTreeSet::new();
    let mut checked = 0usize;
    for (name, member) in &refs {
        // A spelling nothing registers is the test above's business, and
        // listing it here as well would report one gap twice and make its
        // ratchet the only file that can be wrong about it.
        let candidates = classes_spelled(name);
        if !registered(&candidates, member) {
            continue;
        }
        checked += 1;
        let covered = candidates.iter().any(|class| {
            if class.constant(member).is_some() {
                // A constant has no call spelling, so the whole written name
                // with its right boundary is the check — `Core\Math::E` is not
                // covered by a case writing `Core\Math::EPSILON`.
                let write = format!("{}::{}", class.name, member);
                texts.iter().any(|text| mentions(text, &write))
            } else if class.members().any(|m| m.name == member.as_str()) {
                index.iter().any(|case| {
                    case.get(class.name)
                        .is_some_and(|named| named.contains(member.as_str()))
                })
            } else {
                false
            }
        });
        if !covered {
            uncovered.insert(format!("{name}::{member}"));
        }
    }

    assert!(
        checked > 150,
        "only {checked} of the migration table's member spellings resolve to a registered \
         member, and the ratchet lists {} — the walk or `classes_spelled` has regressed, since \
         that list only ever shrinks",
        outstanding_file("migration-members-outstanding.txt")
            .keys
            .len()
    );
    assert!(
        uncovered.is_empty(),
        "{} member(s) docs/spec/02-php-migration.md promises a migrating program are registered \
         and no conformance case calls them: {}\n\
         Write one under tests/conformance/ — never with an `--ORACLE--` section, which makes \
         the Linux leg skip the case entirely. A case reaching the member through a value it \
         was handed counts; one that only names the class does not.",
        uncovered.len(),
        uncovered
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    );
}
