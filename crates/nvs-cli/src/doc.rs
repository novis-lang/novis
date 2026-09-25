//! `nvs doc <entry>` — one Markdown page per class, rendered from
//! `nvs meta --json`'s document and from nothing else.
//!
//! `rule:tooling/nvs-doc-renders-and-decides-nothing` is the whole design, and
//! this module is deliberately the least interesting part of it: it reads
//! [`crate::meta::program_document`]'s value — the same document a consumer
//! outside the binary reads — and turns it into text. It resolves no name,
//! reaches no source file and asks nothing of the compiler, so a page can never
//! say something `nvs meta --json` did not, and replacing it later costs
//! nothing.
//!
//! It exists for a project that does not have this repository's
//! `bun nv reference` or the website's Core reference, which are the other two
//! renderers on that one document (`rule:tooling/one-json-several-renderers`).
//!
//! **A page per class, an interface and an enum**, named for the declaration it
//! documents with `\` written as `.`, so a namespaced class lands in one flat
//! directory rather than a tree that has to be walked to be read. A `type`
//! alias gets no page: it is one line, and a line is not a page.
//!
//! **Text is rendered, never escaped.** A doc comment's prose is Markdown by
//! `rule:tooling/doc-comment-tags-are-see-and-example`, so it is written out as
//! it was authored, and the lexer has already refused an unterminated
//! directional control anywhere in the file
//! (`rule:security/bidi-boundaries`) — this renderer grows no check of its own.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde_json::Value;

/// Writes one page per class under `out`, and prints each path written.
///
/// The program goes through the same front end `nvs check` does, so a program
/// that does not check writes nothing at all — a half-rendered directory of
/// pages describing a program that does not compile is worse than none.
pub(crate) fn run(entry: &Path, out: &Path) -> ExitCode {
    let document = match crate::meta::program_document(entry) {
        Ok(document) => document,
        Err(code) => return code,
    };
    let program = &document["program"];
    let pages = pages(program);
    if pages.is_empty() {
        eprintln!("no class, interface or enum in this program: nothing to render");
        return ExitCode::SUCCESS;
    }
    if let Err(err) = std::fs::create_dir_all(out) {
        eprintln!("error: could not create {}: {err}", out.display());
        return ExitCode::FAILURE;
    }
    let names: Vec<&str> = pages.iter().map(|page| page.name).collect();
    for page in &pages {
        let path = out.join(file_name(page.name));
        if let Err(err) = std::fs::write(&path, render(page, &names)) {
            eprintln!("error: could not write {}: {err}", path.display());
            return ExitCode::FAILURE;
        }
        println!("{}", path.display());
    }
    ExitCode::SUCCESS
}

/// One declaration that gets a page: its roster's own word for what it is, and
/// the document's object for it.
struct Page<'a> {
    name: &'a str,
    kind: &'a str,
    value: &'a Value,
}

/// Every page the program half asks for, in the document's own order — which is
/// load order, so two runs over one program render the same directory.
fn pages(program: &Value) -> Vec<Page<'_>> {
    let mut out = Vec::new();
    for (roster, kind) in [
        ("classes", "class"),
        ("interfaces", "interface"),
        ("enums", "enum"),
    ] {
        let Some(values) = program[roster].as_array() else {
            continue;
        };
        for value in values {
            if let Some(name) = value["name"].as_str() {
                out.push(Page { name, kind, value });
            }
        }
    }
    out
}

/// `Shop\Money` as `Shop.Money.md`. A backslash is a path separator on the one
/// platform that would read it as one, so a namespace becomes a dotted name
/// rather than a directory nobody asked for.
fn file_name(name: &str) -> PathBuf {
    PathBuf::from(format!("{}.md", name.replace('\\', ".")))
}

/// One page: the declaration's own card, then a section per roster it carries.
fn render(page: &Page<'_>, names: &[&str]) -> String {
    let mut out = format!("# {}\n\n*{}*\n", page.name, page.kind);
    card(&mut out, &page.value["doc"], page.name, names);
    section(&mut out, page, "cases", "Cases", names);
    section(&mut out, page, "constants", "Constants", names);
    section(&mut out, page, "members", "Members", names);
    out
}

/// One roster as a `##` section, or nothing when the document omitted it —
/// which is what an empty roster is, per `rule:tooling/meta-json`'s omission
/// rule.
fn section(out: &mut String, page: &Page<'_>, key: &str, title: &str, names: &[&str]) {
    let Some(entries) = page.value[key].as_array() else {
        return;
    };
    let shown: Vec<&Value> = entries.iter().filter(|entry| reachable(entry)).collect();
    if shown.is_empty() {
        return;
    }
    out.push_str(&format!("\n## {title}\n"));
    for entry in shown {
        let name = entry["name"].as_str().unwrap_or_default();
        out.push_str(&format!("\n### {name}\n"));
        if let Some(signature) = entry["signature"].as_str() {
            out.push_str(&format!("\n```nvs\n{signature}\n```\n"));
        } else if let Some(value) = entry["value"].as_str() {
            let ty = entry["type"]
                .as_str()
                .map_or(String::new(), |ty| format!("{ty} "));
            out.push_str(&format!("\n```nvs\n{ty}{name} = {value}\n```\n"));
        }
        card(out, &entry["doc"], page.name, names);
    }
}

/// Whether a reader of this package can reach the entry — the document's own
/// `visibility` key, which a `private` or `protected` member carries and an
/// enum case, having no visibility, does not.
///
/// **A page is the surface**, which is the one thing this renderer decides that
/// the document does not say outright: a helper nobody outside the class can
/// call is not documentation, and `rule:tooling/strict-docs` reports the same
/// set from the other side rather than a different one.
fn reachable(entry: &Value) -> bool {
    entry["visibility"]
        .as_str()
        .is_none_or(|seen| seen == "public")
}

/// A card: its prose as it was written, then each tag as its own line.
///
/// `@see` renders as a link when its target has a page in this run and as code
/// when it does not — a `Core` member's page is the reference this renderer does
/// not produce, and a link to a file that was never written is worse than the
/// name.
fn card(out: &mut String, doc: &Value, owner: &str, names: &[&str]) {
    if let Some(short) = doc["short"].as_str() {
        out.push_str(&format!("\n{short}\n"));
    }
    if let Some(targets) = doc["see"].as_array() {
        let rendered: Vec<String> = targets
            .iter()
            .filter_map(|target| target.as_str())
            .map(|target| see(target, owner, names))
            .collect();
        out.push_str(&format!("\nSee also: {}\n", rendered.join(", ")));
    }
    if let Some(files) = doc["example"].as_array() {
        let rendered: Vec<String> = files
            .iter()
            .filter_map(|file| file.as_str())
            .map(|file| format!("`{file}`"))
            .collect();
        out.push_str(&format!("\nExample: {}\n", rendered.join(", ")));
    }
}

/// One `@see` target as a link into this run's own pages, or as code.
///
/// `self` and `static` name the page they were written on, exactly as they name
/// a class in code. `parent` does not: the document carries no `extends` edge,
/// and guessing one here would be this renderer deciding something.
fn see(target: &str, owner: &str, names: &[&str]) -> String {
    let (class, member) = match target.split_once("::") {
        Some((class, member)) => (class.trim(), Some(member.trim())),
        None => (target.trim(), None),
    };
    let class = match class {
        "self" | "static" => owner,
        other => other,
    };
    if !names.contains(&class) {
        return format!("`{target}`");
    }
    let file = file_name(class);
    let file = file.display();
    match member {
        Some(member) => format!("[{target}]({file}#{})", anchor(member)),
        None => format!("[{target}]({file})"),
    }
}

/// A `###` heading's own anchor, in the spelling every Markdown renderer agrees
/// on: lower case, a space as a hyphen, and nothing else kept. `$text`'s
/// heading is `### $text`, so its anchor is `text` — which is why the sigil and
/// a trailing `()` come off here rather than at the call site.
fn anchor(member: &str) -> String {
    member
        .trim_end_matches("()")
        .trim_start_matches('$')
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_ascii_alphanumeric() || c == '-' || c == '_' => Some(c.to_ascii_lowercase()),
            _ => None,
        })
        .collect()
}
