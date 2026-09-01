//! The gate `docs/agent/loop-goal.md` names as this loop's own definition of
//! done: every member
//! [docs/spec/01-core-library.md](../../../docs/spec/01-core-library.md) §§ 1-12
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

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use nvs_stdlib::registry;

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
/// than copied, which is the whole point of the test below: ADR 0063 R2 makes
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

/// The keys the named ratchet file under `tests/` lists, blank lines and `#`
/// comments dropped.
///
/// There are two, one per walk — §§ 1-12's and §§ 14-19's — because the two
/// halves are finished by different loops and a single file would make a Part I
/// regression indistinguishable from a Part II member nobody has reached yet.
fn outstanding_file(name: &str) -> (std::path::PathBuf, BTreeSet<String>) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(name);
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

    let (path, listed) = outstanding_file("spec-members-outstanding.txt");
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

/// ADR 0063 R2's names, held against the one place they are written.
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
         The spec's signature column is the source (ADR 0063 R2) — change the row's `names`, \
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

/// Every member [docs/spec/01-core-library.md](../../../docs/spec/01-core-library.md)
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
            // `Development` — and ADR 0063's naming is what tells the two apart
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
/// [docs/spec/01-core-library.md](../../../docs/spec/01-core-library.md)
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

    let (path, listed) = outstanding_file("spec-members-part-two-outstanding.txt");
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

/// Every class [docs/spec/01-core-library.md](../../../docs/spec/01-core-library.md)
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

    let (path, listed) = outstanding_file("spec-classes-part-two-outstanding.txt");
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
            .1
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
