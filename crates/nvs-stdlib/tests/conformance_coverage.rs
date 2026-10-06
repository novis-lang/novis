//! A coverage gate: every `Core`
//! member this crate registers is called by at least one `.nvst` case under
//! `tests/conformance/`.
//!
//! A registry row and an implementation together still prove nothing about
//! *behaviour* — `crates/nvs-stdlib/src/lib.rs` says a half-added member
//! cannot link, and this says a member cannot be added without a case that
//! runs it. It is the one check that reads across the crate boundary, which is
//! why it is an integration test rather than a `#[cfg(test)]` module: it needs
//! the repository, not the crate.
//!
//! **The enumerable set is what is registered, not what the spec owes.**
//! [docs/spec/01-core-library.md](/docs/spec/01-core-library.md) §§ 1-12
//! is the work list and `nvs-stdlib`'s known gap 1 tracks how much of it is on
//! disk; a member the registry does not hold is not yet a member, and the
//! compiler refuses to resolve a call to one, so there is nothing here to
//! check. What this forbids is the other order: registering a member, then
//! never writing the case.
//!
//! **The second gate is depth, and it is the same corpus read a second way.**
//! One case per member says a member runs; it says nothing about whether it
//! was asked anything it could get wrong. Stage 5's item 10 puts a floor of
//! three cases under every member, and
//! [`every_core_class_has_a_conformance_floor_of_three`] is that floor —
//! landed as a ratchet over the twenty-six members that were already below it,
//! for the reason [`BELOW_THE_FLOOR`] states.
//!
//! **The third gate is the paths no case takes.** A member's guards are
//! reached by neither the happy path nor the boundaries above it, and Stage
//! 5's item 11 is
//! [`every_error_path_is_asserted_or_declared_unreachable`]: every `Fault::`
//! message either appears in a case's frozen output, or carries a sentence at
//! its own site saying which diagnostic refuses the call before the runtime
//! can ever answer it. [`OWED_A_CASE`] is the ratchet that landed over the
//! fifty-seven that were neither.
//!
//! **How the corpus is read is not in this file.** `tests/corpus/mod.rs` holds
//! it, because `spec_registry_coverage.rs` asks the same corpus the same
//! question from the spec's end and Cargo compiles the two as separate crates.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use nvs_stdlib::registry;

mod corpus;

use corpus::{Attribution, cases, mentions, sources};

#[test]
fn every_part_one_member_has_a_conformance_case() {
    let source = sources().join("\n");

    let mut uncovered = BTreeSet::new();
    for class in registry::CLASSES {
        for method in class.methods {
            // The call spelling, with the open parenthesis, so `Str::trim`
            // does not answer for `Str::trimStart`. A member declaring a
            // written type parameter is spelled `decodeAs<User>(` at every
            // call site, so its own boundary is the `<`.
            let opener = if method.written().is_empty() {
                '('
            } else {
                '<'
            };
            let call = format!("{}::{}{opener}", class.name, method.name);
            if !source.contains(&call) {
                uncovered.insert(call);
            }
        }
        for method in class.instance {
            // An instance member is reached through a value, so what a case
            // writes is `->name(` and the class name appears nowhere. That is
            // weaker than the static spelling above — any receiver's `->text(`
            // answers for `Core\Regex\Match::text` — and deliberately so: the
            // alternative is inferring a receiver's type here, which would
            // mean a second checker rather than a coverage gate.
            //
            // The `<` boundary is the static loop's, for the same reason:
            // `$q->queryAs<Person>(` is the only spelling `Core\Db\Queryable`'s
            // generic member has, so `->queryAs(` appears in no case that
            // compiles.
            let opener = if method.written().is_empty() {
                '('
            } else {
                '<'
            };
            let call = format!("->{}{opener}", method.name);
            if !source.contains(&call) {
                uncovered.insert(format!("{}::{}", class.name, method.name));
            }
        }
        for declared in class.constants {
            // A constant has no parenthesis to bound it, so the boundary is
            // checked instead — otherwise `Core\Math::E` would be covered by
            // any case that writes `Core\Math::EPSILON`.
            let write = format!("{}::{}", class.name, declared.name);
            if !mentions(&source, &write) {
                uncovered.insert(write);
            }
        }
    }

    assert!(
        uncovered.is_empty(),
        "{} registered `Core` member(s) are never used by a conformance case: {}\n\
         Write one under tests/conformance/ — never with an `--ORACLE--` section, which \
         makes the Linux leg skip the case entirely.",
        uncovered.len(),
        uncovered
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    );
}

/// The members that were below the floor when the gate was written, and are
/// the whole of Stage 5 item 10's remaining worklist.
///
/// **It is empty**, so item 10's worklist is closed and what is left here is
/// the ratchet: every `Core` member is at or above the floor, and the test
/// below is now only the thing that keeps the next one from arriving thin.
///
/// **This list may only shrink.** It is not a permission to be thin: the test
/// below fails on a member that is *not* here and below the floor, and equally
/// on one that is here and has reached it, so the list cannot go stale in
/// either direction and nothing can be added to it without deleting this
/// sentence. A gate introduced over an existing violation set either arrives
/// as a ratchet or does not arrive, and what it buys on the day it lands is
/// that there is never a twenty-seventh.
///
/// `bun nv gaps --coverage` ranks these by class and names each one's
/// anchor; a case that asks one of them a *second question* — not the same
/// question again — is what removes a line.
const BELOW_THE_FLOOR: &[&str] = &[];

/// Stage 5's item 10: no `Core` member is asked by fewer than three cases.
///
/// The floor, not the median, and per class rather than per corpus — a median
/// rises by writing a fourth case for a member that already has three, which
/// is the metric a corpus grows *around* rather than into, and a corpus total
/// rises without any member moving at all. What this forbids is a member that
/// ships with one case pinning the happy path and nothing on either side of
/// it; the four shapes a third case takes are in
/// [docs/agent/conventions.md](/docs/agent/conventions.md).
///
/// Three is the smallest floor that cannot be met by the happy path alone: a
/// member with three cases has been asked something it refuses, or something
/// at a boundary, or the same question its siblings were asked. Constants are
/// not counted — a constant is one value, and the sibling check above already
/// requires a case to name it.
#[test]
fn every_core_class_has_a_conformance_floor_of_three() {
    const FLOOR: usize = 3;

    let texts = sources();
    let rules = Attribution::new();
    let index: Vec<BTreeMap<&'static str, BTreeSet<&str>>> =
        texts.iter().map(|text| rules.asked(text)).collect();

    let mut thin: Vec<String> = Vec::new();
    let mut closed: Vec<String> = Vec::new();
    for class in registry::CLASSES {
        let mut asked: BTreeMap<&'static str, usize> = class
            .methods
            .iter()
            .chain(class.instance)
            .map(|method| (method.name, 0))
            .collect();
        for case in &index {
            let Some(named) = case.get(class.name) else {
                continue;
            };
            for (member, count) in &mut asked {
                if named.contains(*member) {
                    *count += 1;
                }
            }
        }
        for (member, count) in &asked {
            let spelling = format!("{}::{member}", class.name);
            let exempt = BELOW_THE_FLOOR.contains(&spelling.as_str());
            if *count < FLOOR && !exempt {
                thin.push(format!("{spelling} is asked by {count} case(s)"));
            }
            if *count >= FLOOR && exempt {
                closed.push(spelling);
            }
        }
    }

    assert!(
        thin.is_empty(),
        "{} `Core` member(s) are asked by fewer than {FLOOR} conformance cases, so their \
         class's floor is below it:\n  {}\n\
         `bun nv gaps --coverage` ranks every class by the same figure and names \
         the thinnest members of each with their anchors. A case that asks one of these a \
         second question closes it; a case that asks the same question again does not. \
         `BELOW_THE_FLOOR` in this file is not where a new one goes — it only shrinks.",
        thin.len(),
        thin.join("\n  ")
    );
    assert!(
        closed.is_empty(),
        "{} member(s) listed in `BELOW_THE_FLOOR` have reached the floor of {FLOOR}. \
         Delete these lines — the list is the worklist, and a stale entry is a member \
         nobody will look at again:\n  {}",
        closed.len(),
        closed.join("\n  ")
    );
}

// ------------------------------------------------------------------ the error paths

/// How far past a `Fault::` site its message is looked for, in bytes.
///
/// Wide enough to span the two arguments a `Fault::thrown` writes before its
/// text and a `format!` wrapped over four lines by rustfmt, which is what the
/// widest sites in `bytes.rs` hold.
const MESSAGE_WINDOW: usize = 700;

/// How many lines above a `Fault::` site its declaration of unreachability may
/// sit, and the phrase that declares it.
///
/// The search stops at the first `Fault::` it meets on the way up, so a
/// declaration is never read as covering the site below the one it was written
/// for. A phrase rather than an attribute because what is being declared is a
/// judgement about *source programs*, which no attribute can carry: the
/// sentence after the colon is the whole content, and a reader who disagrees
/// with it deletes the comment and writes the case.
const DECLARATION_WINDOW: usize = 8;
const DECLARATION: &str = "unreachable from source";

/// The second declaration, for a path that *is* reachable and that no case in
/// either suite can reach: the phrase, plus the name of what asserts it instead.
///
/// [`DECLARATION`] says a source program cannot get here. This says one can,
/// but not one either suite is able to run — a `.nvst` case is a program with
/// no request in front of it, so every error path behind
/// `nvs_runtime::Inbound`'s body is reachable from Novis and unreachable from
/// the corpus. Collapsing the two would be a lie in whichever direction it was
/// written: declaring such a site "unreachable from source" claims the checker
/// refuses the call, and leaving it in [`OWED_A_CASE`] claims a case is owed
/// that nobody can write.
///
/// The comment naming a `#[test]` is the content, and it is a judgement no
/// machine makes: the phrase is what this gate reads, and the name after it is
/// what a reader checks. A site that could have had a case and took this
/// spelling instead is a bug in the comment, findable by grepping the phrase.
const ASSERTED_OFF_THE_CORPUS: &str = "no case can reach this";

/// Every `.rs` file under this crate's `src/`, as `(name, contents)`.
fn stdlib_sources() -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).unwrap_or_else(|err| panic!("{}: {err}", dir.display())) {
        let path = entry.expect("a readable directory entry").path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            let name = path
                .file_name()
                .expect("a file with an extension has a name")
                .to_string_lossy()
                .into_owned();
            let text = fs::read_to_string(&path).unwrap_or_else(|err| panic!("{name}: {err}"));
            out.push((name, text));
        }
    }
    out.sort();
    out
}

/// The one tree whose cases assert an error path: [`ERROR_CORPUS_ROOT`] is
/// `tests/conformance/`, and nothing outside it counts.
const ERROR_CORPUS_ROOT: &str = "tests/conformance";

/// Every case [`error_corpus`] reads, which is every `.nvst` file under
/// [`ERROR_CORPUS_ROOT`].
fn error_corpus_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    cases(&nvs_repo::path(ERROR_CORPUS_ROOT), &mut paths);
    assert!(
        paths.len() > 100,
        "`{ERROR_CORPUS_ROOT}` holds {} cases, which is too few to be the corpus — a check \
         over it would pass vacuously",
        paths.len()
    );
    paths
}

/// The whole text of every case under `tests/conformance/`, concatenated.
///
/// Whole text rather than the `--FILE--` section [`case_sources`] reads: what
/// asserts an error path is the *output* a case froze — an `--EXPECT--` line
/// or an `--EXPECTF-ERROR--` block — and enumerating those sections is how one
/// of them gets missed.
///
/// This is the corpus `bun nv gaps --errors` reads, deliberately the
/// same one: that tool is the worklist [`OWED_A_CASE`] freezes, and a gate
/// computing a different set could not be cross-checked against it.
/// [`the_error_corpus_is_the_conformance_tree_alone`] holds both to it.
fn error_corpus() -> String {
    error_corpus_paths()
        .iter()
        .map(|path| {
            fs::read_to_string(path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// One `Fault::` site: where it is, the run of its message before the first
/// format hole, and whether a [`DECLARATION`] or an
/// [`ASSERTED_OFF_THE_CORPUS`] sits above it.
struct Site {
    file: String,
    line: usize,
    stem: String,
    declared: bool,
}

/// `text` truncated to at most `bytes`, at a character boundary, and to the
/// first doc comment before that.
///
/// A `///` at the start of a line cannot appear inside an argument list — a
/// doc comment attaches to an item — so it is where the call being read ends
/// however the search is going. Without that stop, a `Fault::` whose message
/// was built above it reads the *next item's* prose as its message, which is
/// how `test.rs`'s `Fault::thrown_as` acquires the stem "a test that asserts
/// nothing" in `bun nv gaps --errors`.
fn head(text: &str, bytes: usize) -> &str {
    let mut end = match text.char_indices().find(|(at, _)| *at >= bytes) {
        Some((at, _)) => at,
        None => text.len(),
    };
    for (at, _) in text[..end].match_indices("///") {
        let before = &text[..at];
        if before.ends_with('\n') || before.trim_end_matches([' ', '\t']).ends_with('\n') {
            end = at;
            break;
        }
    }
    &text[..end]
}

/// The first string literal in `window` whose content is 10 to 400 characters,
/// still carrying its escapes.
///
/// The bound is the same one `tools/nv/cmd/gaps.ts` writes: under ten characters is a
/// separator or a member name passed alongside the message rather than the
/// message, and past four hundred no case froze it whole.
fn first_literal(window: &str) -> Option<String> {
    let mut chars = window.char_indices();
    while let Some((_, opening)) = chars.next() {
        if opening != '"' {
            continue;
        }
        let mut content = String::new();
        let mut escaped = false;
        let mut closed = false;
        for (_, c) in chars.by_ref() {
            if escaped {
                content.push('\\');
                content.push(c);
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                closed = true;
                break;
            } else {
                content.push(c);
            }
        }
        if !closed {
            return None;
        }
        if (10..=400).contains(&content.chars().count()) {
            return Some(content);
        }
    }
    None
}

/// A Rust string literal's content as the bytes it spells, for the two escapes
/// a diagnostic message uses.
///
/// A trailing backslash continues the literal onto the next line and eats the
/// indent that follows, so a message rustfmt wrapped would otherwise be
/// truncated at the wrap. **A carriage return counts as that newline**, because
/// a checkout with CRLF endings puts one between the backslash and the line
/// feed — reading only `\n` there left every wrapped message on Windows with a
/// stem no case could ever contain, which reads as an unasserted error path and
/// is not one. Every other escape keeps both of its characters:
/// `\n` before a format hole would be part of the stem either way, and
/// rewriting it to a newline would only make the stem harder to grep for.
fn unescape(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\n' | '\r') => while chars.next_if(|next| next.is_whitespace()).is_some() {},
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// Every `Fault::fatal` / `Fault::thrown` / `Fault::thrown_as` site in this
/// crate's `src/` whose message has a stem long enough to find in a case.
fn fault_sites() -> Vec<Site> {
    let mut out = Vec::new();
    for (file, text) in stdlib_sources() {
        let lines: Vec<&str> = text.lines().collect();
        for (at, _) in text.match_indices("Fault::") {
            let rest = &text[at + "Fault::".len()..];
            let kind: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !matches!(kind.as_str(), "fatal" | "thrown" | "thrown_as") {
                continue;
            }
            let after = rest[kind.len()..].trim_start();
            let Some(arguments) = after.strip_prefix('(') else {
                continue;
            };
            let Some(raw) = first_literal(head(arguments, MESSAGE_WINDOW)) else {
                continue;
            };
            let message = unescape(&raw);
            let stem = message
                .split(['{', '}'])
                .next()
                .unwrap_or_default()
                .trim()
                .to_owned();
            // Too short to tell one site from another, and too short to match
            // a case's output without matching half the corpus with it.
            if stem.chars().count() < 14 {
                continue;
            }
            let line = text[..at].matches('\n').count() + 1;
            let declared = lines[line.saturating_sub(1 + DECLARATION_WINDOW)..line - 1]
                .iter()
                .rev()
                .take_while(|above| !above.contains("Fault::"))
                .any(|above| {
                    let above = above.to_ascii_lowercase();
                    above.contains(DECLARATION) || above.contains(ASSERTED_OFF_THE_CORPUS)
                });
            out.push(Site {
                file: file.clone(),
                line,
                stem,
                declared,
            });
        }
    }
    out
}

/// The error paths that neither suite reaches and no site declares
/// unreachable, frozen at the size the gate below landed at.
///
/// **It is empty, and that is the whole of item 11**: every `Fault::` site in
/// this crate with a findable stem is now either caught by a case or declared
/// at the site. The list stays here because an empty allowance is what makes
/// the gate below a ratchet — a site added tomorrow with neither answer fails
/// the test rather than quietly extending a roster, and the two paragraphs
/// after this one are the instructions for the session that meets one.
///
/// **This list may only shrink**, and it shrinks three ways, because item 11
/// has three answers and the site is what decides between them. A boundary a
/// program can reach loses its line here by gaining a case that catches the
/// message and echoes it. An invariant no source program can reach — an
/// argument type-guard behind a parameter the checker already types, most of
/// these — loses its line by gaining a [`DECLARATION`] comment saying which
/// diagnostic refuses the call first. A boundary a program can reach and a
/// *case* cannot — everything behind a served request's body, since a `.nvst`
/// case runs with no request in front of it — loses its line by gaining an
/// [`ASSERTED_OFF_THE_CORPUS`] comment naming the `#[test]` that asserts it.
///
/// The test below fails on a site that is *not* here and is neither asserted
/// nor declared, and equally on a line here whose site has since become one or
/// the other, so an entry cannot go stale in either direction and nothing can
/// be added without deleting this sentence.
///
/// `bun nv gaps --errors` is the same set with each site's anchor and
/// whole message, which is what a session works from; the key here is the file
/// and the stem because a line number moves under an unrelated edit and a stem
/// does not. One line covers every site in its file writing that stem.
const OWED_A_CASE: &[(&str, &str)] = &[];

/// Stage 5's item 11: every error path a `Core` member can take is asserted by
/// a case, or is declared at the site — as one no source program reaches, or as
/// one no *case* can run.
///
/// The two are not the same claim and only the site can tell them apart. A
/// `Fault::thrown` is a boundary — a column of the wrong type, an origin that
/// was never configured — and a case catches it and echoes the message. Most
/// of the `Fault::fatal` sites are the other kind: `Core\Arr::count expected
/// array, got tag 3` is what the *runtime* would say to an argument
/// `nvs_types` refuses at `E0401` before any of it runs, so there is no
/// program to write and the honest answer is a sentence at the site saying
/// which diagnostic gets there first. Both are unreachable in the same sense a
/// `debug_assert!` is, and neither is dead code: the guard is what makes the
/// `unsafe` below it sound.
///
/// [`ASSERTED_OFF_THE_CORPUS`] is the third, and its own doc says why it could
/// not be folded into either.
///
/// What this forbids is the state where all of them start — a message nobody
/// has judged, which reads exactly like a live path and exactly like an
/// impossible one.
///
/// **The limit, stated rather than implied:** of the crate's 269 `Fault::`
/// sites this reads 165, the ones whose message begins with enough literal
/// text to find in a case. A message opening on its own format hole —
/// `` `{code}` is not a pack directive `` — has a stem that would match half
/// the corpus or none of it, so neither this nor `bun nv gaps` can say whether a
/// case reaches it, and requiring a declaration for one would be requiring a
/// judgement no case could ever discharge. Giving such a site a few words of
/// its own before the first hole brings it under the gate, which is a better
/// message anyway.
#[test]
fn every_error_path_is_asserted_or_declared_unreachable() {
    let corpus = error_corpus();
    let sites = fault_sites();
    assert!(
        sites.len() > 150,
        "found {} `Fault::` site(s) with a stem in this crate's src/, which is too few to \
         be all of them — the scan has stopped matching and the gate is passing vacuously",
        sites.len()
    );

    let owing: Vec<Site> = sites
        .into_iter()
        .filter(|site| !site.declared && !corpus.contains(&site.stem))
        .collect();

    let listed = |site: &Site| {
        OWED_A_CASE
            .iter()
            .any(|(file, stem)| *file == site.file && *stem == site.stem)
    };
    // Grouped by the pair the list is keyed on, so a stem two sites share
    // freezes as one line carrying both anchors rather than as two identical
    // ones — `Core\Bytes::pack` guards its format and its value list with the
    // same sentence, and both are the same judgement.
    let mut unlisted: Vec<(&Site, Vec<usize>)> = Vec::new();
    for site in owing.iter().filter(|site| !listed(site)) {
        match unlisted
            .iter_mut()
            .find(|(first, _)| first.file == site.file && first.stem == site.stem)
        {
            Some((_, lines)) => lines.push(site.line),
            None => unlisted.push((site, vec![site.line])),
        }
    }
    let missing = unlisted
        .iter()
        .map(|(site, lines)| {
            let anchors = lines
                .iter()
                .map(|line| format!("{}:{line}", site.file))
                .collect::<Vec<_>>()
                .join(", ");
            format!("    ({:?}, {:?}), // {anchors}", site.file, site.stem)
        })
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "{} error path(s) are neither asserted by a case nor declared at the site. Write \
         the case, or write a comment within the {DECLARATION_WINDOW} lines above the \
         site: \"{DECLARATION}\" plus the diagnostic that refuses the call first, or — \
         where a program reaches the path and no case can, as nothing behind a served \
         request's body can be reached by one — \"{ASSERTED_OFF_THE_CORPUS}\" plus the \
         `#[test]` that asserts it instead. A declaration further up than that window is \
         one this gate cannot see. If this gate is being re-frozen, these are the \
         lines:\n{}",
        missing.len(),
        missing.join("\n")
    );

    for (file, stem) in OWED_A_CASE {
        assert!(
            owing
                .iter()
                .any(|site| site.file == *file && site.stem == *stem),
            "{file}'s `{stem}` is asserted or declared and is still named in `OWED_A_CASE`; \
             delete that line, because the list only shrinks"
        );
    }
}

/// The error corpus is `tests/conformance/` and nothing else, here and in
/// `bun nv gaps --errors`.
///
/// Every case [`error_corpus`] reads lies under [`ERROR_CORPUS_ROOT`], and
/// `tools/nv/cmd/gaps.ts`'s `errorGaps` builds its corpus from that tree
/// alone. The second half reads the tool's source because the two must
/// compute the same set: a gate and a worklist over different corpora could
/// not be cross-checked against each other.
#[test]
fn the_error_corpus_is_the_conformance_tree_alone() {
    let root = nvs_repo::path(ERROR_CORPUS_ROOT);
    let outside: Vec<String> = error_corpus_paths()
        .iter()
        .filter(|path| !path.starts_with(&root))
        .map(|path| path.display().to_string())
        .collect();
    assert!(
        outside.is_empty(),
        "the error corpus reads {} case(s) outside `{ERROR_CORPUS_ROOT}`:\n{}",
        outside.len(),
        outside.join("\n")
    );

    let gaps_path = nvs_repo::path("tools/nv/cmd/gaps.ts");
    let gaps = fs::read_to_string(&gaps_path)
        .unwrap_or_else(|err| panic!("{}: {err}", gaps_path.display()));
    let body = gaps
        .split_once("function errorGaps(")
        .and_then(|(_, rest)| rest.split_once("\n}\n"))
        .map(|(body, _)| body)
        .unwrap_or_else(|| panic!("{} defines no `errorGaps` function", gaps_path.display()));
    let reads: Vec<&str> = body
        .match_indices("corpus(")
        .map(|(at, _)| {
            let rest = &body[at + "corpus(".len()..];
            &rest[..rest.find(')').unwrap_or(rest.len())]
        })
        .collect();
    assert_eq!(
        reads,
        ["CONFORMANCE"],
        "`errorGaps` in {} must build its corpus from `corpus(CONFORMANCE)` once and from \
         nothing else, so `bun nv gaps --errors` reads the corpus this gate reads",
        gaps_path.display()
    );
    assert!(
        gaps.contains(&format!("const CONFORMANCE = \"{ERROR_CORPUS_ROOT}\";")),
        "{} no longer defines `CONFORMANCE` as `{ERROR_CORPUS_ROOT}`",
        gaps_path.display()
    );
}
