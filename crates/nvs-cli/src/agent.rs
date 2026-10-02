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
//! it owns — and beside it what the agent running it needs, by [`AGENTS`]: the
//! row whose environment variable is set, the `--agent` names, or every row
//! with `--all`. An agent that reads `AGENTS.md` itself gets nothing more; the
//! Claude Code skill and the Copilot instructions file are the two pointers,
//! each naming those commands and the `nvs check` loop. A pointer an earlier run
//! wrote is kept current whichever agent runs now, and one this binary no
//! longer writes ([`RETIRED`]) is deleted while nobody has edited it.
//! `rule:tooling/an-adapter-carries-protocol-and-never-language` is why none of
//! them names anything else: a language fact written into a pointer is a copy
//! that goes stale the day the member changes, and every copy is read by an
//! agent with no way to know it is old. So [`PROTOCOL`] is the whole of what
//! every pointer says, and an [`Adapter`] contributes only the front matter its
//! own harness reads it through.
//!
//! An agent whose tool runs a plain command after each edit also gets a
//! [`HookFile`] entry in that tool's settings: `nvs agent hook <agent>`, which
//! [`hook`] answers by checking the edited `.nvs` file as `nvs check` does and
//! giving the errors back in the shape that tool reads. The hook is protocol in
//! the same sense: it runs the checker and states nothing itself. The entry is
//! merged into a file the project owns, so [`Doc`] keeps every key and entry in
//! the order it found them, the entry is found again by its command alone, and
//! a file that is not the JSON object the tool reads refuses the run whatever
//! `--force` says. `--no-hooks` removes the entry instead of adding it.
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
//!
//! ## `--json`
//!
//! `index`, `find` and `show` each take `--json` for a consumer that parses
//! rather than reads, and the document is another rendering of the same
//! [`Entry`] and the same card, never a source of its own: the text card is
//! rendered from the very [`card`] map `show --json` prints. Its shape follows
//! `nvs check --json`'s (`rule:ide/check-json-is-the-diagnostic-record-as-a-document`):
//! `schemaVersion` at the root, pretty-printed, every key present with `null`
//! or `[]` for what was not written, and on a failure the document still goes
//! to standard output while the exit status says it failed.
//!
//! ```text
//! index, find  {schemaVersion, entries: [record]}
//! record       {symbol, kind, line, alias, signature, capability, title, summary}
//! show         record + the card's parts for its kind:
//!                member     prose, params [{name, desc}], returns, throws [{error, desc}]
//!                class      prose, members [{symbol, signature}]   (and an attribute)
//!                enum       prose, cases [{name, desc}]
//!                exception  parent, properties [string]
//!                chapter    sections [{symbol, title, level, line}]
//!                section, config, command, flag, code   text
//! unknown      {schemaVersion, error, nearest: [record]}, exit non-zero
//! ```
//!
//! A record's keys are the parts its line is written from, so a record says
//! nothing its line does not, apart from a chapter's summary.

use std::cell::RefCell;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use nvs_diagnostics::{Diagnostics, Renderer, Severity, SourceMap};
use serde_json::{Map, Value, json};

mod names;

/// The shape of the three `--json` documents the module doc sketches, frozen
/// the way `nvs check --json`'s is. It goes up when a key is removed or its
/// meaning changes; a key added beside the others does not move it.
const SCHEMA_VERSION: u64 = 1;

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

/// One line of the index: the symbol `show` resolves, the kind of name it is,
/// and the line `index` prints for it.
struct Entry {
    symbol: String,
    /// `class`, `member`, `enum`, `exception`, `attribute`, `chapter`,
    /// `section`, `config`, `command`, `flag` or `code`.
    kind: &'static str,
    line: String,
    /// A second name `find` and `show` reach the line by: a configuration
    /// key's dotted spelling, `server.max_in_flight`, beside the
    /// `[server] max_in_flight` its line writes.
    also: Option<String>,
    /// A member's signature as the document spells it, without the class.
    signature: Option<String>,
    /// The capability a member's call is gated on, joined from the roster.
    capability: Option<String>,
    /// A chapter's title, or a heading's text.
    title: Option<String>,
    /// A chapter's front-matter summary, or the first sentence of a code's card.
    summary: Option<String>,
    /// What `show` prints under the line for an entry whose text is its own
    /// rather than the document's; empty for every other entry.
    body: String,
}

impl Entry {
    fn new(kind: &'static str, symbol: String, line: String) -> Self {
        Self {
            symbol,
            kind,
            line,
            also: None,
            signature: None,
            capability: None,
            title: None,
            summary: None,
            body: String::new(),
        }
    }

    /// The entry as one record of the `--json` documents. Every key is always
    /// present, and a part this kind of entry does not have is `null`.
    fn record(&self) -> Map<String, Value> {
        let mut out = Map::new();
        out.insert("symbol".into(), Value::from(self.symbol.as_str()));
        out.insert("kind".into(), Value::from(self.kind));
        out.insert("line".into(), Value::from(self.line.as_str()));
        out.insert("alias".into(), Value::from(self.also.clone()));
        out.insert("signature".into(), Value::from(self.signature.clone()));
        out.insert("capability".into(), Value::from(self.capability.clone()));
        out.insert("title".into(), Value::from(self.title.clone()));
        out.insert("summary".into(), Value::from(self.summary.clone()));
        out
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
            out.push(Entry::new("class", class_name.to_owned(), line));
        }
        for member in array(class, "members") {
            let symbol = format!("{class_name}::{}", text(member, "name"));
            let signature = text(member, "signature");
            let mut line = format!("{class_name}::{signature}");
            let capability = gates
                .iter()
                .find(|(gated, _)| *gated == symbol)
                .map(|(_, capability)| *capability);
            if let Some(capability) = capability {
                line.push_str("  [");
                line.push_str(capability);
                line.push(']');
            }
            let mut entry = Entry::new("member", symbol, line);
            entry.signature = Some(signature.to_owned());
            entry.capability = capability.map(str::to_owned);
            out.push(entry);
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
        out.push(Entry::new("enum", name.to_owned(), line));
    }

    for exception in array(document, "exceptions") {
        let name = text(exception, "name");
        let mut line = format!("{name}  exception");
        if let Some(parent) = exception["parent"].as_str() {
            line.push_str(" extends ");
            line.push_str(parent);
        }
        out.push(Entry::new("exception", name.to_owned(), line));
    }

    for attribute in array(document, "attributes") {
        let Some(name) = attribute.as_str() else {
            continue;
        };
        out.push(Entry::new(
            "attribute",
            name.to_owned(),
            format!("{name}  attribute"),
        ));
    }

    for topic in topics() {
        let kind = if topic.level == 0 {
            "chapter"
        } else {
            "section"
        };
        let mut entry = Entry::new(kind, topic.symbol, topic.line);
        entry.title = Some(topic.title);
        entry.summary = topic.summary.map(str::to_owned);
        out.push(entry);
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
    /// The chapter's title, or the heading's text.
    title: String,
    /// The chapter's front-matter summary; nothing for a heading.
    summary: Option<&'static str>,
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
            title: chapter.title.to_owned(),
            summary: Some(chapter.summary),
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
                title: heading.to_owned(),
                summary: None,
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

/// The card's own parts for a chapter or a section: a section's `text`, which
/// is the chapter's own text with the primer's markers taken out, or a
/// chapter's `sections`, each with its depth. The chapter's summary is already
/// on its entry.
fn topic_card(symbol: &str) -> Map<String, Value> {
    let topics = topics();
    let mut out = Map::new();
    let Some(topic) = topics.iter().find(|topic| topic.symbol == symbol) else {
        return out;
    };

    if topic.level > 0 {
        let mut body = String::new();
        for line in topic.lines.iter().filter(|line| line.trim() != MARKER) {
            body.push_str(without_rule_citations(line.trim_end()).as_str());
            body.push('\n');
        }
        out.insert("text".into(), Value::from(body.trim_end()));
        return out;
    }

    let prefix = format!("{symbol}#");
    let sections: Vec<Value> = topics
        .iter()
        .filter(|section| section.symbol.starts_with(&prefix))
        .map(|section| {
            json!({
                "symbol": section.symbol,
                "title": section.title,
                "level": section.level,
                "line": section.line,
            })
        })
        .collect();
    out.insert("sections".into(), Value::from(sections));
    out
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
pub(crate) fn index(json: bool) -> ExitCode {
    let document = crate::meta::document();
    print!("{}", lines(&entries(&document), json));
    ExitCode::SUCCESS
}

/// `entries` as `index` and `find` print them: one line each, or with `json`
/// the document whose `entries` are their records.
fn lines(entries: &[Entry], json: bool) -> String {
    if json {
        let records: Vec<Value> = entries
            .iter()
            .map(|entry| Value::Object(entry.record()))
            .collect();
        return pretty(json!({ "schemaVersion": SCHEMA_VERSION, "entries": records }));
    }
    let mut out = String::new();
    for entry in entries {
        out.push_str(&entry.line);
        out.push('\n');
    }
    out
}

/// A `--json` document as it is printed: pretty, as `nvs check --json`'s is,
/// with a line break after it.
fn pretty(document: Value) -> String {
    // It cannot fail: the document holds strings, integers and nulls, and
    // `serde_json` only errors on a non-string map key or a non-finite float.
    let mut out = serde_json::to_string_pretty(&document)
        .expect("the document holds no unserializable value");
    out.push('\n');
    out
}

/// The index lines whose symbol contains `query`, compared without case.
///
/// A query nothing matches still succeeds with nothing on standard output, or
/// with `json` a document whose `entries` is empty, and says on standard error
/// what that silence covers and where to look next.
pub(crate) fn find(query: &str, json: bool) -> ExitCode {
    let document = crate::meta::document();
    let wanted = query.to_lowercase();
    let found: Vec<Entry> = entries(&document)
        .into_iter()
        .filter(|entry| entry.named(|name| name.contains(&wanted)))
        .collect();
    if found.is_empty() {
        eprintln!(
            "nothing matches `{query}`: no `Core` symbol, chapter heading, configuration key, command, flag or diagnostic code has it."
        );
        eprintln!(
            "a keyword may be written under a heading that does not name it: `nvs agent primer` ends with the chapter map, and `nvs agent show <chapter>` lists one chapter's sections."
        );
    }
    print!("{}", lines(&found, json));
    ExitCode::SUCCESS
}

/// One symbol's card, or a refusal naming the symbols nearest to what was asked
/// for.
///
/// With `json` both go to standard output as one document, as `nvs check
/// --json`'s diagnostics do: the card, or an `error` and the `nearest` entries'
/// records. The exit status is what tells the two apart.
pub(crate) fn show(symbol: &str, json: bool) -> ExitCode {
    let document = crate::meta::document();
    if let Some(entry) = show_target(&document, symbol) {
        let card = card(&document, &entry);
        if json {
            let mut out = Map::new();
            out.insert("schemaVersion".into(), Value::from(SCHEMA_VERSION));
            out.extend(card);
            print!("{}", pretty(Value::Object(out)));
        } else {
            print!("{}", render_card(&Value::Object(card)));
        }
        return ExitCode::SUCCESS;
    }

    let entries = entries(&document);
    let nearest = nearest(&entries, &wanted(symbol));
    let error = format!("no symbol named `{symbol}`");
    if json {
        let records: Vec<Value> = nearest
            .iter()
            .map(|entry| Value::Object(entry.record()))
            .collect();
        print!(
            "{}",
            pretty(json!({
                "schemaVersion": SCHEMA_VERSION,
                "error": error,
                "nearest": records,
            }))
        );
        return ExitCode::FAILURE;
    }

    eprintln!("{error}");
    if nearest.is_empty() {
        eprintln!("`nvs agent index` lists every symbol there is.");
    } else {
        eprintln!("nearest:");
        for entry in nearest {
            eprintln!("  {}", entry.symbol);
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
fn nearest<'a>(entries: &'a [Entry], wanted: &str) -> Vec<&'a Entry> {
    let mut scored: Vec<(usize, &Entry)> = entries
        .iter()
        .map(|entry| (shared_run(&entry.symbol.to_lowercase(), wanted), entry))
        .filter(|(run, _)| *run >= 3)
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.symbol.cmp(&b.1.symbol)));
    scored.truncate(8);
    scored.into_iter().map(|(_, entry)| entry).collect()
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

/// What a row lookup gives where the row is absent: indexing it gives `null`
/// again, so every field read off it is the empty one.
static ABSENT: Value = Value::Null;

/// `text` as a card part: `null` where nothing was written.
fn or_null(text: &str) -> Value {
    if text.is_empty() {
        Value::Null
    } else {
        Value::from(text)
    }
}

/// The row named `name` in the document's array under `key`.
fn row<'a>(document: &'a Value, key: &str, name: &str) -> &'a Value {
    array(document, key)
        .iter()
        .find(|row| text(row, "name") == name)
        .unwrap_or(&ABSENT)
}

/// `{name, desc}` for each row, the shape of a parameter and of an enum case.
fn named_rows(rows: &[Value], name: &str) -> Value {
    rows.iter()
        .map(|row| json!({ name: text(row, name), "desc": or_null(text(row, "desc")) }))
        .collect()
}

/// One symbol's card: the entry's record, then the parts its kind has. Every
/// part a kind has is always present, as `null` or `[]` where its row wrote
/// nothing, so the text card and `show --json` are two renderings of this.
///
/// A class's card lists its members, so the class name alone is a way into its
/// members without knowing one of them. An attribute that is also a class,
/// `Core\Test`, has the same card; one that is not has no members.
fn card(document: &Value, entry: &Entry) -> Map<String, Value> {
    let mut out = entry.record();
    let symbol = entry.symbol.as_str();
    match entry.kind {
        "member" => {
            let (class_name, member_name) = symbol.split_once("::").unwrap_or((symbol, ""));
            let doc = &row(row(document, "classes", class_name), "members", member_name)["doc"];
            out.insert("prose".into(), or_null(text(doc, "short")));
            out.insert("params".into(), named_rows(array(doc, "params"), "name"));
            out.insert("returns".into(), or_null(text(doc, "return")));
            let throws: Value = array(doc, "errors")
                .iter()
                .map(|error| {
                    json!({ "error": text(error, "error"), "desc": or_null(text(error, "desc")) })
                })
                .collect();
            out.insert("throws".into(), throws);
        }
        "class" | "attribute" => {
            let class = row(document, "classes", symbol);
            out.insert("prose".into(), or_null(text(&class["doc"], "short")));
            let members: Value = array(class, "members")
                .iter()
                .map(|member| {
                    json!({
                        "symbol": format!("{symbol}::{}", text(member, "name")),
                        "signature": text(member, "signature"),
                    })
                })
                .collect();
            out.insert("members".into(), members);
        }
        "enum" => {
            let doc = &row(document, "enums", symbol)["doc"];
            out.insert("prose".into(), or_null(text(doc, "short")));
            out.insert("cases".into(), named_rows(array(doc, "cases"), "name"));
        }
        "exception" => {
            let exception = row(document, "exceptions", symbol);
            out.insert("parent".into(), or_null(text(exception, "parent")));
            let properties: Value = array(exception, "properties")
                .iter()
                .filter(|property| property.is_string())
                .cloned()
                .collect();
            out.insert("properties".into(), properties);
        }
        "chapter" | "section" => out.extend(topic_card(symbol)),
        _ => {
            out.insert("text".into(), or_null(&entry.body));
        }
    }
    out
}

/// The text `show` prints for a [`card`]: its index line, then whatever its
/// row wrote and nothing for what it did not.
fn render_card(card: &Value) -> String {
    let mut out = format!("{}\n", text(card, "line"));
    match text(card, "kind") {
        "member" => {
            push_prose(&mut out, text(card, "prose"));
            push_pairs(&mut out, "params", array(card, "params"), |param| {
                (format!("${}", text(param, "name")), text(param, "desc"))
            });
            push_prose_section(&mut out, "returns", text(card, "returns"));
            push_pairs(&mut out, "throws", array(card, "throws"), |error| {
                (text(error, "error").to_owned(), text(error, "desc"))
            });
        }
        "class" | "attribute" => {
            push_prose(&mut out, text(card, "prose"));
            let members = array(card, "members");
            if !members.is_empty() {
                out.push_str("\nmembers:\n");
                let class = text(card, "symbol");
                for member in members {
                    out.push_str(&format!("  {class}::{}\n", text(member, "signature")));
                }
            }
        }
        "enum" => {
            push_prose(&mut out, text(card, "prose"));
            push_pairs(&mut out, "cases", array(card, "cases"), |case| {
                (text(case, "name").to_owned(), text(case, "desc"))
            });
        }
        "exception" => {
            push_pairs(
                &mut out,
                "properties",
                array(card, "properties"),
                |property| (property.as_str().unwrap_or_default().to_owned(), ""),
            );
        }
        "section" => {
            out.push('\n');
            out.push_str(text(card, "text"));
            out.push('\n');
        }
        "chapter" => {
            out.push('\n');
            out.push_str(text(card, "summary"));
            out.push_str("\n\nsections:\n");
            for section in array(card, "sections") {
                let level = section["level"].as_u64().unwrap_or_default();
                out.push_str(&"  ".repeat(usize::try_from(level).unwrap_or_default()));
                out.push_str(text(section, "line"));
                out.push('\n');
            }
        }
        _ => push_prose(&mut out, text(card, "text")),
    }
    out
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
/// against the `legacy` fingerprint its [`Adapter`] names.
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

`index`, `find` and `show` take `--json` when a tool reads the answer instead of a person.

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

/// The front matter of the Cursor project rule an earlier `init` wrote. Nothing
/// writes it now, because Cursor reads `AGENTS.md` itself; [`RETIRED`] keeps it
/// so a re-run can tell an untouched copy from an edited one.
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

/// One agent's pointer: where its file goes, the front matter it opens with,
/// and the fingerprint of the file an `init` before fingerprints wrote there,
/// when it wrote one.
///
/// The front matter is the only part an agent owns. What the file *says* is
/// [`PROTOCOL`], identical in every one of them, which is what keeps this list
/// open: another agent is another row, adding one decides nothing, and none of
/// them can disagree with the language because none of them says anything about
/// it.
#[derive(Debug)]
struct Adapter {
    path: &'static str,
    front: &'static str,
    legacy: Option<&'static str>,
}

impl Adapter {
    /// The whole file: this agent's front matter, the marker, and the
    /// protocol every pointer carries under one title.
    fn text(&self) -> String {
        owned(self.front, &format!("\n# Novis\n\n{PROTOCOL}"))
    }
}

/// The Claude Code skill. Claude Code reads `AGENTS.md` only in a project with
/// no `CLAUDE.md`, so the skill is what reaches it everywhere else.
const CLAUDE_SKILL: Adapter = Adapter {
    path: ".claude/skills/novis/SKILL.md",
    front: CLAUDE_FRONT,
    legacy: Some(LEGACY_CLAUDE),
};

/// The Copilot path-specific instructions file. Which Copilot surface reads
/// `AGENTS.md` is not documented, and every one of them reads this file.
const COPILOT_INSTRUCTIONS: Adapter = Adapter {
    path: ".github/instructions/novis.instructions.md",
    front: COPILOT_FRONT,
    legacy: None,
};

/// Pointers an earlier `init` wrote and this one does not. A re-run deletes an
/// untouched one and leaves an edited one where it is, with a note.
const RETIRED: &[Adapter] = &[Adapter {
    path: ".cursor/rules/novis.mdc",
    front: CURSOR_FRONT,
    legacy: None,
}];

/// An environment variable whose presence says a command runs under an agent:
/// set and not empty, and when `prefix` is given, with a value that opens with
/// it.
#[derive(Debug)]
struct Detect {
    var: &'static str,
    prefix: Option<&'static str>,
}

/// A coding agent `init` installs for: the name `--agent` takes, the variables
/// that say a command runs under it, and the pointer it needs beside
/// `AGENTS.md`.
///
/// An agent that reads `AGENTS.md` itself has no pointer, and an agent whose
/// tool runs no plain command after an edit has no hook. ADR 0260 has the
/// sources for every row.
#[derive(Debug)]
struct Agent {
    name: &'static str,
    detect: &'static [Detect],
    pointer: Option<&'static Adapter>,
    hook: Option<&'static HookFile>,
}

impl Agent {
    /// The first of this agent's variables that is set, if one is.
    fn detected(&self) -> Option<&'static str> {
        self.detect.iter().find_map(|detect| {
            let value = std::env::var_os(detect.var)?;
            let value = value.to_string_lossy();
            let matches =
                !value.is_empty() && detect.prefix.is_none_or(|prefix| value.starts_with(prefix));
            matches.then_some(detect.var)
        })
    }
}

/// Every agent `init` knows, in the order it reports them.
const AGENTS: &[Agent] = &[
    Agent {
        name: "claude-code",
        // Claude Code sets this in the commands its own tools run. `CLAUDECODE`
        // is also set in an editor's integrated terminal, so it is not used.
        detect: &[Detect {
            var: "CLAUDE_CODE_CHILD_SESSION",
            prefix: None,
        }],
        pointer: Some(&CLAUDE_SKILL),
        hook: Some(&CLAUDE_HOOK),
    },
    Agent {
        name: "cursor",
        detect: &[Detect {
            var: "CURSOR_AGENT",
            prefix: None,
        }],
        pointer: None,
        hook: Some(&CURSOR_HOOK),
    },
    // Codex's hooks are experimental and give a hook the patch text, not the
    // path of the edited file, so it gets none.
    Agent {
        name: "codex",
        detect: &[Detect {
            var: "CODEX_THREAD_ID",
            prefix: None,
        }],
        pointer: None,
        hook: None,
    },
    // Copilot's hooks are a preview with two payload formats, and the fields
    // that name an edited file are not documented, so it gets none.
    Agent {
        name: "copilot",
        detect: &[
            Detect {
                var: "COPILOT_AGENT",
                prefix: None,
            },
            Detect {
                var: "AI_AGENT",
                prefix: Some("github_copilot"),
            },
            Detect {
                var: "COPILOT_AGENT_SESSION_ID",
                prefix: None,
            },
            Detect {
                var: "GITHUB_COPILOT_API_TOKEN",
                prefix: None,
            },
        ],
        pointer: Some(&COPILOT_INSTRUCTIONS),
        hook: None,
    },
    // OpenCode runs code after an edit only as a JavaScript or TypeScript
    // plugin, and there is no plain command to name, so it gets none.
    Agent {
        name: "opencode",
        detect: &[Detect {
            var: "OPENCODE",
            prefix: None,
        }],
        pointer: None,
        hook: None,
    },
];

/// The names `--agent` takes, in the table's order.
pub(crate) fn agent_names() -> Vec<&'static str> {
    AGENTS.iter().map(|agent| agent.name).collect()
}

/// The names `nvs agent hook` takes: the agents with a hook, in the table's
/// order.
pub(crate) fn hooked_agent_names() -> Vec<&'static str> {
    AGENTS
        .iter()
        .filter(|agent| agent.hook.is_some())
        .map(|agent| agent.name)
        .collect()
}

/// Which agents a run installs for, and the line that says how they were
/// chosen: the `--agent` names when there are any, every agent with `all`, and
/// otherwise the agents whose variables are set. `hooks` is whether the run
/// adds hooks, which decides whether an agent with one needs more than
/// `AGENTS.md`.
fn chosen(asked: &[String], all: bool, hooks: bool) -> (Vec<&'static Agent>, String) {
    let (agents, why): (Vec<&'static Agent>, Vec<String>) = if !asked.is_empty() {
        let agents: Vec<&'static Agent> = AGENTS
            .iter()
            .filter(|agent| asked.iter().any(|name| name == agent.name))
            .collect();
        let names: Vec<&str> = agents.iter().map(|agent| agent.name).collect();
        (agents, vec![format!("{} (--agent)", names.join(", "))])
    } else if all {
        (
            AGENTS.iter().collect(),
            vec!["every one (--all)".to_owned()],
        )
    } else {
        AGENTS
            .iter()
            .filter_map(|agent| {
                let var = agent.detected()?;
                Some((agent, format!("{} ({var})", agent.name)))
            })
            .unzip()
    };
    if agents.is_empty() {
        let line = "agent: none detected; pass --agent <name> to install an agent's own files";
        return (agents, line.to_owned());
    }
    let mut line = format!("agent: {}", why.join(", "));
    if agents
        .iter()
        .all(|agent| agent.pointer.is_none() && !(hooks && agent.hook.is_some()))
    {
        line.push_str(if agents.len() == 1 {
            "; AGENTS.md is all it needs"
        } else {
            "; AGENTS.md is all they need"
        });
    }
    (agents, line)
}

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
/// to update it to, a refusal unless `--force`, which carries the text
/// `--force` writes instead, or the removal of what `init` put there — the
/// file's text without it, or `None` to delete the file.
#[derive(Debug)]
enum Owed {
    Nothing,
    Missing(String),
    Outdated(String),
    Edited(String),
    Unwanted(Option<String>),
}

/// What kind of thing a planned change is about, which is what decides the
/// word printed beside its path.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    /// The stanza or a pointer this binary writes.
    Pointer,
    /// A pointer this binary no longer writes.
    Retired,
    /// An agent's check-on-edit hook, which is one entry in a file the
    /// project owns.
    Hook,
}

impl Kind {
    /// The word a run prints for `owed`, and with `check` the word `--check`
    /// prints. `None` when there is nothing to say.
    fn word(self, owed: &Owed, check: bool) -> Option<&'static str> {
        Some(match (self, owed, check) {
            (_, Owed::Nothing, _) => return None,
            (Kind::Hook, Owed::Missing(_), true) => "missing hook",
            (Kind::Hook, Owed::Missing(_), false) => "added hook to",
            (Kind::Hook, Owed::Unwanted(_), _) => "removed hook from",
            (_, Owed::Missing(_), true) => "missing",
            (_, Owed::Missing(_), false) => "wrote",
            (_, Owed::Outdated(_), true) => "outdated",
            (_, Owed::Edited(_), true) => "edited",
            (_, Owed::Outdated(_) | Owed::Edited(_), false) => "updated",
            (Kind::Retired, Owed::Unwanted(_), true) => "retired",
            (_, Owed::Unwanted(_), _) => "removed",
        })
    }
}

/// What `init` was asked to do.
#[derive(Debug)]
pub(crate) struct InitOptions {
    /// The `--agent` names, empty when none was given.
    pub(crate) agents: Vec<String>,
    pub(crate) all: bool,
    pub(crate) force: bool,
    pub(crate) check: bool,
    /// `--no-hooks`: add no hook, and remove the ones an earlier run added.
    pub(crate) no_hooks: bool,
}

/// What a run does with the check-on-edit hooks.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Hooks {
    /// Add the hook of each chosen agent that lacks it.
    Add,
    /// Remove every hook an earlier run added, whichever agent it was for.
    Remove,
    /// Read no hook file: `--check` with `--no-hooks`.
    Leave,
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

/// Everything one run owes the project under `root`, judged before anything
/// is written: the changes, the notes about files it leaves alone, and the
/// errors that refuse the run.
#[derive(Debug, Default)]
struct Plan {
    changes: Vec<(&'static str, Kind, Owed)>,
    notes: Vec<String>,
    errors: Vec<String>,
}

/// What a run owes for `agents`: the stanza always, the pointer of each of
/// those agents, every pointer an earlier run wrote whichever agent it was
/// for, the hooks as `hooks` says, and the removal of a retired pointer nobody
/// edited.
fn plan(root: &Path, agents: &[&'static Agent], hooks: Hooks) -> Plan {
    let mut plan = Plan::default();
    match stanza_owed(&root.join("AGENTS.md")) {
        Ok(owed) => plan.changes.push(("AGENTS.md", Kind::Pointer, owed)),
        Err(error) => plan.errors.push(error),
    }
    for adapter in AGENTS.iter().filter_map(|agent| agent.pointer) {
        let wanted = agents
            .iter()
            .any(|agent| agent.pointer.is_some_and(|it| std::ptr::eq(it, adapter)));
        match adapter_owed(root, adapter) {
            Ok(Owed::Missing(_)) if !wanted => {}
            Ok(owed) => plan.changes.push((adapter.path, Kind::Pointer, owed)),
            Err(error) => plan.errors.push(error),
        }
    }
    if hooks != Hooks::Leave {
        for agent in AGENTS {
            let Some(hook) = agent.hook else {
                continue;
            };
            let wanted = hooks == Hooks::Add && agents.iter().any(|it| std::ptr::eq(*it, agent));
            match hook_owed(root, hook, hooks, wanted) {
                Ok(owed) => plan.changes.push((hook.path, Kind::Hook, owed)),
                Err(error) => plan.errors.push(error),
            }
        }
    }
    for adapter in RETIRED {
        match adapter_owed(root, adapter) {
            Ok(Owed::Missing(_)) => {}
            Ok(Owed::Edited(_)) => plan.notes.push(format!(
                "note: {} is no longer written by `nvs agent init`, and it was edited, so it \
                 is left alone",
                adapter.path
            )),
            Ok(_) => {
                plan.changes
                    .push((adapter.path, Kind::Retired, Owed::Unwanted(None)));
            }
            Err(error) => plan.errors.push(error),
        }
    }
    plan
}

/// Install the surface into this project: the `AGENTS.md` stanza, and what
/// each chosen agent needs beside it ([`chosen`] says which).
///
/// A missing file is written, a file that still matches its fingerprint is
/// updated to this binary's text, and an edited one is refused unless `force`.
/// A file an earlier run wrote is kept current whichever agent runs now, so
/// every member of a team gets the same files. A hook is added for a chosen
/// agent that lacks it and is otherwise left where it is, and `no_hooks`
/// removes every hook instead. With `check` nothing is
/// written, and each file that is missing, outdated or edited is named
/// instead. Every file is read and judged before any is written, so a refusal
/// about one of them leaves all of them as they were.
pub(crate) fn init(options: &InitOptions) -> ExitCode {
    let root = match std::env::current_dir() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("error: this directory cannot be read: {error}");
            return ExitCode::FAILURE;
        }
    };
    let hooks = match (options.no_hooks, options.check) {
        (false, _) => Hooks::Add,
        (true, false) => Hooks::Remove,
        (true, true) => Hooks::Leave,
    };
    let (agents, line) = chosen(&options.agents, options.all, !options.no_hooks);
    println!("{line}");
    let plan = plan(&root, &agents, hooks);
    for error in &plan.errors {
        eprintln!("error: {error}");
    }

    if options.check {
        let mut clean = plan.errors.is_empty();
        for (path, kind, owed) in &plan.changes {
            if let Some(word) = kind.word(owed, true) {
                println!("{word} {path}");
                clean = false;
            }
        }
        if !clean {
            return ExitCode::FAILURE;
        }
        println!("up to date");
        return ExitCode::SUCCESS;
    }

    let mut refused = !plan.errors.is_empty();
    if !options.force {
        for (path, _, owed) in &plan.changes {
            if matches!(owed, Owed::Edited(_)) {
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
    for note in &plan.notes {
        eprintln!("{note}");
    }

    let mut wrote = false;
    for (path, kind, owed) in plan.changes {
        let Some(word) = kind.word(&owed, false) else {
            continue;
        };
        let done = match owed {
            Owed::Missing(text)
            | Owed::Outdated(text)
            | Owed::Edited(text)
            | Owed::Unwanted(Some(text)) => write_file(&root.join(path), &text),
            Owed::Unwanted(None) => std::fs::remove_file(root.join(path))
                .map_err(|error| format!("{path} cannot be removed: {error}")),
            Owed::Nothing => Ok(()),
        };
        if let Err(error) = done {
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

/// The one line `primer` puts on standard error when a re-run of `init` would
/// change something in the project in the working directory: a file an older
/// `init` wrote that nobody edited since, a retired pointer, or — once the
/// stanza is there — a file or a hook the agent running now needs and lacks. `None`
/// when it would change nothing, and for a project with no stanza.
pub(crate) fn stale_note() -> Option<&'static str> {
    let root = std::env::current_dir().ok()?;
    let (agents, _) = chosen(&[], false, true);
    let plan = plan(&root, &agents, Hooks::Add);
    let installed = plan
        .changes
        .iter()
        .any(|(path, _, owed)| *path == "AGENTS.md" && !matches!(owed, Owed::Missing(_)));
    let stale = plan.changes.iter().any(|(path, _, owed)| match owed {
        Owed::Nothing | Owed::Edited(_) => false,
        Owed::Missing(_) => installed && *path != "AGENTS.md",
        Owed::Outdated(_) | Owed::Unwanted(_) => true,
    });
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

/// What one agent's pointer owes: the file when it is missing, otherwise what
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

/// One agent's check-on-edit hook: the settings file it goes in, the event
/// under that file's `hooks` object whose array it joins, the file a new one
/// starts from, the entry, and the command that entry runs.
///
/// The entry is found again by its command alone, so a project may change its
/// matcher or move it within the array and a re-run still sees it as there.
#[derive(Debug)]
struct HookFile {
    path: &'static str,
    event: &'static str,
    /// The JSON a file this hook creates starts from. Its keys are also what
    /// is left of a file that held nothing else, which removing the hook
    /// deletes.
    skeleton: &'static str,
    /// The entry, as JSON.
    entry: &'static str,
    command: &'static str,
}

/// Claude Code's hook, in the project's shared settings. A command hook after
/// each tool that edits a file; the hook itself skips a file that is not
/// `.nvs`. `.claude/settings.local.json` is each person's own and is never
/// read or written.
const CLAUDE_HOOK: HookFile = HookFile {
    path: ".claude/settings.json",
    event: "PostToolUse",
    skeleton: "{}",
    entry: r#"{"matcher": "Write|Edit|MultiEdit", "hooks": [{"type": "command", "command": "nvs agent hook claude-code"}]}"#,
    command: "nvs agent hook claude-code",
};

/// Cursor's hook. Cursor reports every edit, whichever tool made it, as its
/// `Write` tool.
const CURSOR_HOOK: HookFile = HookFile {
    path: ".cursor/hooks.json",
    event: "postToolUse",
    skeleton: r#"{"version": 1}"#,
    entry: r#"{"command": "nvs agent hook cursor", "matcher": "Write"}"#,
    command: "nvs agent hook cursor",
};

/// A JSON document that keeps the order of its keys.
///
/// `serde_json::Map` sorts its keys in this workspace, and turning on its
/// `preserve_order` feature would reorder every other document the workspace
/// writes. A hook is merged into a file the project owns, so its keys and
/// entries are written back in the order they were found; only the
/// indentation is the one `serde_json::to_string_pretty` writes.
#[derive(Clone, Debug, PartialEq)]
enum Doc {
    Object(Vec<(String, Doc)>),
    Array(Vec<Doc>),
    /// A string, number, boolean or `null`.
    Scalar(Value),
}

impl Doc {
    /// `text` as a document. A byte order mark in front of it is skipped.
    fn parse(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text.strip_prefix('\u{feff}').unwrap_or(text))
    }

    /// One of this crate's own JSON constants as a document.
    fn constant(text: &'static str) -> Self {
        Self::parse(text).expect("the hook constants are valid JSON")
    }

    /// The value under `key`, when this is an object that has one.
    fn field(&self, key: &str) -> Option<&Doc> {
        match self {
            Doc::Object(pairs) => pairs.iter().find(|(name, _)| name == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The value under `key`, added at the end as `default` when this object
    /// has none. `None` when this is not an object.
    fn field_or(&mut self, key: &str, default: Doc) -> Option<&mut Doc> {
        let Doc::Object(pairs) = self else {
            return None;
        };
        let at = match pairs.iter().position(|(name, _)| name == key) {
            Some(at) => at,
            None => {
                pairs.push((key.to_owned(), default));
                pairs.len() - 1
            }
        };
        Some(&mut pairs[at].1)
    }

    /// Whether this is an object whose `command` is `command`.
    fn runs(&self, command: &str) -> bool {
        matches!(self.field("command"), Some(Doc::Scalar(Value::String(it))) if it == command)
    }

    /// The document as a file: two-space indentation and a final newline.
    fn text(&self) -> String {
        let mut out =
            serde_json::to_string_pretty(self).expect("a document of JSON values serializes");
        out.push('\n');
        out
    }
}

impl<'de> serde::Deserialize<'de> for Doc {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        /// Builds a [`Doc`] from whatever value the parser meets.
        struct Visit;

        impl<'de> serde::de::Visitor<'de> for Visit {
            type Value = Doc;

            fn expecting(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                out.write_str("a JSON value")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Doc, E> {
                Ok(Doc::Scalar(Value::Bool(value)))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Doc, E> {
                Ok(Doc::Scalar(Value::from(value)))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Doc, E> {
                Ok(Doc::Scalar(Value::from(value)))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Doc, E> {
                Ok(Doc::Scalar(Value::from(value)))
            }

            fn visit_str<E>(self, value: &str) -> Result<Doc, E> {
                Ok(Doc::Scalar(Value::from(value)))
            }

            fn visit_string<E>(self, value: String) -> Result<Doc, E> {
                Ok(Doc::Scalar(Value::String(value)))
            }

            fn visit_unit<E>(self) -> Result<Doc, E> {
                Ok(Doc::Scalar(Value::Null))
            }

            fn visit_none<E>(self) -> Result<Doc, E> {
                Ok(Doc::Scalar(Value::Null))
            }

            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Doc, A::Error> {
                let mut items = Vec::new();
                while let Some(item) = seq.next_element()? {
                    items.push(item);
                }
                Ok(Doc::Array(items))
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Doc, A::Error> {
                let mut pairs = Vec::new();
                while let Some(pair) = map.next_entry::<String, Doc>()? {
                    pairs.push(pair);
                }
                Ok(Doc::Object(pairs))
            }
        }

        deserializer.deserialize_any(Visit)
    }
}

impl serde::Serialize for Doc {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{SerializeMap as _, SerializeSeq as _};
        match self {
            Doc::Object(pairs) => {
                let mut map = serializer.serialize_map(Some(pairs.len()))?;
                for (key, value) in pairs {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
            Doc::Array(items) => {
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            Doc::Scalar(value) => value.serialize(serializer),
        }
    }
}

impl HookFile {
    /// The hook's file under `root`, read and held to the shape the agent's
    /// tool reads: an object, whose `hooks` is an object, whose event is an
    /// array. `None` when there is no file.
    ///
    /// Anything else is an error that refuses the run, and `--force` does not
    /// override it: the file holds the project's other settings, and a file
    /// this cannot read as JSON is one it cannot rewrite without losing them.
    fn read(&self, root: &Path) -> Result<Option<Doc>, String> {
        let path = self.path;
        let text = match std::fs::read_to_string(root.join(path)) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("{path} cannot be read: {error}")),
        };
        let refuse = |what: String| {
            format!(
                "{path} {what}, so its hook cannot be added or removed; fix the file and run \
                 this again (`--force` does not replace it, because it holds your other settings)"
            )
        };
        let doc =
            Doc::parse(&text).map_err(|error| refuse(format!("is not valid JSON ({error})")))?;
        if !matches!(doc, Doc::Object(_)) {
            return Err(refuse("is not a JSON object".to_owned()));
        }
        match doc.field("hooks") {
            None => {}
            Some(Doc::Object(_)) => match doc.field("hooks").and_then(|it| it.field(self.event)) {
                None | Some(Doc::Array(_)) => {}
                Some(_) => {
                    return Err(refuse(format!(
                        "has a `hooks.{}` that is not an array",
                        self.event
                    )));
                }
            },
            Some(_) => return Err(refuse("has a `hooks` that is not an object".to_owned())),
        }
        Ok(Some(doc))
    }

    /// Whether `doc`, as [`HookFile::read`] returned it, carries this hook:
    /// an entry of the event's array whose command is this one, or a group in
    /// it whose own `hooks` array holds one.
    fn carried(&self, doc: &Doc) -> bool {
        let Some(Doc::Array(entries)) = doc.field("hooks").and_then(|it| it.field(self.event))
        else {
            return false;
        };
        entries.iter().any(|entry| {
            entry.runs(self.command)
                || matches!(entry.field("hooks"), Some(Doc::Array(inner))
                    if inner.iter().any(|hook| hook.runs(self.command)))
        })
    }

    /// `doc` with this hook added at the end of the event's array.
    fn added(&self, mut doc: Doc) -> Doc {
        let hooks = doc
            .field_or("hooks", Doc::Object(Vec::new()))
            .expect("`read` held the file to an object");
        let entries = hooks
            .field_or(self.event, Doc::Array(Vec::new()))
            .expect("`read` held `hooks` to an object");
        if let Doc::Array(entries) = entries {
            entries.push(Doc::constant(self.entry));
        }
        doc
    }

    /// `doc` without this hook, or `None` when nothing but the skeleton's keys
    /// is left. A group that held only this hook goes with it, and so do an
    /// event array and a `hooks` object that removing it left empty.
    fn removed(&self, mut doc: Doc) -> Option<Doc> {
        let command = self.command;
        if let Doc::Object(pairs) = &mut doc {
            if let Some((_, Doc::Object(events))) = pairs.iter_mut().find(|(key, _)| key == "hooks")
            {
                if let Some((_, Doc::Array(entries))) =
                    events.iter_mut().find(|(key, _)| key == self.event)
                {
                    entries.retain_mut(|entry| {
                        if entry.runs(command) {
                            return false;
                        }
                        let Doc::Object(fields) = entry else {
                            return true;
                        };
                        let Some((_, Doc::Array(inner))) =
                            fields.iter_mut().find(|(key, _)| key == "hooks")
                        else {
                            return true;
                        };
                        let before = inner.len();
                        inner.retain(|hook| !hook.runs(command));
                        !(inner.is_empty() && before > 0)
                    });
                }
                events.retain(|(key, value)| key != self.event || value != &Doc::Array(Vec::new()));
            }
            pairs.retain(|(key, value)| key != "hooks" || value != &Doc::Object(Vec::new()));
            let skeleton = Doc::constant(self.skeleton);
            if pairs.iter().all(|(key, _)| skeleton.field(key).is_some()) {
                return None;
            }
        }
        Some(doc)
    }
}

/// What one hook owes. With [`Hooks::Add`], a `wanted` hook that is missing
/// is the file with it added, and a hook that is there is left. With
/// [`Hooks::Remove`], a hook that is there is removed.
fn hook_owed(root: &Path, hook: &HookFile, hooks: Hooks, wanted: bool) -> Result<Owed, String> {
    let found = hook.read(root)?;
    let carried = found.as_ref().is_some_and(|doc| hook.carried(doc));
    Ok(match (hooks, carried, found) {
        (Hooks::Add, false, found) if wanted => {
            let doc = found.unwrap_or_else(|| Doc::constant(hook.skeleton));
            Owed::Missing(hook.added(doc).text())
        }
        (Hooks::Remove, true, Some(doc)) => Owed::Unwanted(hook.removed(doc).map(|doc| doc.text())),
        _ => Owed::Nothing,
    })
}

/// How many errors a hook gives back. The rest are counted, and `nvs check`
/// prints them all.
const HOOK_ERRORS: usize = 5;

/// What a hook's check found: how many errors, and the first [`HOOK_ERRORS`]
/// rendered.
#[derive(Debug)]
struct Report {
    errors: usize,
    rendered: String,
    /// Whether a rendered error carries a code, which is when the line naming
    /// `nvs agent show` closes the report.
    coded: bool,
}

thread_local! {
    /// Where [`crate::Sink::Hook`] leaves the report of the check [`hook`]
    /// runs. The sink is a `Copy` value passed down through the front end, and
    /// this slot is the smallest way to get the text back out of it.
    static REPORT: RefCell<Option<Report>> = const { RefCell::new(None) };
}

/// [`crate::Sink::Hook`]'s rendering: the errors in `diags`, by position,
/// without colour, kept for [`hook`]. Warnings are left out: a hook speaks
/// after every edit, and a warning that stands would be repeated after each
/// one until the agent learned to pass over the report. `nvs check` shows
/// them.
pub(crate) fn keep_errors(diags: &mut Diagnostics, map: &SourceMap) {
    diags.sort_by_position();
    let errors: Vec<_> = diags
        .iter()
        .filter(|diagnostic| matches!(diagnostic.severity, Severity::Error | Severity::Bug))
        .collect();
    if errors.is_empty() {
        return;
    }
    let renderer = Renderer::new().with_color(false);
    let mut out = Vec::new();
    let mut coded = false;
    for diagnostic in errors.iter().take(HOOK_ERRORS) {
        coded |= diagnostic.code.is_some();
        renderer
            .render(diagnostic, map, &mut out)
            .expect("rendering to an in-memory buffer cannot fail");
    }
    let report = Report {
        errors: errors.len(),
        rendered: String::from_utf8_lossy(&out).into_owned(),
        coded,
    };
    REPORT.with(|slot| *slot.borrow_mut() = Some(report));
}

/// `nvs agent hook <agent>`: check the `.nvs` file an agent's tool just
/// edited, and give its errors back to the agent in the JSON that tool reads.
///
/// The payload is the tool's JSON on standard input. Whatever goes wrong
/// before the check — a payload that is not JSON, no path in it, a path that
/// is not a `.nvs` file — ends the hook silently and successfully, because a
/// hook must never stop the agent's own work. A clean file prints nothing.
///
/// The check is `nvs check <file>`'s, run in the payload's working directory,
/// so the configuration and the program that lends a class file its
/// `autoload` map are found as they would be there. It never writes a default
/// `nvs.toml`: a hook runs after every edit, and the project did not ask for
/// one.
pub(crate) fn hook(name: &str) -> ExitCode {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return ExitCode::SUCCESS;
    }
    // A shell that pipes text in may put a byte order mark in front of it.
    let Ok(payload) = serde_json::from_str::<Value>(input.trim_start_matches('\u{feff}')) else {
        return ExitCode::SUCCESS;
    };
    let Some(edited) = edited_path(&payload) else {
        return ExitCode::SUCCESS;
    };
    let bytes = edited.as_bytes();
    if bytes.len() < 4 || !bytes[bytes.len() - 4..].eq_ignore_ascii_case(b".nvs") {
        return ExitCode::SUCCESS;
    }

    // Claude Code and Cursor send `cwd`. Cursor's common fields also carry
    // `workspace_roots`, whose first root is the project when `cwd` is absent.
    let cwd = payload["cwd"]
        .as_str()
        .or_else(|| payload["workspace_roots"][0].as_str())
        .map(PathBuf::from)
        .filter(|dir| dir.is_dir());
    if let Some(dir) = &cwd
        && !runs_in(dir)
    {
        return hook_in(dir, name, &input);
    }
    // Cursor runs the hooks in `.claude/settings.json` too. When the project
    // also has Cursor's own hook, that one answers and this one stays quiet.
    if name == "claude-code" && under_cursor(&payload) && cursor_hooked() {
        return ExitCode::SUCCESS;
    }

    let mut path = PathBuf::from(edited);
    if let Some(dir) = &cwd
        && path.is_absolute()
        && let Ok(inside) = path.strip_prefix(dir)
    {
        path = inside.to_path_buf();
    }
    let shown = path.display().to_string();

    REPORT.with(|slot| slot.borrow_mut().take());
    let checked = crate::front_end_granted(
        &path,
        Some(&[]),
        false,
        crate::Sink::Hook,
        crate::config::Init::Never,
        Some(Path::new(".")),
    );
    if checked.is_ok() {
        return ExitCode::SUCCESS;
    }
    let text = match REPORT.with(|slot| slot.borrow_mut().take()) {
        Some(report) => report_text(&report, &shown),
        None => format!("`nvs check` could not check {shown}. Run `nvs check {shown}` to see why."),
    };
    // Claude Code shows a PostToolUse `reason` to the agent as feedback it
    // must act on, and the edit itself stays made. Cursor's postToolUse has no
    // decision, only context added to the conversation.
    let answer = if name == "cursor" {
        json!({ "additional_context": text })
    } else {
        json!({ "decision": "block", "reason": text })
    };
    println!("{answer}");
    ExitCode::SUCCESS
}

/// The path of the file the payload says was edited. Claude Code documents
/// `tool_input.file_path`. Which field of Cursor's `Write` tool input names the
/// file is not documented, so the names its tools are known to use are tried,
/// and a `tool_input` sent as a JSON string is read too (not checked against a
/// live Cursor session).
fn edited_path(payload: &Value) -> Option<String> {
    let parsed;
    let input = match &payload["tool_input"] {
        Value::String(text) => {
            parsed = serde_json::from_str::<Value>(text).unwrap_or(Value::Null);
            &parsed
        }
        other => other,
    };
    ["file_path", "path", "target_file", "filePath"]
        .iter()
        .find_map(|key| input[key].as_str())
        .or_else(|| payload["file_path"].as_str())
        .map(str::to_owned)
}

/// The variable [`hook_in`] sets on the copy of the hook it starts, so that
/// copy checks where it is and never starts another.
const HOOK_CHILD: &str = "NVS_AGENT_HOOK_CHILD";

/// Whether the hook already runs in `dir`, or is the copy [`hook_in`] started
/// there.
fn runs_in(dir: &Path) -> bool {
    if std::env::var_os(HOOK_CHILD).is_some() {
        return true;
    }
    match (
        std::env::current_dir().and_then(|here| here.canonicalize()),
        dir.canonicalize(),
    ) {
        (Ok(here), Ok(there)) => here == there,
        _ => false,
    }
}

/// Runs this hook again with `dir` as its working directory, hands it the same
/// `payload`, and lets it print to this process's output. The front end finds
/// the configuration and the program that lends the `autoload` map from the
/// working directory, and a second process is how the hook gets one without
/// changing this process's. The agent's tool usually starts the hook in the
/// payload's directory already, and then no copy is started.
fn hook_in(dir: &Path, name: &str, payload: &str) -> ExitCode {
    let Ok(exe) = std::env::current_exe() else {
        return ExitCode::SUCCESS;
    };
    let Ok(mut child) = std::process::Command::new(exe)
        .args(["agent", "hook", name])
        .current_dir(dir)
        .env(HOOK_CHILD, "1")
        .stdin(std::process::Stdio::piped())
        .spawn()
    else {
        return ExitCode::SUCCESS;
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = std::io::Write::write_all(&mut stdin, payload.as_bytes());
    }
    let _ = child.wait();
    ExitCode::SUCCESS
}

/// The text a hook gives back for `report` about the file `shown`.
fn report_text(report: &Report, shown: &str) -> String {
    let plural = |count: usize| if count == 1 { "error" } else { "errors" };
    let mut text = format!(
        "`nvs check` found {} {} in {shown}:\n\n{}\n",
        report.errors,
        plural(report.errors),
        report.rendered.trim_end()
    );
    if report.errors > HOOK_ERRORS {
        let more = report.errors - HOOK_ERRORS;
        text.push_str(&format!(
            "\nand {more} more {}; run `nvs check {shown}` to see them all\n",
            plural(more)
        ));
    }
    if report.coded {
        text.push('\n');
        text.push_str(nvs_diagnostics::AGENT_SHOW_LINE);
        text.push('\n');
    }
    text
}

/// Whether the hook runs under Cursor: its payload carries `cursor_version`,
/// or one of the variables Cursor sets is. Cursor's hooks page documents the
/// field and `CURSOR_PROJECT_DIR` and `CURSOR_VERSION`; its agent terminal page
/// documents `CURSOR_AGENT`.
fn under_cursor(payload: &Value) -> bool {
    payload.get("cursor_version").is_some()
        || ["CURSOR_PROJECT_DIR", "CURSOR_VERSION", "CURSOR_AGENT"]
            .iter()
            .any(|var| std::env::var_os(var).is_some_and(|value| !value.is_empty()))
}

/// Whether the project in the working directory carries Cursor's own hook.
fn cursor_hooked() -> bool {
    matches!(CURSOR_HOOK.read(Path::new(".")), Ok(Some(doc)) if CURSOR_HOOK.carried(&doc))
}
