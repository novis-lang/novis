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
//! No answer is written to disk and nothing is cached. The document is rebuilt
//! per call from compile-time tables, so the answer cannot describe a version
//! that is not installed — which is the failure the whole surface exists to
//! remove. `init` is the one command here that writes a file, and what it writes
//! is a pointer at the four above rather than anything they would have said.
//!
//! ## The install
//!
//! `init` writes an `AGENTS.md` stanza — harness-neutral, delimited by markers
//! it owns — and beside it one adapter per harness the tree shows (a Claude Code
//! skill, a Cursor rule, a Copilot instructions file), each naming those
//! commands and the `nvs check` loop.
//! `rule:tooling/an-adapter-carries-protocol-and-never-language` is why none of
//! them names anything else: a language fact written into a pointer is a copy
//! that goes stale the day the member changes, and every copy is read by an
//! agent with no way to know it is old. So [`PROTOCOL`] is the whole of what
//! every pointer says, and an [`Adapter`] contributes only the front matter its
//! own harness reads it through.
//!
//! Every unit `init` writes carries a marker with the [`fingerprint`] of the
//! text around it, so a re-run tells an upgrade from an edit: a unit that still
//! matches its fingerprint is replaced with this binary's text, and one that
//! does not is refused unless `--force`, because overwriting a reader's own
//! sentence is the worse mistake. A unit an `init` before fingerprints wrote is
//! held to the `LEGACY_*` fingerprint of that text. `--check` writes nothing
//! and fails while any unit is missing, outdated or edited, and `primer` puts
//! one line on standard error while the working directory has an outdated one,
//! so the agent that reads the primer learns of it with nobody in between.
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
//! `find` succeeds having printed nothing on standard output when a query
//! matches nothing: the index is complete, so an empty result is the answer that
//! no `Core` symbol and no chapter heading carries the word. Standard error says
//! so, and names the chapter map, because a keyword written under a heading that
//! does not name it is in the language all the same. `show` is the opposite — it
//! was asked for one specific thing and exits non-zero when it cannot produce it,
//! naming what it has instead.
//!
//! ## A chapter, and a heading in it
//!
//! The registry holds what `Core` declares and nothing the grammar does, so a
//! keyword — `autoload`, `require`, `match` — has no member to be found by. The
//! chapters this binary carries are where those are written, and [`topics`] puts
//! them on the same index: one line per chapter, whose symbol is the chapter's
//! `id`, and one per heading, whose symbol is `id#slug`:
//!
//! ```text
//! programs  chapter: Programs, files and names
//! programs#autoload-find-a-class-by-its-namespace  section: `autoload`: find a class by its namespace
//! ```
//!
//! A slug is the heading lowercased with every run of anything but a letter or a
//! digit written as one `-`, so it holds no `(`, `<` or space and the rule above
//! still cuts the symbol. `show` on a section prints the section as the chapter
//! has it; on a chapter it prints the summary and the chapter's own lines, since
//! a whole chapter is more than the question that reached it asked for.
//!
//! ## A key, a command, a flag and a code
//!
//! The index ends with a line per configuration key, command, flag and
//! diagnostic code, which [`names`] derives. Those lines open with the kind of
//! name they carry — `config: [server] max_in_flight` — so their symbol is not
//! the line's first token, and that module says what it is instead.

use std::path::Path;
use std::process::ExitCode;

use serde_json::Value;

mod names;

/// The line that marks the section under it, on a line of its own.
const MARKER: &str = "<!-- primer -->";

/// The reference chapters, in the order `docs/novis.md` concatenates them, which
/// is the order the chapter map prints them in.
///
/// The whole chapter is embedded rather than an extract of its marked sections,
/// because an extract is a second artefact that can disagree with the chapter
/// and the primer exists to be a document that cannot — and because the whole
/// chapter is what `show` answers a section from. It spends the chapters' own
/// size, a few hundred KB of the binary's read-only data — once per binary, never
/// per request — to buy that.
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
    include_str!("../../../docs/reference/tools/15-install.md"),
    include_str!("../../../docs/reference/tools/20-config.md"),
    include_str!("../../../docs/reference/tools/25-server.md"),
    include_str!("../../../docs/reference/tools/30-php-differences.md"),
    include_str!("../../../docs/reference/tools/40-editor.md"),
    include_str!("../../../docs/reference/tools/50-agents.md"),
];

/// The chapters whose marked sections open the primer, in the order it prints
/// them: the lookup protocol, the worked program, the capability model, then the
/// refusals. Every other chapter's marked sections follow in reference order, so
/// this list fixes where a section lands and never whether it is lifted.
const PRIMER_FIRST: &[&str] = &["agents", "programs", "config", "php-differences"];

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
    /// A second name `find` and `show` reach the line by: a configuration
    /// key's dotted spelling, `server.max_in_flight`, beside the
    /// `[server] max_in_flight` its line writes.
    also: Option<String>,
    /// What `show` prints under the line for an entry whose text is its own
    /// rather than the document's; empty for every other entry.
    body: String,
}

impl Entry {
    fn new(symbol: String, line: String) -> Self {
        Self {
            symbol,
            line,
            also: None,
            body: String::new(),
        }
    }

    /// Whether `test` holds for the symbol or for the second name.
    fn named(&self, test: impl Fn(&str) -> bool) -> bool {
        test(&self.symbol.to_lowercase())
            || self
                .also
                .as_deref()
                .is_some_and(|also| test(&also.to_lowercase()))
    }
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
/// class, then the enums, exceptions and attributes beside them, then each
/// chapter over its headings, then the configuration keys, the commands with
/// their flags and the diagnostic codes.
fn entries(document: &Value) -> Vec<Entry> {
    let gates = gates(document);
    let mut out = Vec::new();

    let attributes: Vec<&str> = array(document, "attributes")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    for class in array(document, "classes") {
        let class_name = text(class, "name");
        // The class has a line of its own, naming its members, so a query for
        // the class — or for a word in its name, `html` — lands on it, and a
        // memberless carrier such as `Core\Html\Markup` is on the index at all.
        // A class that is also an attribute, `Core\Test`, keeps the attribute's
        // line as its only one, and that line's card lists the members too.
        if !attributes.contains(&class_name) {
            let names: Vec<&str> = array(class, "members")
                .iter()
                .map(|member| text(member, "name"))
                .collect();
            let mut line = format!("{class_name}  class");
            if !names.is_empty() {
                line.push_str(&format!(": {}", names.join(", ")));
            }
            out.push(Entry::new(class_name.to_owned(), line));
        }
        for member in array(class, "members") {
            let symbol = format!("{class_name}::{}", text(member, "name"));
            let mut line = format!("{class_name}::{}", text(member, "signature"));
            if let Some((_, capability)) = gates.iter().find(|(gated, _)| *gated == symbol) {
                line.push_str("  [");
                line.push_str(capability);
                line.push(']');
            }
            out.push(Entry::new(symbol, line));
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
        out.push(Entry::new(name.to_owned(), line));
    }

    for exception in array(document, "exceptions") {
        let name = text(exception, "name");
        let mut line = format!("{name}  exception");
        if let Some(parent) = exception["parent"].as_str() {
            line.push_str(" extends ");
            line.push_str(parent);
        }
        out.push(Entry::new(name.to_owned(), line));
    }

    for attribute in array(document, "attributes") {
        let Some(name) = attribute.as_str() else {
            continue;
        };
        out.push(Entry::new(name.to_owned(), format!("{name}  attribute")));
    }

    for topic in topics() {
        out.push(Entry::new(topic.symbol, topic.line));
    }

    out.extend(names::config_keys());
    out.extend(names::commands());
    out.extend(names::codes());
    out
}

/// One thing a chapter is asked for: the chapter, or one heading in it.
struct Topic {
    symbol: String,
    line: String,
    /// The heading's level, and `0` for the chapter itself.
    level: usize,
    /// The section from its heading to the next heading at its level or above.
    /// Empty for a chapter, whose card is its sections' lines.
    lines: Vec<&'static str>,
}

/// Every chapter this binary carries, each followed by its headings in the order
/// the chapter writes them.
fn topics() -> Vec<Topic> {
    let mut out = Vec::new();
    for chapter in CHAPTERS.iter().map(|text| chapter(text)) {
        out.push(Topic {
            symbol: chapter.id.to_owned(),
            line: format!("{}  chapter: {}", chapter.id, chapter.title),
            level: 0,
            lines: Vec::new(),
        });

        let lines: Vec<&'static str> = chapter.text.lines().collect();
        let levels = heading_levels(&lines);
        let mut slugs: Vec<String> = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let level = levels[i];
            if level == 0 {
                continue;
            }
            let heading = line[level..].trim();
            let end = (i + 1..lines.len())
                .find(|&j| levels[j] > 0 && levels[j] <= level)
                .unwrap_or(lines.len());
            let symbol = format!("{}#{}", chapter.id, unique_slug(&mut slugs, heading));
            out.push(Topic {
                line: format!("{symbol}  section: {heading}"),
                symbol,
                level,
                lines: lines[i..end].to_vec(),
            });
        }
    }
    out
}

/// A heading as the half of a symbol after the `#`: lowercased, with every run
/// of anything but a letter or a digit written as one `-`. A chapter that writes
/// the same heading twice numbers the later ones, so every symbol resolves to
/// one section.
fn unique_slug(taken: &mut Vec<String>, heading: &str) -> String {
    let mut slug = String::new();
    for c in heading.chars() {
        if c.is_alphanumeric() {
            slug.extend(c.to_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_end_matches('-').to_owned();

    let mut unique = slug.clone();
    let mut n = 2;
    while taken.contains(&unique) {
        unique = format!("{slug}-{n}");
        n += 1;
    }
    taken.push(unique.clone());
    unique
}

/// The line with every parenthesised rule citation — a `` `rule:` `` token in
/// backticks inside brackets — and the space before it, taken out. A citation
/// names a file of this repository, which the agent reading the section does
/// not have and cannot resolve through `show`, so on the shipped surface it is
/// a dangling name. The token is spelled here without a topic and a name on
/// purpose: `bun nv rules --check` reads a whole one as a citation wherever
/// it stands, so a made-up rule id in this comment fails the rulebook's gate.
fn without_rule_citations(line: &str) -> String {
    const OPEN: &str = "(`rule:";
    const CLOSE: &str = "`)";
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(start) = rest.find(OPEN) {
        let Some(len) = rest[start..].find(CLOSE) else {
            break;
        };
        out.push_str(rest[..start].trim_end_matches(' '));
        rest = &rest[start + len + CLOSE.len()..];
    }
    out.push_str(rest);
    out
}

/// What `show` prints for a chapter or a section, or nothing where `symbol` is
/// neither.
///
/// A section is the chapter's own text with the primer's markers taken out. A
/// chapter is its summary over the lines of its sections, indented by depth.
fn topic_card(symbol: &str) -> Option<String> {
    let topics = topics();
    let topic = topics.iter().find(|topic| topic.symbol == symbol)?;
    let mut out = format!("{}\n\n", topic.line);

    if topic.level > 0 {
        let mut body = String::new();
        for line in topic.lines.iter().filter(|line| line.trim() != MARKER) {
            body.push_str(without_rule_citations(line.trim_end()).as_str());
            body.push('\n');
        }
        out.push_str(body.trim_end());
        out.push('\n');
        return Some(out);
    }

    let summary = CHAPTERS
        .iter()
        .map(|text| chapter(text))
        .find(|chapter| chapter.id == symbol)
        .map_or("", |chapter| chapter.summary);
    out.push_str(summary);
    out.push_str("\n\nsections:\n");
    let prefix = format!("{symbol}#");
    for section in topics
        .iter()
        .filter(|section| section.symbol.starts_with(&prefix))
    {
        out.push_str(&"  ".repeat(section.level));
        out.push_str(&section.line);
        out.push('\n');
    }
    Some(out)
}

/// The document that makes an agent productive: the marked chapter sections in
/// the order `rule:tooling/a-primer-claim-is-executed` fixes, then the chapter
/// map from the chapters' own front matter.
pub(crate) fn primer() -> ExitCode {
    if let Some(note) = stale_note() {
        eprintln!("{note}");
    }
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

    out.push_str(
        "\n## The chapters\n\n\
         The reference is one chapter per topic, and these are all of them. This binary carries each \
         one whole: `nvs agent show <id>` lists a chapter's sections, `nvs agent show <id>#<section>` \
         prints one, and `nvs agent find <word>` matches a heading as it matches a member — which is \
         how a keyword such as `autoload`, which no `Core` member is named for, is found.\n\n",
    );
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

/// One line per member the registry holds, one per enum, exception and
/// attribute beside them, and one per chapter and heading this binary carries
/// (`rule:tooling/the-index-is-one-line-per-member`).
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
///
/// A query nothing matches still succeeds with nothing on standard output, and
/// says on standard error what that silence covers and where to look next.
pub(crate) fn find(query: &str) -> ExitCode {
    let document = crate::meta::document();
    let wanted = query.to_lowercase();
    let mut out = String::new();
    for entry in entries(&document) {
        if entry.named(|name| name.contains(&wanted)) {
            out.push_str(&entry.line);
            out.push('\n');
        }
    }
    if out.is_empty() {
        eprintln!(
            "nothing matches `{query}`: no `Core` symbol, chapter heading, configuration key, command, flag or diagnostic code has it."
        );
        eprintln!(
            "a keyword may be written under a heading that does not name it: `nvs agent primer` ends with the chapter map, and `nvs agent show <chapter>` lists one chapter's sections."
        );
    }
    print!("{out}");
    ExitCode::SUCCESS
}

/// One symbol's card, or a refusal naming the symbols nearest to what was asked
/// for.
pub(crate) fn show(symbol: &str) -> ExitCode {
    let document = crate::meta::document();
    if let Some(entry) = show_target(&document, symbol) {
        print!("{}", card(&document, &entry));
        return ExitCode::SUCCESS;
    }

    eprintln!("no symbol named `{symbol}`");
    let nearest = nearest(&entries(&document), &wanted(symbol));
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

/// The entry whose symbol, or second name, is `symbol`, compared without case.
fn show_target(document: &Value, symbol: &str) -> Option<Entry> {
    let wanted = wanted(symbol);
    entries(document)
        .into_iter()
        .find(|entry| entry.named(|name| name == wanted))
}

/// `symbol` as `show` compares it: lowercased, and with a generic member's type
/// parameters taken off. A generic member's line opens `Core\Json::decodeAs<T>(…`,
/// and the whole leading token is what an agent copies, so `show` accepts it.
fn wanted(symbol: &str) -> String {
    match symbol.split_once('<') {
        Some((name, _)) => name.to_lowercase(),
        None => symbol.to_lowercase(),
    }
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
    if !entry.body.is_empty() {
        push_prose(&mut out, &entry.body);
        return out;
    }
    let Some((class_name, member_name)) = entry.symbol.split_once("::") else {
        if let Some(card) = topic_card(&entry.symbol) {
            return card;
        }
        push_class_card(&mut out, document, &entry.symbol);
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

/// The card for a class: its own prose when the registry row carries a
/// `ClassDoc`, then the index line of each of its members, so the class name
/// alone is a way into its members without knowing one of them. A class with
/// no card and no members writes nothing under its line.
fn push_class_card(out: &mut String, document: &Value, symbol: &str) {
    let Some(class) = array(document, "classes")
        .iter()
        .find(|class| text(class, "name") == symbol)
    else {
        return;
    };
    push_prose(out, text(&class["doc"], "short"));
    let members = array(class, "members");
    if members.is_empty() {
        return;
    }
    out.push_str("\nmembers:\n");
    for member in members {
        out.push_str(&format!("  {symbol}::{}\n", text(member, "signature")));
    }
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

/// What every marker `init` writes opens with. The `AGENTS.md` stanza opens on
/// the marker, and an adapter carries it on the line under its front matter.
///
/// The whole marker is this, `, fingerprint `, the [`fingerprint`] of the text
/// around it, and ` -->`. An HTML comment is what every harness here passes
/// over, and a front-matter key is not: a harness may reject a key it does not
/// know.
const MARK: &str = "<!-- nvs agent: written by `nvs agent init`";

/// The marker an `init` that wrote no fingerprint opened its stanza with.
/// A unit that carries it, or an adapter with no marker at all, is judged
/// against the `legacy` fingerprint its [`Unit`] names.
const LEGACY_OPEN: &str = "<!-- nvs agent: written by `nvs agent init` -->";

/// The line that closes the `AGENTS.md` stanza.
///
/// The open marker and this delimit the region `init` owns, so a project's own
/// prose above and below the pair is never read.
const STANZA_CLOSE: &str = "<!-- /nvs agent -->";

/// The [`fingerprint`] of the stanza, and of the Claude Code skill, as the
/// `init` before fingerprints wrote them. A file that still reads as that text
/// was not edited, so it is updated like any other.
const LEGACY_STANZA: &str = "78328bd5";
const LEGACY_CLAUDE: &str = "e55b58b7";

/// The fingerprint of a unit `init` writes: the first eight hex digits of the
/// BLAKE3 hash of its text with the marker line taken out and every `\r\n`
/// read as `\n`. BLAKE3 is fixed by its specification, so the same text gives
/// the same fingerprint on every platform and from every build. A checkout
/// under Git's `core.autocrlf` therefore still reads as untouched.
fn fingerprint(text: &str) -> String {
    let text = text.replace("\r\n", "\n");
    blake3::hash(text.as_bytes()).to_hex()[..8].to_owned()
}

/// A unit's text: `before`, the marker carrying the fingerprint of `before`
/// and `after` together, then `after`.
fn owned(before: &str, after: &str) -> String {
    let print = fingerprint(&format!("{before}{after}"));
    format!("{before}{MARK}, fingerprint {print} -->\n{after}")
}

/// What every pointer says, in every harness: where to ask, and the check loop.
///
/// Not a signature, not a type, not a refusal —
/// `rule:tooling/an-adapter-carries-protocol-and-never-language` is why this is
/// the whole of it, and why one const serves every harness: a pointer that says
/// nothing about the language cannot disagree with the binary that answers.
const PROTOCOL: &str = "\
Novis is the language this project is written in, and the `nvs` binary installed on this machine is
its documentation. It answers from the registry it compiles against, so an answer always describes
the installed version. That is why nothing about the language itself is written here.

- `nvs agent primer` — read once, before writing anything. The short document that makes a coding
  agent productive.
- `nvs agent index` — one line per standard library member, reference heading, configuration key,
  command, flag and error code.
- `nvs agent find <query>` — the index lines whose name matches: a member, a keyword such as
  `autoload`, a configuration key, a command, a flag or an error code. Use it instead of grep: the
  shell removes the backslash from a namespaced name before grep sees it.
- `nvs agent show <symbol>` — one card: a member's signature, prose, parameters and errors, a
  reference section, a key, a command's help, or an error code, as in `nvs agent show E0621`.

Then check what you wrote. `nvs check <file>` names what is wrong and where, `nvs check --json <file>`
prints the same diagnostics as JSON, and `nvs test` runs the tests. That is the loop: read the primer
once, `find` a name, `show` its card, `nvs check`.

`nvs agent init` wrote this text, and running it again updates it to the installed `nvs`.
";

/// The front matter a Claude Code skill is found and summarised by.
///
/// Each front matter here is spelled a line at a time so the const stays
/// indented with the code around it: a multi-line literal would have to begin
/// every one of its lines at column 0. The format is the one thing here a
/// harness owns.
const CLAUDE_FRONT: &str = concat!(
    "---\n",
    "name: novis\n",
    "description: Ask the installed `nvs` binary about the Novis language and check what you \
     wrote — the primer, the index, one symbol's card, then `nvs check`. Use it whenever reading \
     or writing Novis code.\n",
    "---\n",
);

/// The front matter of a Cursor project rule. The agent attaches the rule when a
/// `.nvs` file is in play, or when the description says it is relevant. Cursor
/// reads `globs` unquoted.
const CURSOR_FRONT: &str = concat!(
    "---\n",
    "description: How to look up the Novis language and check Novis code with the installed `nvs` \
     binary. Use it whenever reading or writing Novis code.\n",
    "globs: **/*.nvs\n",
    "alwaysApply: false\n",
    "---\n",
);

/// The front matter of a GitHub Copilot path-specific instructions file, which
/// applies it to every `.nvs` file.
const COPILOT_FRONT: &str = concat!("---\n", "applyTo: \"**/*.nvs\"\n", "---\n");

/// One harness's pointer: where its file goes, the paths whose presence says
/// that harness is in use here, the front matter it opens with, and the
/// fingerprint of the file an `init` before fingerprints wrote there, when it
/// wrote one.
///
/// The front matter is the only part a harness owns. What the file *says* is
/// [`PROTOCOL`], identical in every one of them, which is what keeps this list
/// open: another harness is another row, adding one decides nothing, and none of
/// them can disagree with the language because none of them says anything about
/// it.
#[derive(Debug)]
struct Adapter {
    path: &'static str,
    present: &'static [&'static str],
    front: &'static str,
    legacy: Option<&'static str>,
}

impl Adapter {
    /// The whole file: this harness's front matter, the marker, and the
    /// protocol every pointer carries under one title.
    fn text(&self) -> String {
        owned(self.front, &format!("\n# Novis\n\n{PROTOCOL}"))
    }

    /// Whether the tree under `root` shows this harness in use.
    fn is_present(&self, root: &Path) -> bool {
        self.present.iter().any(|path| root.join(path).exists())
    }
}

/// Every harness `init` knows how to point at. Copilot is detected by its own
/// instruction files, because a `.github` directory alone is in most projects
/// whichever harness they use.
const ADAPTERS: &[Adapter] = &[
    Adapter {
        path: ".claude/skills/novis/SKILL.md",
        present: &[".claude"],
        front: CLAUDE_FRONT,
        legacy: Some(LEGACY_CLAUDE),
    },
    Adapter {
        path: ".cursor/rules/novis.mdc",
        present: &[".cursor"],
        front: CURSOR_FRONT,
        legacy: None,
    },
    Adapter {
        path: ".github/instructions/novis.instructions.md",
        present: &[".github/copilot-instructions.md", ".github/instructions"],
        front: COPILOT_FRONT,
        legacy: None,
    },
];

/// The stanza as it belongs in `AGENTS.md`, with no trailing newline, which is
/// both what `init` writes and what it compares an existing stanza against.
fn stanza() -> String {
    owned("", &format!("\n## Novis\n\n{PROTOCOL}\n{STANZA_CLOSE}"))
}

/// How a unit `init` writes stands against the text this binary writes.
#[derive(Debug, PartialEq)]
enum State {
    /// It reads as this binary writes it.
    Current,
    /// It reads as an earlier `init` wrote it, so nobody edited it.
    Outdated,
    /// Its text no longer matches the fingerprint it carries, or it carries none.
    Edited,
}

/// Judge `found`, a unit read from disk, against `written`, what this binary
/// writes there. `legacy` is the fingerprint a unit with the old marker, or
/// with no marker, is held to.
fn judge(found: &str, written: &str, legacy: Option<&str>) -> State {
    let found = found.replace("\r\n", "\n");
    if found == written {
        return State::Current;
    }
    let (line, rest) = match found.find(MARK) {
        Some(start) => {
            let end = found[start..]
                .find('\n')
                .map_or(found.len(), |at| start + at + 1);
            let line = found[start..end].trim_end_matches('\n');
            (Some(line), format!("{}{}", &found[..start], &found[end..]))
        }
        None => (None, found.clone()),
    };
    let recorded = match line {
        Some(line) if line != LEGACY_OPEN => line
            .strip_prefix(MARK)
            .and_then(|tail| tail.strip_prefix(", fingerprint "))
            .and_then(|tail| tail.strip_suffix(" -->")),
        _ => legacy,
    };
    if recorded.is_some_and(|recorded| recorded == fingerprint(&rest)) {
        State::Outdated
    } else {
        State::Edited
    }
}

/// What one run owes one file: nothing, the text to create it with, the text
/// to update it to, or a refusal unless `--force`, which carries the text
/// `--force` writes instead.
#[derive(Debug)]
enum Owed {
    Nothing,
    Missing(String),
    Outdated(String),
    Edited(String),
}

impl Owed {
    /// The owed state of a unit whose replacement is `text`.
    fn from(state: State, text: String) -> Self {
        match state {
            State::Current => Owed::Nothing,
            State::Outdated => Owed::Outdated(text),
            State::Edited => Owed::Edited(text),
        }
    }
}

/// Install the surface into this project: the `AGENTS.md` stanza, and one
/// adapter for each harness the tree shows — every one of them when `all`.
///
/// A missing file is written, a file that still matches its fingerprint is
/// updated to this binary's text, and an edited one is refused unless `force`.
/// With `check` nothing is written, and each file that is missing, outdated or
/// edited is named instead. Every file is read and judged before any is
/// written, so a refusal about one of them leaves all of them as they were.
pub(crate) fn init(all: bool, force: bool, check: bool) -> ExitCode {
    let root = match std::env::current_dir() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("error: this directory cannot be read: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut owed: Vec<(&'static str, Owed)> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    match stanza_owed(&root.join("AGENTS.md")) {
        Ok(state) => owed.push(("AGENTS.md", state)),
        Err(error) => errors.push(error),
    }
    for adapter in ADAPTERS {
        if !all && !adapter.is_present(&root) {
            continue;
        }
        match adapter_owed(&root, adapter) {
            Ok(state) => owed.push((adapter.path, state)),
            Err(error) => errors.push(error),
        }
    }
    for error in &errors {
        eprintln!("error: {error}");
    }

    if check {
        let mut clean = errors.is_empty();
        for (path, state) in &owed {
            let word = match state {
                Owed::Nothing => continue,
                Owed::Missing(_) => "missing",
                Owed::Outdated(_) => "outdated",
                Owed::Edited(_) => "edited",
            };
            println!("{word} {path}");
            clean = false;
        }
        if !clean {
            return ExitCode::FAILURE;
        }
        println!("up to date");
        return ExitCode::SUCCESS;
    }

    let mut refused = !errors.is_empty();
    if !force {
        for (path, state) in &owed {
            if matches!(state, Owed::Edited(_)) {
                refused = true;
                eprintln!(
                    "error: {path} has changed since `nvs agent init` wrote it, so it is left \
                     alone; run `nvs agent init --force` to replace it"
                );
            }
        }
    }
    if refused {
        return ExitCode::FAILURE;
    }

    let mut wrote = false;
    for (path, state) in owed {
        let (word, text) = match state {
            Owed::Nothing => continue,
            Owed::Missing(text) => ("wrote", text),
            Owed::Outdated(text) | Owed::Edited(text) => ("updated", text),
        };
        if let Err(error) = write_file(&root.join(path), &text) {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
        println!("{word} {path}");
        wrote = true;
    }
    if !wrote {
        println!("up to date");
    }
    ExitCode::SUCCESS
}

/// The one line `primer` puts on standard error when the project in the working
/// directory carries files an older `init` wrote and nobody edited since, or
/// carries the stanza and lacks the adapter of a harness it shows. `None` when
/// it carries none of them. It reads only `AGENTS.md` and the adapter paths.
pub(crate) fn stale_note() -> Option<&'static str> {
    let root = std::env::current_dir().ok()?;
    let stanza = stanza_owed(&root.join("AGENTS.md")).ok()?;
    let installed = !matches!(stanza, Owed::Missing(_));
    let mut stale = matches!(stanza, Owed::Outdated(_));
    for adapter in ADAPTERS {
        match adapter_owed(&root, adapter) {
            Ok(Owed::Outdated(_)) => stale = true,
            Ok(Owed::Missing(_)) if installed && adapter.is_present(&root) => stale = true,
            _ => {}
        }
    }
    stale.then_some(
        "note: this project's agent files are older than this nvs; \
         run `nvs agent init` to update them",
    )
}

/// What `AGENTS.md` owes the stanza.
///
/// A file with no stanza gains one at its foot, so a project's own instructions
/// keep the opening of their own document. A stanza is judged by [`judge`], and
/// its replacement changes only the region between the markers.
fn stanza_owed(path: &Path) -> Result<Owed, String> {
    let block = stanza();
    let existing = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Owed::Missing(format!("{block}\n")));
        }
        Err(error) => return Err(format!("AGENTS.md cannot be read: {error}")),
    };

    let Some(open) = existing.find(MARK) else {
        let mut out = existing;
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&block);
        out.push('\n');
        return Ok(Owed::Missing(out));
    };

    let Some(close) = existing[open..].find(STANZA_CLOSE) else {
        return Err(format!(
            "AGENTS.md opens an `nvs agent` stanza and never closes it; \
             close it with `{STANZA_CLOSE}` or delete it, then run this again"
        ));
    };

    let end = open + close + STANZA_CLOSE.len();
    let state = judge(&existing[open..end], &block, Some(LEGACY_STANZA));
    let replaced = format!("{}{block}{}", &existing[..open], &existing[end..]);
    Ok(Owed::from(state, replaced))
}

/// What one harness's pointer owes: the file when it is missing, otherwise what
/// [`judge`] makes of the file there.
fn adapter_owed(root: &Path, adapter: &Adapter) -> Result<Owed, String> {
    let text = adapter.text();
    match std::fs::read_to_string(root.join(adapter.path)) {
        Ok(found) => Ok(Owed::from(judge(&found, &text, adapter.legacy), text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Owed::Missing(text)),
        Err(error) => Err(format!("{} cannot be read: {error}", adapter.path)),
    }
}

/// Write `body` to `path`, creating the directories above it.
fn write_file(path: &Path, body: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("{} cannot be created: {error}", parent.display()))?;
    }
    std::fs::write(path, body)
        .map_err(|error| format!("{} cannot be written: {error}", path.display()))
}
