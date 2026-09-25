//! A coverage gate: every member
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
//! [`every_compiler_facing_spec_member_is_registered`] asks it of § 13, whose
//! `| Class | Owns | ADR |` rows put a class's roster and a paragraph about it
//! in one cell. [`compiler_facing_members`] owns the rule that tells the two
//! apart — a span is a member when it writes a signature or a generic, or when
//! commas and slashes alone join it to one that does — and
//! [`the_compiler_facing_walk_reads_a_roster_and_not_the_prose_beside_it`] pins
//! both directions of it, since a walk reading the whole cell would report a
//! *Replaces* clause's PHP twins as members nobody has written.
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
//! is empty, every member 1,167 PHP functions were pointed at exists.
//!
//! They are here rather than in `conformance_coverage.rs` because the walk is a
//! spec walk — the table is the enumerable set and the registry is what it is
//! held against, which is this file's direction and not that one's.
//!
//! # The outstanding list is a file, and it only shrinks
//!
//! A section the registry does not yet hold whole is the ordinary state of a
//! spec being implemented — `crates/nvs-stdlib`'s own module doc says which
//! ones those are — so the honest reading of "fails naming every one with no
//! registry entry" is a test that is red for the whole of the loop that exists
//! to make it green. That trade was refused:
//! `nv verify` stops at the first failing step, so a permanently red
//! `cargo test` costs every later session its clippy and fmt signal, which is
//! a much larger loss than the one it buys. The acceptance gate withholds
//! *done* on the conformance suite's `minPassing` floor in the goal records
//! under `data/goals/` regardless, and that number is untouched by anything
//! here.
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
//! When one of these files is empty, its emptiness *is* the sentence at the top
//! of this doc for the sections that file walks.
//!
//! # An outstanding key names its owner, in a column
//!
//! Every key in every one of these files carries `# <owner>` after it: the goal
//! from [the goals directory](/docs/agent/goals/) that will strike the line, or
//! a milestone ahead of the program whose own plan carries the work.
//! [`every_outstanding_key_names_an_owner`] is what makes that a field rather
//! than a note — it reads the chain and the plan's table and fails on an owner
//! neither of them answers for, so a goal renamed or dropped cannot leave a key
//! pointing at nothing. [`owner_problem`] is where the two kinds are decided,
//! and `tools/nv/cmd/owners.ts`'s module doc states the same kinds for a gap
//! record.
//!
//! **The word `unowned` is refused by name**, and
//! [`unowned_is_no_longer_an_owner_for_a_key`] is what holds it refused. A key
//! whose owner is a scheduling question puts that question to the user, and
//! what comes back is a goal on the chain or a milestone whose plan states the
//! scope; a word standing for "nobody has decided" is the one owner a reader
//! cannot act on. `tools/nv/cmd/owners.ts` refuses the same word as the owner
//! of a gap record under `data/gaps/`, and this file is the ratchet-file half
//! of that rule.
//!
//! The column exists because these facts were header prose, where one paragraph
//! owned eight keys and could not say which was which. The failure it exists to
//! stop is on record: `§18 stream` read as "goal `database`'s" for six goals
//! after goal `database` closed. `tools/nv/cmd/owners.ts` holds the same rule
//! for a gap record under `data/gaps/`, which is the other half of the contract.
//!
//! What the gate deliberately does not check is whether an owner is still
//! *ahead*. A chain entry that goes green without striking its key is the more
//! interesting failure and it is a reader's to catch, because the chain file
//! holds the order and not the position — nothing on disk says where the loop
//! is. An owner that goes green without striking its key is struck and never
//! renamed, and what it is struck for is whatever carries the work now.
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
            // Held rather than written, and written below only where it turns
            // out to have escaped nothing: a cell naming `Core\Reflect` holds
            // one backslash and not two, which is what `classes_named` is
            // asked about.
            '\\' if !escaped => {
                escaped = true;
                continue;
            }
            '|' if !escaped => out.push(String::new()),
            _ => {
                if escaped && c != '|' {
                    out.last_mut().expect("a cell").push('\\');
                }
                out.last_mut().expect("a cell").push(c);
            }
        }
        escaped = false;
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
/// There is one per walk — §§ 1-12's, § 13's, §§ 14-19's, §§ 16-17's classes
/// and the migration table's — because the halves are finished by different
/// loops and a single file would make a Part I regression indistinguishable
/// from a Part II member nobody has reached yet.
const RATCHETS: [&str; 5] = [
    "spec-members-outstanding.txt",
    "spec-members-compiler-facing-outstanding.txt",
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

/// The chain — `data/chain.json`'s `goals`, the list of goal slugs in the order
/// the loop walks them — read for the one thing an owner column is checked
/// against: which goals it still holds. The **slug** is what an owner column
/// names, because a goal's position moves whenever anything is inserted in front of it.
fn chain_goals() -> BTreeSet<String> {
    let path = nvs_repo::path("data/chain.json");
    let text = fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    let chain: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    let goals = chain["goals"]
        .as_array()
        .unwrap_or_else(|| panic!("{}: `goals` is not a list", path.display()));
    goals
        .iter()
        .map(|g| {
            g.as_str()
                .unwrap_or_else(|| panic!("{}: a goal that is not a slug: {g}", path.display()))
                .to_owned()
        })
        .collect()
}

/// The first milestone a key may be deferred to.
///
/// Everything before it is complete at the end of the program, so a key tagged
/// to an earlier one names a carrier that has been and gone rather than work
/// that is scheduled. `tools/nv/cmd/owners.ts`'s `FIRST_FUTURE_MILESTONE` is the
/// same number for the same reason over the gap records; there is no file both
/// sides can read it from, so each states it.
const FIRST_FUTURE_MILESTONE: u32 = 9;

/// Every milestone in the plan's table, mapped to its *Carried by* cell.
///
/// The row shape is the carrier cell first, then
/// the milestone as a link to its own file under `docs/plan/`. That cell is the
/// only place a milestone is finished — the plan's own § under the table says
/// so — and `done` is the whole vocabulary for it.
fn plan_milestones() -> BTreeMap<String, String> {
    let path = nvs_repo::path("docs/implementation-plan.md");
    let text = fs::read_to_string(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    let row = Regex::new(r"^\|\s*([^|]*?)\s*\|\s*\[(M\d+[A-Z]?)\]").expect("the plan's table row");
    let mut found = BTreeMap::new();
    for line in text.lines() {
        if let Some(cells) = row.captures(line) {
            found.insert(cells[2].to_owned(), cells[1].to_owned());
        }
    }
    found
}

/// The number a milestone tag names, or `None` when the owner is not a tag.
///
/// A suffixed milestone is its number's — `M4S` and `M4B` are both M4's, which
/// is what the suffix means in the plan's table — so the suffix is read and
/// dropped rather than making the tag unparseable. The shape is
/// `tools/nv/cmd/owners.ts`'s `MILESTONE`, and a goal slug cannot collide with it: a
/// slug is lower case throughout.
fn milestone_number(owner: &str) -> Option<u32> {
    let rest = owner.strip_prefix('M')?;
    let digits = match rest.strip_suffix(|c: char| c.is_ascii_uppercase()) {
        Some(head) => head,
        None => rest,
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// What is wrong with one key's owner column, or `None` if nothing is.
///
/// Two kinds pass here: a goal
/// the chain still holds, named by its slug, and a milestone tag. A milestone
/// owns a key the way it owns a gap —
/// the work is scheduled rather than missing — and the three things that make a
/// tag empty are the same three `tools/nv/cmd/owners.ts:@classify` asks about, in its
/// order. A tag the plan's table does not list names no milestone at all; one
/// before [`FIRST_FUTURE_MILESTONE`] is behind the program, so nothing is left
/// to carry the key; and a row whose *Carried by* cell reads `done` closed
/// without closing this.
///
/// `unowned` is asked about ahead of both and refused with a sentence of its
/// own, because "this owner kind is retired" is the edit a reader has to make,
/// where the generic refusal would send them looking for a typo in a word that
/// is spelled correctly.
///
/// It is a function rather than a `match` inside the gate so that
/// [`an_owner_that_is_not_a_live_chain_entry_fails`] can ask it about an owner
/// no file on disk writes: a refusal nothing ever exercises is a refusal that
/// can stop refusing without anything going red. The table arrives as an
/// argument for that reason too — a `done` future milestone is a row the plan
/// does not currently hold.
fn owner_problem(
    owner: Option<&String>,
    goals: &BTreeSet<String>,
    plan: &BTreeMap<String, String>,
) -> Option<String> {
    let Some(owner) = owner else {
        return Some("names no owner".to_owned());
    };
    if owner == "unowned" {
        return Some(
            "names `unowned`, which is a retired owner kind — a key is owned by a live goal on \
             the chain or by a milestone whose plan states the scope"
                .to_owned(),
        );
    }
    if goals.contains(owner.as_str()) {
        return None;
    }
    if let Some(number) = milestone_number(owner) {
        let Some(carried) = plan.get(owner.as_str()) else {
            return Some(format!(
                "names milestone {owner}, which is no row in the plan's table"
            ));
        };
        if number < FIRST_FUTURE_MILESTONE {
            return Some(format!(
                "names milestone {owner}, which is behind the program — only \
                 M{FIRST_FUTURE_MILESTONE} and later is a deferral"
            ));
        }
        if carried.to_lowercase().starts_with("done") {
            return Some(format!(
                "names milestone {owner}, which the plan's table marks done"
            ));
        }
        return None;
    }
    Some(format!(
        "names goal `{owner}`, which is no goal in data/chain.json"
    ))
}

/// Every key in every [`RATCHETS`] file names an owner a reader can act on: a
/// goal [docs/agent/goals/](/docs/agent/goals/) still holds, named by its slug,
/// or a milestone ahead of the program.
///
/// [`owner_problem`] owns which two those are, and the module doc owns why the
/// owner is a column rather than a header sentence.
#[test]
fn every_outstanding_key_names_an_owner() {
    let goals = chain_goals();
    let plan = plan_milestones();
    assert!(
        goals.len() > 10,
        "data/chain.json yielded only {} goal(s) — the chain's shape has changed under this \
         walk, and every owner below is being accepted against almost nothing",
        goals.len()
    );
    assert!(
        plan.len() > 10,
        "docs/implementation-plan.md's table yielded only {} milestone(s) — the row shape has \
         moved under `plan_milestones`, and every milestone owner below is being refused for the \
         wrong reason",
        plan.len()
    );

    let mut wrong = Vec::new();
    for name in RATCHETS {
        let ratchet = outstanding_file(name);
        for key in &ratchet.keys {
            if let Some(problem) = owner_problem(ratchet.owners.get(key), &goals, &plan) {
                wrong.push(format!("{name}: `{key}` {problem}"));
            }
        }
    }

    assert!(
        wrong.is_empty(),
        "{} outstanding key(s) name an owner nobody can act on:\n  {}\n\
         Write `# <goal-slug>` after the key, taking the slug from docs/agent/goals/, or \
         `# M<n>` naming a milestone the plan's table still carries. An owner that went \
         green without striking its key is struck, not renamed.",
        wrong.len(),
        wrong.join("\n  ")
    );
    // **Every key is struck**: §§ 1-19 and the migration table are walked, and all four files
    // hold their header and nothing else. That is the state a ratchet is built to arrive at, so
    // reaching it is not a failure and the loop above having nothing to check is not one either
    // — what `owner_problem`'s own cases below assert is the arithmetic, and they feed it owners
    // no file has to hold, so it stays exercised with the worklist at zero.
    //
    // The files stay. A ratchet is how the *next* section of the spec is walked, and a deleted
    // one would make a member that regressed out of the registry read as an empty worklist
    // rather than as a key nobody owns. What is left to assert about an empty one is that it is
    // still a file saying what it is for.
    for name in RATCHETS {
        let ratchet = outstanding_file(name);
        let text = fs::read_to_string(&ratchet.path)
            .unwrap_or_else(|err| panic!("{}: {err}", ratchet.path.display()));
        assert!(
            text.trim_start().starts_with('#'),
            "{} has lost the header saying which walk fills it and what a line means, so the \
             next key written into it goes into a file nobody can read a purpose off",
            ratchet.path.display()
        );
    }
}

/// The refusal [`every_outstanding_key_names_an_owner`] is written around, asked
/// of the columns no file on disk writes: a goal the chain does not list, a
/// milestone the plan's table has no row for, a milestone whose row reads
/// `done`, and no column at all.
///
/// The `done` case is handed a table of its own, because every milestone the
/// plan currently marks `done` is also behind the program and would be refused
/// one branch earlier. [`a_future_milestone_owns_a_key_and_a_past_one_does_not`]
/// is the half that reads the real table.
#[test]
fn an_owner_that_is_not_a_live_chain_entry_fails() {
    let goals = chain_goals();
    let plan = plan_milestones();
    let live = "carried-gaps".to_owned();
    let orphan = "no-such-goal".to_owned();
    let unplanned = "M99".to_owned();
    let finished = "M9".to_owned();

    assert!(
        goals.contains(live.as_str()),
        "goal `{live}` is not on the chain, so this case is asserting nothing — take a slug from \
         docs/agent/goals/"
    );
    assert!(
        !goals.contains(orphan.as_str()),
        "goal `{orphan}` is on the chain now, so it is no longer an orphan — pick a slug no goal \
         file uses"
    );

    assert!(
        !plan.contains_key(unplanned.as_str()),
        "the plan's table now carries {unplanned}, so this case is asserting nothing — take a tag \
         no row writes"
    );

    assert_eq!(owner_problem(Some(&live), &goals, &plan), None);
    assert!(
        owner_problem(Some(&orphan), &goals, &plan).is_some(),
        "an owner naming goal `{orphan}`, which the chain does not hold, was accepted — a key can \
         point at nothing again"
    );
    assert!(
        owner_problem(Some(&unplanned), &goals, &plan).is_some(),
        "an owner naming milestone {unplanned}, which the plan's table has no row for, was \
         accepted — a key can be deferred to a milestone nobody has planned"
    );
    let done = BTreeMap::from([(finished.clone(), "done".to_owned())]);
    assert!(
        owner_problem(Some(&finished), &goals, &done).is_some(),
        "an owner naming milestone {finished}, whose row reads `done`, was accepted — a milestone \
         that finished without closing this key still reads as its carrier"
    );
    assert!(
        owner_problem(None, &goals, &plan).is_some(),
        "a key with no owner column at all was accepted"
    );
}

/// `unowned` is refused for being itself, and not for looking like a goal slug
/// the chain does not hold.
///
/// The word is slug-shaped, so an implementation that simply dropped the branch
/// would still go red here — with a message telling a reader to check the
/// spelling of a word that is spelled correctly. The message is what this asks
/// about for that reason, rather than only the refusal.
#[test]
fn unowned_is_no_longer_an_owner_for_a_key() {
    let goals = chain_goals();
    let plan = plan_milestones();
    let unowned = "unowned".to_owned();

    assert!(
        !goals.contains(unowned.as_str()),
        "a goal named `{unowned}` is on the chain, so the refusal below is asserting the wrong \
         thing — rename the goal"
    );

    let problem = owner_problem(Some(&unowned), &goals, &plan)
        .expect("`unowned` was accepted as an owner — the retired kind owns keys again");
    assert!(
        problem.contains("retired"),
        "`unowned` is refused as a goal the chain does not hold rather than as a retired kind, \
         so the message reads as a typo to fix: {problem}"
    );
}

/// The owner kind stage 3 of goal `gap-register` adds: a key deferred to a
/// milestone ahead of the program is scheduled work, and one deferred to a
/// milestone behind it is not.
///
/// Both tags are read from the real table, because the claim is about the plan
/// as it stands: M9 and later are what the program has left, and M1 is carried
/// by goals that are walking now. The `milestone` and `past` kinds in
/// `tools/nv/cmd/owners.ts`'s module doc are the rule, and the user's decision
/// behind it is goal `gap-register` § *Standing decisions*.
#[test]
fn a_future_milestone_owns_a_key_and_a_past_one_does_not() {
    let goals = chain_goals();
    let plan = plan_milestones();
    let future = format!("M{FIRST_FUTURE_MILESTONE}");
    let past = format!("M{}", FIRST_FUTURE_MILESTONE - 1);

    for tag in [&future, &past] {
        assert!(
            plan.contains_key(tag.as_str()),
            "the plan's table has no row for {tag}, so this case is asserting nothing about the \
             milestone either side of M{FIRST_FUTURE_MILESTONE}"
        );
    }

    assert_eq!(
        owner_problem(Some(&future), &goals, &plan),
        None,
        "a key deferred to {future}, which the plan still carries, was refused — a milestone ahead \
         of the program owns a key the way it owns a gap"
    );
    assert!(
        owner_problem(Some(&past), &goals, &plan).is_some(),
        "a key deferred to {past} was accepted — everything before M{FIRST_FUTURE_MILESTONE} is \
         complete at the end of the program, so the tag names a carrier that has been and gone"
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
    let spec = nvs_repo::path("docs/spec/01-core-library.md");
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
/// different arity than the registry builds is not a failure here:
/// `docs/agent/goals/concurrency.md` § *Standing decisions* says the spec is
/// authoritative for the names and the registry for what is built.
///
/// The floor on the comparison count is the real assertion about the parser —
/// it makes 234 comparisons today, and one that stops matching the spec's
/// shape fails here rather than passing vacuously. [`cells`] is why that
/// number is not much smaller: splitting a row on every `|` tears the rows
/// declaring a union type into fragments, and a Signature cell torn that way
/// has no closing backtick and reads as no signature at all.
#[test]
fn every_registry_rows_names_are_the_specs_signature_column() {
    let spec = nvs_repo::path("docs/spec/01-core-library.md");
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

/// The type a `| Type | Members … |` row is about: its first cell's own code
/// span, with any type argument dropped.
///
/// `Rows<T>` is one class at one argument and the registry knows it under its
/// bare name, so the argument is what this drops rather than something it has
/// to resolve.
fn typed_row_name(cell: &str) -> Option<&str> {
    let span = *spans(cell).first()?;
    Some(span.split_once('<').map_or(span, |(name, _)| name))
}

/// `candidates`, narrowed to the class a `| Type | Members … |` row names.
///
/// [`scoped`]'s narrowing keyed off the row's own first cell instead of off a
/// qualifier written inside a member span, and it falls back the same way for
/// the same reason: § 18's `InList` is a type the registry declares no class
/// for, and that row still names a member — `Db::inList` — that a class in the
/// section does declare, which [`scoped`] then resolves off the span itself.
fn typed_row_classes<'a>(
    candidates: &[&'a registry::CoreClass],
    name: &str,
) -> Vec<&'a registry::CoreClass> {
    let suffix = format!(r"\{name}");
    let narrowed: Vec<&'a registry::CoreClass> = candidates
        .iter()
        .copied()
        .filter(|found| found.name == name || found.name.ends_with(&suffix))
        .collect();
    if narrowed.is_empty() {
        candidates.to_vec()
    } else {
        narrowed
    }
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
/// **§§ 16 and 17 are out of scope.** Both write `| Class | Surface | ADR |` —
/// one row per class, its members inside an English cell beside the prose about
/// them — and a "Replaces `fsockopen`" clause sits in that cell with no dash or
/// column separating it, where § 13's otherwise identical shape puts its roster
/// in one run of commas and slashes that [`compiler_facing_members`] can find an
/// end to. What that costs is worth naming rather than leaving implicit:
/// nothing checks that `Core\Http\Client`,
/// `Core\RateLimit`, `Core\Metrics`, `Core\Net`, `Core\Crypto`, `Core\Html`,
/// `Core\Xml`, `Core\Compress`, `Core\Zip` or `Core\Mime` has a row for every
/// member the spec gives it — the registry-side gates walk the registry, so
/// they can only ask about the members that *are* registered. The four this
/// walk does read are § 14's and § 15's bullets, § 18's and § 19's Member
/// tables, and § 18's two `| Type | Members … |` tables — the shape that
/// states a roster per *type* rather than a row per member, and the one that
/// left `Connection::serverVersion` enumerated by nothing until
/// [`every_member_beyond_queryable_is_enumerated_from_the_spec`] pinned it.
fn part_two_members() -> Vec<PartTwoMember> {
    let spec = nvs_repo::path("docs/spec/01-core-library.md");
    let text = fs::read_to_string(&spec).unwrap_or_else(|err| panic!("{}: {err}", spec.display()));

    let mut section = None;
    let mut heading: Vec<&'static registry::CoreClass> = Vec::new();
    let mut in_members = false;
    let mut members_at: Option<usize> = None;
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
            members_at = None;
            continue;
        }
        if !line.starts_with('|') {
            in_members = false;
            members_at = None;
            continue;
        }
        let row = cells(line);
        let cell = row.first().map_or("", String::as_str);
        if cell == "Member" {
            in_members = true;
            members_at = None;
            continue;
        }
        // § 18 states a class's members two ways, and this is the second: a
        // table whose rows are *types* and whose member column holds the whole
        // roster of one. Read by the column its header names rather than by
        // position, because the two such tables are three columns and two —
        // and reading the other columns would take `Replaces`' PHP twins for
        // members of the very class they are being replaced on.
        if cell == "Type" {
            in_members = false;
            members_at = row.iter().position(|head| head.starts_with("Members"));
            continue;
        }
        if cell.starts_with("---") {
            continue;
        }
        if in_members {
            for span in spans(cell) {
                record(number, &heading, "", span);
            }
            continue;
        }
        let Some(listed) = members_at.and_then(|at| row.get(at)) else {
            continue;
        };
        let Some(named) = typed_row_name(cell) else {
            continue;
        };
        // The bullet path's own two steps, in its order and for its reasons:
        // § 18's `Transaction` row states what `rollBack` *does* after an em
        // dash, and a parenthesised aside beside a member is prose either way.
        let cleaned = without_asides(listed);
        let candidates = typed_row_classes(&heading, named);
        let prefix = format!("{named}::");
        for span in spans(member_list(&cleaned)) {
            // A span qualified with some *other* class is a cross-reference and
            // not one of this row's members: § 18's `InList` row is a sentence
            // saying the type is opaque and "produced by `Db::inList`", which
            // is a member of `Core\Db`. Read as a member of the row's own type
            // it would be a ratchet key no session could ever strike, which is
            // [`member_list`]'s warning about prose in a roster's clothes.
            if qualifier(span).is_some_and(|class| class != named) {
                continue;
            }
            // The receiver form these cells are written in — `$c->close()`,
            // `->driver()` — is dropped here rather than inside [`record`], so
            // the key reads `§18 Connection::close` and the ratchet's keys stay
            // the member spellings the rest of this file's do.
            let member = span.split_once("->").map_or(span, |(_, rest)| rest);
            record(number, &candidates, &prefix, member);
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

/// The members § 18 states in its *Members beyond `Queryable`* table reach the
/// walk, resolved to the class each row is about.
///
/// **A gate over a parser needs a case that knows what the parser must find**,
/// and this table is why: the walk turned its member reading on at a header
/// cell reading `Member`, so a table headed `Type` was walked past in silence
/// and every member in it — `close`, `driver`, `serverVersion`, `isOpen` and a
/// transaction's `rollBack` — was enumerated by nothing at all. Neither gate
/// below could report that, because both ask whether what was *found* is
/// registered and a member that is never found is never asked about.
///
/// Both halves are asserted, because either alone passes on a walk that is
/// wrong: that the key exists says the table is read, and that its candidates
/// declare the member says the row resolved to the class it is about rather
/// than to the section's whole namespace, which is what would let a `Rows`
/// member answer for a `Connection` one.
#[test]
fn every_member_beyond_queryable_is_enumerated_from_the_spec() {
    let found = part_two_members();

    for key in [
        "§18 Connection::close",
        "§18 Connection::driver",
        "§18 Connection::serverVersion",
        "§18 Connection::isOpen",
        "§18 Transaction::rollBack",
    ] {
        let member = found
            .iter()
            .find(|member| member.key == key)
            .unwrap_or_else(|| {
                panic!(
                    "the walk over docs/spec/01-core-library.md § 18 did not enumerate `{key}` — a \
                 `| Type | Members … |` table states a roster per type, and one the walk cannot \
                 read is a roster nothing gates"
                )
            });
        assert!(
            registered(&member.candidates, &member.name),
            "`{key}` resolves to {} candidate class(es), none of which declares `{}`",
            member.candidates.len(),
            member.name
        );
    }
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
    let spec = nvs_repo::path("docs/spec/01-core-library.md");
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

/// Every code span in `text`, paired with the run of text between it and the
/// span before it — `("forObject", ", ")`, and `""` for the first.
///
/// [`spans`] is the same walk without the gaps, and the gap is what
/// [`a_roster_separator`] reads: § 13 states a roster and a sentence about it
/// in one cell, and the only thing telling the two apart is what sits between
/// two spans.
fn spans_with_gaps(text: &str) -> Vec<(&str, &str)> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('`') {
        let gap = &rest[..open];
        let after = &rest[open + 1..];
        let Some(close) = after.find('`') else { break };
        found.push((&after[..close], gap));
        rest = &after[close + 1..];
    }
    found
}

/// Whether the text between two code spans joins them into one roster rather
/// than ending the clause the first one was in.
///
/// A roster is written `` `a`, `b` and `c` `` or `` `a`/`b` ``, with bold on the
/// conjunction in one § 13 row, so a separator is commas, slashes, whitespace,
/// emphasis and the two conjunctions and nothing else. Every other gap — a full
/// stop, an em dash, any word at all — ends the roster, which is what keeps a
/// *Replaces* clause's PHP twins and a sentence's `decimal` out of the walk.
fn a_roster_separator(between: &str) -> bool {
    between
        .split(|c: char| c.is_whitespace() || c == ',' || c == '/' || c == '*')
        .all(|word| word.is_empty() || word == "and" || word == "or")
}

/// One key docs/spec/01-core-library.md § 13 produces: a member one of its rows
/// names, or a row's class where the row names no member at all.
struct CompilerFacingMember {
    /// Whether the registry answers for it — some candidate class declares the
    /// member, or a class of that exact name is registered.
    declared: bool,
    /// `§13 Reflect::forClass`, or `§13 Core\BigInt` for a class-only row. The
    /// spelling the ratchet file lists.
    key: String,
}

/// Every member [docs/spec/01-core-library.md](/docs/spec/01-core-library.md)
/// § 13 names, resolved to the classes the row naming it is about.
///
/// § 13 is a third shape again: `| Class | Owns | ADR |`, one row per class,
/// with that class's roster and a paragraph about it sharing the *Owns* cell.
/// That is the shape §§ 16-17 are excluded on, and reading it off position
/// alone is what a naive walk gets wrong — `Core\Decimal`'s cell says "the
/// non-operator members of the `decimal` scalar", `Core\Command`'s ends "no
/// conversion from `string` are compile errors", and a *Replaces* clause puts
/// eleven PHP function names in backticks beside `Core\Reflect`'s three
/// members.
///
/// So a span is a member when it **anchors** — it writes a signature or a
/// generic argument, `` `typeOf(mixed): TypeKind` ``, `` `get<T>` `` — or when
/// an unbroken run of [`a_roster_separator`] gaps joins it to one that does.
/// `` `forClass`, `forObject`, `typeOf(…)` `` is one roster and all three are
/// read; `` `decimal` `` anchors nothing and neighbours nothing, and the full
/// stop before *Replaces* ends the only run that could have reached its twins.
/// A span that is not a member name at all, or that names a type by
/// `rule:core-api/shape-rules`' casing, is never accepted and **breaks the run**, so a
/// roster cannot reach across one.
///
/// A row whose cell names no member is keyed by its class instead, because the
/// roster is the only claim a row makes and a class stated as English —
/// `Core\Decimal` and `Core\BigInt` share one — would otherwise be checked by
/// nothing. A row that does name members needs no class key: its members are
/// resolved through [`classes_named`], which is empty for a class nobody has
/// written, so every one of them is outstanding already.
fn compiler_facing_members() -> Vec<CompilerFacingMember> {
    let spec = nvs_repo::path("docs/spec/01-core-library.md");
    let text = fs::read_to_string(&spec).unwrap_or_else(|err| panic!("{}: {err}", spec.display()));

    let mut in_section = false;
    let mut found: Vec<CompilerFacingMember> = Vec::new();
    for line in text.lines() {
        if let Some(number) = section_number(line) {
            in_section = number == 13;
            continue;
        }
        // Only a table row, so the prose under the table — which writes
        // `Core\Reflect::typeOf`, `#[Test]` and `unserialize` in the same
        // backticks — is not a roster this walk can be told apart from.
        if !in_section || !line.starts_with('|') {
            continue;
        }
        let row = cells(line);
        let (Some(class_cell), Some(owns)) = (row.first(), row.get(1)) else {
            continue;
        };
        let named: Vec<&str> = spans(class_cell)
            .into_iter()
            .filter(|span| span.starts_with(r"Core\"))
            .collect();
        if named.is_empty() {
            continue;
        }
        let mut candidates: Vec<&'static registry::CoreClass> = Vec::new();
        for name in &named {
            for class in classes_named(name) {
                if !candidates.iter().any(|held| held.name == class.name) {
                    candidates.push(class);
                }
            }
        }
        let prefix = format!("{}::", named[0].trim_start_matches(r"Core\"));

        // A parenthesised aside goes the way it does in Part II's bullets, and
        // § 13 needs it more: `Core\Router` writes four of them, one holding a
        // markdown link and one holding `url` itself.
        let cleaned = without_asides(owns);
        let listed = spans_with_gaps(&cleaned);
        let names: Vec<Option<&str>> = listed
            .iter()
            .map(|(span, _)| {
                let head = span[..span.find(['(', '<']).unwrap_or(span.len())].trim();
                let name = member_name(head)?;
                let mut rest = name.chars();
                let a_type = rest.next().is_some_and(|first| first.is_ascii_uppercase())
                    && rest.any(|later| later.is_ascii_lowercase());
                (!a_type).then_some(name)
            })
            .collect();
        let mut accepted: Vec<bool> = listed
            .iter()
            .zip(&names)
            .map(|((span, _), name)| name.is_some() && span.contains(['(', '<']))
            .collect();
        // A run reaches its anchor from either side — `forClass` is two commas
        // to the left of `typeOf(…)`, `assertSame` one slash — so the pass runs
        // forwards and then backwards, and a span that is not a member is never
        // accepted and so never carries acceptance across itself.
        for index in 1..listed.len() {
            accepted[index] = accepted[index]
                || (names[index].is_some()
                    && accepted[index - 1]
                    && a_roster_separator(listed[index].1));
        }
        for index in (0..listed.len().saturating_sub(1)).rev() {
            accepted[index] = accepted[index]
                || (names[index].is_some()
                    && accepted[index + 1]
                    && a_roster_separator(listed[index + 1].1));
        }

        let mut any = false;
        for (index, name) in names.iter().enumerate() {
            let (Some(name), true) = (name, accepted[index]) else {
                continue;
            };
            // A span qualified with some other class is a cross-reference and
            // not one of this row's members, exactly as it is in Part II's
            // tables: `Core\Router`'s cell says `Core\Request::route()` is the
            // match it produced, and read as a `Router` member that would be a
            // ratchet key no session could ever strike.
            if qualifier(listed[index].0).is_some_and(|class| !named.contains(&class)) {
                continue;
            }
            any = true;
            found.push(CompilerFacingMember {
                declared: registered(&candidates, name),
                key: format!("§13 {prefix}{name}"),
            });
        }
        if !any {
            for name in named {
                found.push(CompilerFacingMember {
                    declared: registry::CLASSES.iter().any(|class| class.name == name),
                    key: format!("§13 {name}"),
                });
            }
        }
    }

    found
}

/// The § 13 walk reads the rosters and stops where the prose beside them
/// starts.
///
/// **A gate over a parser needs a case that knows what the parser must find**,
/// and here it needs the other half too: a walk that accepted every span in the
/// *Owns* cell would report `§13 Reflect::get_class` and `§13 Decimal::decimal`
/// as members nobody has written, and they are a PHP twin and a scalar's name.
/// Both directions are pinned, because either alone passes on a walk that is
/// wrong — the keys that must be there say the roster is read, and the keys that
/// must not say the sentence around it is not.
#[test]
fn the_compiler_facing_walk_reads_a_roster_and_not_the_prose_beside_it() {
    let found = compiler_facing_members();
    let keys: BTreeSet<&str> = found.iter().map(|member| member.key.as_str()).collect();

    for key in [
        // A bare name two commas from its row's only signature.
        "§13 Reflect::forClass",
        "§13 Reflect::typeOf",
        // A slash-separated roster that anchors on one generic in the middle.
        "§13 Test::assertSame",
        "§13 Test::expectFailure",
        // The cell with four parenthesised asides in it.
        "§13 Router::urlSigned",
        "§13 Serialize::encode",
        // The row that names no member at all.
        r"§13 Core\BigInt",
    ] {
        assert!(
            keys.contains(key),
            "the walk over docs/spec/01-core-library.md § 13 did not enumerate `{key}`, so the \
             gate below cannot ask the registry about it"
        );
    }

    for key in [
        // A *Replaces* clause's PHP twin, a full stop away from the roster.
        "§13 Reflect::get_class",
        // The last span of a sentence about compile errors.
        "§13 Command::string",
        // A scalar's name, in a cell that names no member.
        "§13 Decimal::decimal",
        // A statement keyword, in the sentence after the em dash.
        "§13 Serialize::spawn",
    ] {
        assert!(
            !keys.contains(key),
            "the walk over docs/spec/01-core-library.md § 13 read `{key}` out of the prose beside \
             a roster — a ratchet key no session could ever strike"
        );
    }

    assert!(
        found.len() > 25,
        "{} yielded only {} § 13 key(s), which is too few to be the whole table — the roster \
         reading has stopped matching the spec's own shape",
        "docs/spec/01-core-library.md",
        found.len()
    );
}

/// [`every_part_one_spec_member_is_registered`]'s third half: every member
/// [docs/spec/01-core-library.md](/docs/spec/01-core-library.md) § 13 names is a
/// member some class in [`registry::CLASSES`] declares.
///
/// [`compiler_facing_members`] owns the walk and what a roster is.  This owns
/// the ratchet: `tests/spec-members-compiler-facing-outstanding.txt`, whose keys
/// carry the row's class — `§13 Router::match` — because § 13 is one row per
/// class and a bare member name would say nothing about which.
#[test]
fn every_compiler_facing_spec_member_is_registered() {
    let outstanding: BTreeSet<String> = compiler_facing_members()
        .into_iter()
        .filter(|member| !member.declared)
        .map(|member| member.key)
        .collect();

    let Ratchet {
        path, keys: listed, ..
    } = outstanding_file("spec-members-compiler-facing-outstanding.txt");
    let unlisted: Vec<&String> = outstanding.difference(&listed).collect();
    assert!(
        unlisted.is_empty(),
        "{} spec member(s) in § 13 have no `registry::CLASSES` row and are not listed in {}: {}\n\
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
        "{} line(s) in {} name a member that is registered now, or a key no § 13 row produces: \
         {}\n\
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
/// `rule:php-migration/every-php-builtin-is-a-completion-candidate`
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
    let table = nvs_repo::path("docs/spec/02-php-migration.md");
    let text =
        fs::read_to_string(&table).unwrap_or_else(|err| panic!("{}: {err}", table.display()));
    // `tools/nv/cmd/migration.ts`'s own `ROW`, transcribed: a backticked
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
