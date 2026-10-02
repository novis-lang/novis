//! The PHP-name table, joined at build time out of the two documents that own
//! it: the oracle inventory and the migration table.
//!
//! `rule:php-migration/every-php-builtin-is-a-completion-candidate` splits this
//! layer across two files and forbids copying either into the other. The
//! **inventory** — `tools/data/php-builtins.txt`, generated from the same PHP
//! build the differential suite uses as its oracle — is the complete list of
//! candidates, so the list is complete on the first day whatever the migration
//! table's coverage. The **migration table** — `docs/spec/02-php-migration.md`
//! — is what an item for one of those names *says*, one row per name, under the
//! four outcomes its own *How to read a row* section writes. This script is the
//! join, and `src/php_names.rs` is the module the emitted table lands in.
//!
//! # The registry is the third input, and it is not joined here
//!
//! A build script cannot link the crate it builds, so `registry::class` — the
//! lookup that decides whether a destination is insertable — is unreachable
//! from this file. Nothing is copied here to work around that: the two
//! documents are all this script reads, and the registry half of the join is
//! `php_names::Destination::is_registered`, asked of the emitted rows. What
//! holds `rule:php-migration/an-item-inserts-only-a-registered-member` is
//! `tests/php_names.rs`, which fails `cargo test -p nvs-stdlib` on a member
//! spelling that a registered class does not declare — a typo in the migration
//! table's Novis cell — exactly as `tests/spec_registry_coverage.rs` already
//! does for the rows themselves.
//!
//! # The second table: the reference intros
//!
//! `docs/reference/core/<Class>.md` is the hand-written intro page of one
//! `Core` class, and [`intros`] joins every one of them into a `(class, body)`
//! table with the YAML front matter cut off, which `src/registry.rs` includes
//! as `CoreClass::intro`'s source. Read here rather than `include_str!`'d by
//! hand, so a page is compiled in the day it is written and no class carries a
//! hand-typed path to its own page. `bun nv reference` and the website keep
//! reading the `.md` files; nothing moves.
//!
//! # What this script may fail the build on
//!
//! Only a document whose *shape* it cannot read: an inventory with no names, a
//! table under its row floor, an outcome word that is not one of the four, or
//! two rows for one PHP name. Every one of those means the parse below has
//! stopped matching the document rather than found it clean, and a table
//! generated from a collapsed parse would pass every assertion downstream
//! vacuously. A `member` row naming a destination nobody has built is *not* a
//! failure — it is the second of the four item shapes, and the item that
//! appears and inserts nothing is what that row is for.

#![allow(
    clippy::print_stdout,
    reason = "`cargo:` directives on stdout are how a build script talks to Cargo"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The four outcomes `docs/spec/02-php-migration.md` § *How to read a row*
/// writes, paired with the `Outcome` variant each renders as.
const OUTCOMES: [(&str, &str); 4] = [
    ("member", "Outcome::Member"),
    ("language", "Outcome::Language"),
    ("dropped", "Outcome::Dropped"),
    ("open", "Outcome::Open"),
];

/// The count below which the inventory has stopped being read rather than
/// found empty.
///
/// It lists a four-figure number of internal functions and a three-figure
/// number of internal types; a parse that returns a handful has lost the
/// section headers. The floor is deliberately far below the real count, which
/// is a fact `tools/data/php-builtins.txt`'s own header owns and this file does
/// not copy.
const INVENTORY_FLOOR: usize = 900;

/// The same guard for the migration table, on the reasoning
/// `tests/spec_registry_coverage.rs`'s `MIGRATION_ROW_FLOOR` states: a table
/// whose shape drifts under the row parser reads as zero rows, and every
/// candidate then comes out undecided with nothing to say so.
const ROW_FLOOR: usize = 800;

#[path = "../../tools/build/tests_without_pdb.rs"]
mod tests_without_pdb;

fn main() {
    tests_without_pdb::main();
    let manifest = PathBuf::from(env("CARGO_MANIFEST_DIR"));
    // crates/nvs-stdlib -> the workspace root.
    let workspace = manifest
        .parent()
        .and_then(Path::parent)
        .map_or_else(|| manifest.clone(), Path::to_path_buf);
    let inventory_path = workspace
        .join("tools")
        .join("data")
        .join("php-builtins.txt");
    let migration_path = workspace
        .join("docs")
        .join("spec")
        .join("02-php-migration.md");

    // Both documents live outside the crate directory, which Cargo does not
    // track on its own.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", inventory_path.display());
    println!("cargo:rerun-if-changed={}", migration_path.display());

    let candidates = inventory(&read(&inventory_path));
    let rows = migration(&read(&migration_path));

    let out = PathBuf::from(env("OUT_DIR")).join("php-names.rs");
    let generated = render(&candidates, &rows);
    std::fs::write(&out, generated).unwrap_or_else(|err| panic!("{}: {err}", out.display()));

    let reference = workspace.join("docs").join("reference").join("core");
    // A directory, which Cargo walks: a page added tomorrow is a rebuild.
    println!("cargo:rerun-if-changed={}", reference.display());
    let out = PathBuf::from(env("OUT_DIR")).join("intros.rs");
    std::fs::write(&out, intros(&reference))
        .unwrap_or_else(|err| panic!("{}: {err}", out.display()));
}

/// Every page under `dir` as one `(class, body)` row, sorted by class.
///
/// The file name is the class's path with `-` for `\` — `Str.md` is
/// `Core\Str`, `Time-Date.md` is `Core\Time\Date` — which is the spelling
/// `bun nv reference` reads the same tree by. A page naming a class the
/// registry does not hold is not a failure here, since a build script cannot
/// ask the registry; `every_intro_page_names_a_registry_class` in
/// `src/registry.rs` is what fails on one.
fn intros(dir: &Path) -> String {
    let mut pages: Vec<(String, String)> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|extension| extension != "md") {
                continue;
            }
            println!("cargo:rerun-if-changed={}", path.display());
            let stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default();
            let class = format!("Core\\{}", stem.replace('-', "\\"));
            pages.push((class, body(&read(&path))));
        }
    }
    pages.sort();
    let mut out = String::from(
        "/// One `(class, intro)` per page of `docs/reference/core/`, the front matter cut off,\n\
         /// sorted by class. Generated by `build.rs`.\n\
         pub(crate) const INTROS: &[(&str, &str)] = &[\n",
    );
    for (class, text) in pages {
        let _ = writeln!(out, "    (r\"{class}\", {}),", raw(&text));
    }
    out.push_str("];\n");
    out
}

/// The page below its front matter: what follows the closing `---` line, or
/// the whole page for one that opens with none. Line endings are normalized,
/// so a checkout with CRLF renders the same text as one without.
fn body(text: &str) -> String {
    let text = text.replace("\r\n", "\n");
    let Some(rest) = text.strip_prefix("---\n") else {
        return text.trim().to_owned();
    };
    rest.find("\n---\n")
        .map_or(text.trim(), |at| rest[at + "\n---\n".len()..].trim())
        .to_owned()
}

/// `text` as a Rust raw string literal, fenced with one more `#` than any run
/// the text itself contains.
fn raw(text: &str) -> String {
    let mut longest = 0;
    let mut run = 0;
    for c in text.chars() {
        run = if c == '#' { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    let fence = "#".repeat(longest + 1);
    format!("r{fence}\"{text}\"{fence}")
}

/// An environment variable Cargo always sets for a build script.
fn env(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("cargo sets {key} for a build script"))
}

/// One file, read, or a build failure naming it.
fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

/// Every name the oracle inventory lists, paired with the `Kind` variant its
/// section renders as.
///
/// The file is `# comment` lines, a `[functions]` and a `[types]` header, and
/// one bare name per line under each. A name outside any section is a file
/// whose shape has changed, and is dropped rather than filed under a guess —
/// the floor in `main` is what notices if that ever becomes most of it.
fn inventory(text: &str) -> Vec<(String, &'static str)> {
    let mut section = None;
    let mut names = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(header) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = match header {
                "functions" => Some("Kind::Function"),
                "types" => Some("Kind::Type"),
                _ => None,
            };
            continue;
        }
        if let Some(kind) = section {
            names.push((line.to_owned(), kind));
        }
    }
    assert!(
        names.len() >= INVENTORY_FLOOR,
        "tools/data/php-builtins.txt produced {} name(s), under the floor of {INVENTORY_FLOOR} — \
         the inventory's shape has changed under this parser",
        names.len()
    );
    names.sort();
    names.dedup();
    names
}

/// One migration row: the outcome word, the Novis cell verbatim, and every
/// `Core\X::member` spelling that cell names.
struct Row {
    outcome: &'static str,
    cell: String,
    destinations: Vec<(String, String)>,
}

/// The migration table, keyed by the PHP name each row is about.
fn migration(text: &str) -> BTreeMap<String, Row> {
    let mut rows = BTreeMap::new();
    for line in text.lines() {
        let Some((php, outcome, cell)) = row(line) else {
            continue;
        };
        let Some((_, variant)) = OUTCOMES.iter().find(|(word, _)| *word == outcome) else {
            panic!(
                "docs/spec/02-php-migration.md: `{php}` has the outcome `{outcome}`, which is not \
                 one of the four that file's *How to read a row* section writes"
            );
        };
        let previous = rows.insert(
            php.to_owned(),
            Row {
                outcome: variant,
                cell: cell.trim().to_owned(),
                destinations: destinations(cell),
            },
        );
        assert!(
            previous.is_none(),
            "docs/spec/02-php-migration.md: two rows for `{php}` — a name is accounted for once, \
             and a second row makes which answer a migrating program gets a question of file order"
        );
    }
    assert!(
        rows.len() >= ROW_FLOOR,
        "docs/spec/02-php-migration.md produced {} row(s), under the floor of {ROW_FLOOR} — the \
         table's shape has changed under this parser",
        rows.len()
    );
    rows
}

/// One table row's three cells, or `None` for a line that is not one.
///
/// `tests/spec_registry_coverage.rs`'s `migration_member_refs` reads the same
/// shape with a regular expression, and this is that expression by hand: a
/// backticked PHP name, a bare lower-case outcome word, and everything up to
/// the line's closing pipe. Both deliberately skip the escape handling
/// `01-core-library.md`'s union types need — this table's Novis cell is the one
/// place in the specification where a backslash means itself.
fn row(line: &str) -> Option<(&str, &str, &str)> {
    let inner = line.trim_end().strip_prefix('|')?.strip_suffix('|')?;
    let (php, rest) = inner.split_once('|')?;
    let (outcome, cell) = rest.split_once('|')?;
    let php = php.trim().strip_prefix('`')?.strip_suffix('`')?;
    if php.is_empty() || php.contains('`') {
        return None;
    }
    let outcome = outcome.trim();
    if outcome.is_empty() || !outcome.bytes().all(|byte| byte.is_ascii_lowercase()) {
        return None;
    }
    Some((php, outcome, cell))
}

/// Every `Core\X::member` spelling in a Novis cell, in the order it writes
/// them.
///
/// Every spelling, not the cell: a cell naming two members is prose the
/// converter may not act on, and `rule:ide/three-of-four-item-shapes-insert-nothing`
/// makes it one completion item per member here, because completion has a
/// person in the loop to pick. A class half is one or more `\`-joined
/// segments, so `Core\Db\Queryable` is taken whole rather than as `Core\Db`
/// with a stray tail.
fn destinations(cell: &str) -> Vec<(String, String)> {
    let bytes = cell.as_bytes();
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(offset) = cell[at..].find("Core\\") {
        let start = at + offset;
        // Every position a complete `Core\Seg\Seg` run could end at, longest
        // last, so the `::` test below takes the longest one that has it.
        let mut ends = Vec::new();
        let mut cursor = start + "Core".len();
        while bytes.get(cursor) == Some(&b'\\') {
            let segment = cursor + 1;
            if !bytes.get(segment).is_some_and(u8::is_ascii_alphabetic) {
                break;
            }
            let mut end = segment + 1;
            while bytes.get(end).is_some_and(u8::is_ascii_alphanumeric) {
                end += 1;
            }
            ends.push(end);
            cursor = end;
        }
        at = start + "Core\\".len();
        for end in ends.into_iter().rev() {
            let Some(rest) = cell.get(end..).and_then(|rest| rest.strip_prefix("::")) else {
                continue;
            };
            if !rest.as_bytes().first().is_some_and(u8::is_ascii_alphabetic) {
                continue;
            }
            let member: String = rest
                .chars()
                .take_while(char::is_ascii_alphanumeric)
                .collect();
            found.push((cell[start..end].to_owned(), member.clone()));
            at = end + "::".len() + member.len();
            break;
        }
    }
    found
}

/// The table, as the Rust source `src/php_names.rs` includes.
fn render(candidates: &[(String, &'static str)], rows: &BTreeMap<String, Row>) -> String {
    let mut out = String::new();
    out.push_str("// @generated by crates/nvs-stdlib/build.rs from the oracle inventory and\n");
    out.push_str("// docs/spec/02-php-migration.md. Edit those two documents, never this.\n\n");
    out.push_str("/// Every name the oracle inventory lists, sorted by its PHP spelling.\n");
    out.push_str("///\n");
    out.push_str(
        "/// A name with no row in the migration table carries [`Outcome::Open`] and an\n",
    );
    out.push_str("/// empty cell, which is the same case an `open` row is.\n");
    out.push_str("pub static CANDIDATES: &[Candidate] = &[\n");
    for (php, kind) in candidates {
        let row = rows.get(php);
        let outcome = row.map_or("Outcome::Open", |row| row.outcome);
        let cell = row.map_or("", |row| row.cell.as_str());
        let destinations = row.map(|row| row.destinations.as_slice()).unwrap_or(&[]);
        let _ = write!(
            out,
            "    Candidate {{ php: {}, kind: {kind}, outcome: {outcome}, cell: {}, \
             destinations: &[",
            quote(php),
            quote(cell)
        );
        for (class, member) in destinations {
            let _ = write!(
                out,
                "Destination {{ class: {}, member: {} }}, ",
                quote(class),
                quote(member)
            );
        }
        out.push_str("] },\n");
    }
    out.push_str("];\n");
    out
}

/// One Rust string literal holding `text`.
///
/// A Novis cell is markdown: em dashes, links and backslashes all reach this,
/// and the backslash is the one that matters, since every destination spelling
/// carries two of them.
fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\r' | '\n' | '\t' => out.push(' '),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}
