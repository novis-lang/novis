//! `nvs agent` — the surface a coding agent reads the language through.
//!
//! `rule:tooling/an-agent-asks-the-binary` is the surface,
//! `rule:tooling/the-index-is-one-line-per-member` is what `index` prints and
//! `rule:tooling/a-primer-claim-is-executed` is what `primer` carries. `index`,
//! `find` and `show` render [`crate::meta::document`] and decide nothing
//! (`rule:tooling/one-json-several-renderers`), which is what makes this a
//! consumer of the registry rather than a second statement of it: a member that
//! lands is in the index at the next call, and there is no list anywhere that
//! can disagree with the binary that compiles the program. `primer` renders that
//! same document beside the reference chapters this binary embeds, and writes no
//! sentence of its own about the language.
//!
//! Nothing is written to disk and nothing is cached. The document is rebuilt per
//! call from compile-time tables, so the answer cannot describe a version that is
//! not installed — which is the failure the whole surface exists to remove.
//!
//! ## The primer, and where its text comes from
//!
//! Every part of the primer is lifted from a chapter section a `<!-- primer -->`
//! comment marks, and a section is in it for that reason and no other, so a
//! section that stops being true stops being rendered rather than becoming a
//! lie. [`CHAPTERS`] is what this binary carries; the chapter map at the foot of
//! the primer is those chapters' own front matter, and the count above it is the
//! registry's. `docs/reference/README.md` § *Marking a section for the primer*
//! is the marker's home.
//!
//! ## A line, and the symbol inside it
//!
//! An index line opens with the symbol `show` resolves and continues with the
//! member's signature in the spec's own spelling:
//!
//! ```text
//! Core\IO::read(string $path): string  [fs.read]
//! Core\Json::decodeAs<T>(string $json): T
//! Core\Order  enum {Asc, Desc}
//! RuntimeError  exception extends Error
//! ```
//!
//! So the symbol is the line up to its first `(`, `<` or space, and a consumer
//! needs no parser to get from a line back to `show`. `<` is in that set because
//! a generic member's signature carries its type parameters in the spec's own
//! spelling while the name does not, and `show` also accepts the whole leading
//! token with the `<T>` still on it — copying a line's opening word is what an
//! agent will do, and refusing that spelling would cost a call to teach a
//! distinction the card itself makes. The capability in
//! brackets is joined from the document's `capabilities` roster at render time
//! and never read off the member row, because
//! `rule:security/capability-declaration-is-one-table` refuses a per-member
//! field; [`crate::meta`] emits the roster this joins against.
//!
//! `find` matches that symbol rather than the whole line, case-insensitively, so
//! a query naming a type — `string` — does not answer with every member that
//! returns one. It is a command rather than an instruction to grep, because a
//! namespaced name loses its backslash to the shell before `grep` sees it and the
//! empty result that follows is indistinguishable from a name the language does
//! not have.
//!
//! `find` succeeds having printed nothing when a query matches nothing: the index
//! is complete, so an empty result is the answer that the name does not exist.
//! `show` is the opposite — it was asked for one specific thing and exits
//! non-zero when it cannot produce it, naming what it has instead.

use std::process::ExitCode;

use serde_json::Value;

/// The line that marks the section under it, on a line of its own.
const MARKER: &str = "<!-- primer -->";

/// The reference chapters, in the order `docs/novis.md` concatenates them, which
/// is the order the chapter map prints them in.
///
/// The whole chapter is embedded rather than an extract of its marked sections,
/// because an extract is a second artefact that can disagree with the chapter
/// and the primer exists to be a document that cannot. It spends ~290 KB of the
/// binary's read-only data — once per binary, never per request — to buy that.
const CHAPTERS: &[&str] = &[
    include_str!("../../../docs/reference/lang/10-programs.md"),
    include_str!("../../../docs/reference/lang/20-types.md"),
    include_str!("../../../docs/reference/lang/30-expressions.md"),
    include_str!("../../../docs/reference/lang/40-statements.md"),
    include_str!("../../../docs/reference/lang/50-classes.md"),
    include_str!("../../../docs/reference/lang/55-enums.md"),
    include_str!("../../../docs/reference/lang/60-iteration.md"),
    include_str!("../../../docs/reference/lang/70-errors.md"),
    include_str!("../../../docs/reference/lang/80-concurrency.md"),
    include_str!("../../../docs/reference/lang/90-attributes.md"),
    include_str!("../../../docs/reference/lang/95-testing.md"),
    include_str!("../../../docs/reference/tools/10-cli.md"),
    include_str!("../../../docs/reference/tools/20-config.md"),
    include_str!("../../../docs/reference/tools/30-php-differences.md"),
    include_str!("../../../docs/reference/tools/40-editor.md"),
];

/// The chapters whose marked sections open the primer, in the order it prints
/// them: the lookup protocol, the worked program, the capability model, then the
/// refusals. Every other chapter's marked sections follow in reference order, so
/// this list fixes where a section lands and never whether it is lifted.
const PRIMER_FIRST: &[&str] = &["cli", "programs", "config", "php-differences"];

/// One chapter: the three front-matter fields the chapter map reads, and the
/// text the marked sections are cut from.
struct Chapter {
    id: &'static str,
    title: &'static str,
    summary: &'static str,
    text: &'static str,
}

/// One line of the index: the symbol `show` resolves, and the line `index`
/// prints for it.
struct Entry {
    symbol: String,
    line: String,
}

/// The document's array under `key`, or nothing where the omission rule dropped
/// it (`rule:tooling/meta-json`).
fn array<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value[key].as_array().map_or(&[], Vec::as_slice)
}

/// The document's string under `key`, or the empty string where it is absent.
fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or_default()
}

/// `Class::member` to the capability that call is gated on, read off the
/// document's own `capabilities` roster.
///
/// A row naming no capability is not in this map: an ungated member renders its
/// line with no bracket, which is the same "nothing written" the absent key
/// means.
fn gates(document: &Value) -> Vec<(String, &str)> {
    array(document, "capabilities")
        .iter()
        .filter_map(|row| {
            let capability = row["capability"].as_str()?;
            Some((
                format!("{}::{}", text(row, "class"), text(row, "member")),
                capability,
            ))
        })
        .collect()
}

/// Every line of the index, in the registry's own order: the members class by
/// class, then the enums, exceptions and attributes beside them.
fn entries(document: &Value) -> Vec<Entry> {
    let gates = gates(document);
    let mut out = Vec::new();

    for class in array(document, "classes") {
        let class_name = text(class, "name");
        for member in array(class, "members") {
            let symbol = format!("{class_name}::{}", text(member, "name"));
            let mut line = format!("{class_name}::{}", text(member, "signature"));
            if let Some((_, capability)) = gates.iter().find(|(gated, _)| *gated == symbol) {
                line.push_str("  [");
                line.push_str(capability);
                line.push(']');
            }
            out.push(Entry { symbol, line });
        }
    }

    for core_enum in array(document, "enums") {
        let name = text(core_enum, "name");
        let cases: Vec<&str> = array(&core_enum["doc"], "cases")
            .iter()
            .map(|case| text(case, "name"))
            .collect();
        let mut line = format!("{name}  enum");
        if !cases.is_empty() {
            line.push_str(&format!(" {{{}}}", cases.join(", ")));
        }
        out.push(Entry {
            symbol: name.to_owned(),
            line,
        });
    }

    for exception in array(document, "exceptions") {
        let name = text(exception, "name");
        let mut line = format!("{name}  exception");
        if let Some(parent) = exception["parent"].as_str() {
            line.push_str(" extends ");
            line.push_str(parent);
        }
        out.push(Entry {
            symbol: name.to_owned(),
            line,
        });
    }

    for attribute in array(document, "attributes") {
        let Some(name) = attribute.as_str() else {
            continue;
        };
        out.push(Entry {
            symbol: name.to_owned(),
            line: format!("{name}  attribute"),
        });
    }

    out
}

/// The document that makes an agent productive: the marked chapter sections in
/// the order `rule:tooling/a-primer-claim-is-executed` fixes, then the chapter
/// map from the chapters' own front matter.
pub(crate) fn primer() -> ExitCode {
    let document = crate::meta::document();
    let chapters: Vec<Chapter> = CHAPTERS.iter().map(|text| chapter(text)).collect();
    let classes = array(&document, "classes");
    let members: usize = classes
        .iter()
        .map(|class| array(class, "members").len())
        .sum();

    let mut out = format!(
        "# Novis, for a coding agent\n\n\
         Generated by `nvs agent primer` from the reference chapters and the registry this binary \
         carries: {} `Core` classes and {members} members, one line each in `nvs agent index`.\n",
        classes.len(),
    );

    for chapter in primer_order(&chapters) {
        for section in marked_sections(chapter.text) {
            push_section(&mut out, &section);
        }
    }

    out.push_str("\n## The chapters\n\nThe reference is one chapter per topic, and these are all of them.\n\n");
    for chapter in &chapters {
        out.push_str(&format!(
            "- **{}** — {}: {}\n",
            chapter.id, chapter.title, chapter.summary
        ));
    }

    print!("{out}");
    ExitCode::SUCCESS
}

/// A chapter's leading `---` block — `key: value` lines, three of which the
/// primer reads — and the text under it.
fn chapter(text: &'static str) -> Chapter {
    let mut chapter = Chapter {
        id: "",
        title: "",
        summary: "",
        text,
    };
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("---") {
        return chapter;
    }
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        // A title holding a `:` is quoted to survive that split; the quotes are
        // the front matter's and not the title's.
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .unwrap_or(value);
        match key.trim() {
            "id" => chapter.id = value,
            "title" => chapter.title = value,
            "summary" => chapter.summary = value,
            _ => {}
        }
    }
    chapter
}

/// The chapters in the order the primer prints their marked sections.
fn primer_order(chapters: &[Chapter]) -> Vec<&Chapter> {
    let mut out: Vec<&Chapter> = PRIMER_FIRST
        .iter()
        .filter_map(|id| chapters.iter().find(|chapter| chapter.id == *id))
        .collect();
    out.extend(
        chapters
            .iter()
            .filter(|chapter| !PRIMER_FIRST.contains(&chapter.id)),
    );
    out
}

/// Each section a [`MARKER`] line marks, lifted whole: the heading on the line
/// directly under the marker, and every line down to the next heading of the
/// same or a shallower level.
///
/// A marker with anything but a heading under it lifts nothing, rather than
/// guessing where the section it meant begins.
fn marked_sections(text: &str) -> Vec<Vec<&str>> {
    let lines: Vec<&str> = text.lines().collect();
    let levels = heading_levels(&lines);
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if line.trim() != MARKER || i + 1 >= lines.len() || levels[i + 1] == 0 {
            continue;
        }
        let level = levels[i + 1];
        let end = (i + 2..lines.len())
            .find(|&j| levels[j] > 0 && levels[j] <= level)
            .unwrap_or(lines.len());
        out.push(lines[i + 1..end].to_vec());
    }
    out
}

/// The heading level of each line, and `0` for a line that is not a heading. A
/// `#` inside a fenced block is a comment or a prompt, never a heading.
fn heading_levels(lines: &[&str]) -> Vec<usize> {
    let mut levels = vec![0; lines.len()];
    let mut fenced = false;
    for (i, line) in lines.iter().enumerate() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            continue;
        }
        let hashes = line.len() - line.trim_start_matches('#').len();
        if !fenced && hashes > 0 && line[hashes..].starts_with(' ') {
            levels[i] = hashes;
        }
    }
    levels
}

/// One lifted section under the primer's own title: its headings demoted by one
/// level, the marker of whatever section follows it dropped, and no trailing
/// blank line.
fn push_section(out: &mut String, lines: &[&str]) {
    let levels = heading_levels(lines);
    let mut body = String::new();
    for (i, line) in lines.iter().enumerate() {
        if line.trim() == MARKER {
            continue;
        }
        if levels[i] > 0 {
            body.push('#');
        }
        body.push_str(line.trim_end());
        body.push('\n');
    }
    out.push('\n');
    out.push_str(body.trim_end());
    out.push('\n');
}

/// One line per member the registry holds, and one per enum, exception and
/// attribute beside them (`rule:tooling/the-index-is-one-line-per-member`).
pub(crate) fn index() -> ExitCode {
    let document = crate::meta::document();
    let mut out = String::new();
    for entry in entries(&document) {
        out.push_str(&entry.line);
        out.push('\n');
    }
    print!("{out}");
    ExitCode::SUCCESS
}

/// The index lines whose symbol contains `query`, compared without case.
pub(crate) fn find(query: &str) -> ExitCode {
    let document = crate::meta::document();
    let wanted = query.to_lowercase();
    let mut out = String::new();
    for entry in entries(&document) {
        if entry.symbol.to_lowercase().contains(&wanted) {
            out.push_str(&entry.line);
            out.push('\n');
        }
    }
    print!("{out}");
    ExitCode::SUCCESS
}

/// One symbol's card, or a refusal naming the symbols nearest to what was asked
/// for.
pub(crate) fn show(symbol: &str) -> ExitCode {
    let document = crate::meta::document();
    let entries = entries(&document);
    // A generic member's line opens `Core\Json::decodeAs<T>(…`, and the whole
    // leading token is what an agent copies, so the type parameters come off
    // here rather than being a spelling the surface refuses.
    let wanted = match symbol.split_once('<') {
        Some((name, _)) => name.to_lowercase(),
        None => symbol.to_lowercase(),
    };

    if let Some(entry) = entries
        .iter()
        .find(|entry| entry.symbol.to_lowercase() == wanted)
    {
        print!("{}", card(&document, entry));
        return ExitCode::SUCCESS;
    }

    eprintln!("no symbol named `{symbol}`");
    let nearest = nearest(&entries, &wanted);
    if nearest.is_empty() {
        eprintln!("`nvs agent index` lists every symbol there is.");
    } else {
        eprintln!("nearest:");
        for line in nearest {
            eprintln!("  {line}");
        }
    }
    ExitCode::FAILURE
}

/// The symbols closest to what was asked for, longest shared run first.
///
/// A wrong guess is usually a near miss — a plural, a `Core\Str` where the
/// member is on `Core\Arr` — so the ranking is the longest run of characters the
/// two names share, which recovers a misspelling in the middle as well as a
/// wrong prefix. Nothing shorter than three characters is a match, or every
/// symbol is one.
fn nearest(entries: &[Entry], wanted: &str) -> Vec<String> {
    let mut scored: Vec<(usize, &str)> = entries
        .iter()
        .map(|entry| {
            (
                shared_run(&entry.symbol.to_lowercase(), wanted),
                entry.symbol.as_str(),
            )
        })
        .filter(|(run, _)| *run >= 3)
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
    scored.truncate(8);
    scored
        .into_iter()
        .map(|(_, symbol)| symbol.to_owned())
        .collect()
}

/// The length of the longest run of bytes `a` and `b` share.
fn shared_run(a: &str, b: &str) -> usize {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut previous = vec![0usize; b.len() + 1];
    let mut current = vec![0usize; b.len() + 1];
    let mut longest = 0;
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            current[j] = if a[i - 1] == b[j - 1] {
                previous[j - 1] + 1
            } else {
                0
            };
            longest = longest.max(current[j]);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    longest
}

/// One symbol's card: its index line, then whatever its row wrote and nothing
/// for what it did not.
fn card(document: &Value, entry: &Entry) -> String {
    let mut out = format!("{}\n", entry.line);
    let Some((class_name, member_name)) = entry.symbol.split_once("::") else {
        push_roster_card(&mut out, document, &entry.symbol);
        return out;
    };

    let Some(member) = array(document, "classes")
        .iter()
        .find(|class| text(class, "name") == class_name)
        .map(|class| array(class, "members"))
        .and_then(|members| {
            members
                .iter()
                .find(|member| text(member, "name") == member_name)
        })
    else {
        return out;
    };

    let doc = &member["doc"];
    push_prose(&mut out, text(doc, "short"));
    push_pairs(&mut out, "params", array(doc, "params"), |param| {
        (format!("${}", text(param, "name")), text(param, "desc"))
    });
    push_prose_section(&mut out, "returns", text(doc, "return"));
    push_pairs(&mut out, "throws", array(doc, "errors"), |error| {
        (text(error, "error").to_owned(), text(error, "desc"))
    });
    out
}

/// The card for a symbol that is not a member: an enum's cases, an exception's
/// own properties, or an attribute, which is a name and nothing else.
fn push_roster_card(out: &mut String, document: &Value, symbol: &str) {
    if let Some(core_enum) = array(document, "enums")
        .iter()
        .find(|core_enum| text(core_enum, "name") == symbol)
    {
        let doc = &core_enum["doc"];
        push_prose(out, text(doc, "short"));
        push_pairs(out, "cases", array(doc, "cases"), |case| {
            (text(case, "name").to_owned(), text(case, "desc"))
        });
        return;
    }
    if let Some(exception) = array(document, "exceptions")
        .iter()
        .find(|exception| text(exception, "name") == symbol)
    {
        let own: Vec<(String, &str)> = array(exception, "properties")
            .iter()
            .filter_map(|property| Some((property.as_str()?.to_owned(), "")))
            .collect();
        push_pairs(out, "properties", &own, |(name, desc)| {
            (name.clone(), *desc)
        });
    }
}

/// A paragraph under the index line, or nothing where none was written.
fn push_prose(out: &mut String, prose: &str) {
    if !prose.is_empty() {
        out.push('\n');
        out.push_str(prose);
        out.push('\n');
    }
}

/// A one-line section, or nothing where the field is absent.
fn push_prose_section(out: &mut String, heading: &str, prose: &str) {
    if !prose.is_empty() {
        out.push_str(&format!("\n{heading}:\n  {prose}\n"));
    }
}

/// A heading and its rows, aligned on the widest name — or nothing at all when
/// the list is absent, which is the document's omission rule read back out.
fn push_pairs<T>(out: &mut String, heading: &str, rows: &[T], each: impl Fn(&T) -> (String, &str)) {
    let rows: Vec<(String, &str)> = rows.iter().map(each).collect();
    if rows.is_empty() {
        return;
    }
    let width = rows.iter().map(|(name, _)| name.len()).max().unwrap_or(0);
    out.push_str(&format!("\n{heading}:\n"));
    for (name, desc) in rows {
        if desc.is_empty() {
            out.push_str(&format!("  {name}\n"));
        } else {
            out.push_str(&format!("  {name:<width$}  {desc}\n"));
        }
    }
}
