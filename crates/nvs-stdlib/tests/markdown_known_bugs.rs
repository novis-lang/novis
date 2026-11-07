//! The known-bug catalogue, `tests/data/markdown/known-bugs.toml`, held as the
//! source of truth for what `Core\Markdown` must not repeat.
//!
//! Each entry is a bug a major Markdown library shipped: the library, the
//! public advisory, issue or test file that records it, the class of the bug,
//! a minimal input and what the right result is. This file reads the catalogue
//! three ways. The first two hold its shape: every library the `markdown` goal
//! names has entries, and every entry is complete. The third renders each
//! entry and fails naming every entry whose result is not the expected one.
//!
//! An entry's input is `input`, then `repeat` written `n` times, then
//! `suffix`. A `work` or `output` expectation is a bound over `n`, so it needs
//! a `repeat`; every other expectation renders the entry once at `n = 1`.
//!
//! How each expectation is checked:
//!
//! - **`html`** compares both sides after the sanitizer's parse and rebuild,
//!   so a quote written as `&quot;` and one written as `"` are the same.
//! - **`html_lacks`** searches only the HTML's tags
//!   ([`nvs_stdlib::fuzz::html_tags`]). Every such string names markup: a
//!   tag, an attribute or a URL scheme. The same characters in text are
//!   escaped where they must be and harmless, and the serializer writes `"`
//!   as it is there. `[x](vbscript:alert(1&#41;)` is not a link, so its
//!   `vbscript:` is text.
//! - **An md4c `html`** is written as cmark writes it. md4c's own tests
//!   compare after collapsing whitespace, so a line feed there may be a
//!   space here.
//! - **`output = "bounded"`** renders at `n` and at `4n`. The longer output
//!   is at most four times the shorter plus the budget, and at most
//!   [`MARKUP_PER_BYTE`] bytes per input byte plus the budget.
//! - **`work = "linear"`** is rendered here once, and its bound is
//!   `work_grows_linearly_over_every_catalogued_slow_input` in the crate's
//!   own tests, which can count the engine's steps.
//! - **A `from-html` entry** runs `Core\Markdown::fromHtml`. Its `markdown`
//!   is the exact Markdown and its `markdown_lacks` strings never appear in
//!   it. Its `html` is what that Markdown renders back to, compared after
//!   both are sanitized, so a legitimate escaping style does not fail it. Its
//!   `work` bound is
//!   `from_html_work_grows_linearly_over_every_catalogued_slow_input` in the
//!   crate's own tests.

use std::collections::BTreeSet;

use nvs_stdlib::fuzz::{from_html, html_sanitized, html_tags, markdown};
use serde::Deserialize;

/// The `n` a bound over `repeat` renders at, and then at four times it.
const N: usize = 1000;

/// The most HTML one input byte may write, past the budget. `>` is one byte
/// and opens a `<blockquote>` with its line feed and its end tag.
const MARKUP_PER_BYTE: usize = 32;

/// `rule:core-classes/markdown-cost`'s budget for added output, 64 KiB.
const ADDED_OUTPUT_SLACK: usize = 64 << 10;

/// The libraries whose shipped bugs the catalogue covers, as an entry spells
/// its `library`.
const LIBRARIES: &[&str] = &[
    "cmark",
    "cmark-gfm",
    "commonmark.js",
    "markdown-it",
    "marked",
    "pulldown-cmark",
    "comrak",
    "md4c",
    "league/commonmark",
    "Parsedown",
    "Python-Markdown",
    "mistune",
    "Turndown",
    "html-to-markdown",
];

/// The classes an entry may name, closed so a sweep over one class finds it.
const CLASSES: &[&str] = &[
    "slow",
    "output-blowup",
    "deep-nesting",
    "crash",
    "xss",
    "url-scheme",
    "attribute-injection",
    "wrong-output",
    "markdown-injection",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalogue {
    bug: Vec<Bug>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bug {
    id: String,
    library: String,
    source: String,
    class: String,
    direction: String,
    input: String,
    repeat: Option<String>,
    suffix: Option<String>,
    html: Option<String>,
    html_lacks: Option<Vec<String>>,
    markdown: Option<String>,
    markdown_lacks: Option<Vec<String>>,
    work: Option<String>,
    output: Option<String>,
}

impl Bug {
    /// The entry's input with `repeat` written `n` times.
    fn text(&self, n: usize) -> String {
        let repeat = self.repeat.as_deref().unwrap_or("");
        let suffix = self.suffix.as_deref().unwrap_or("");
        let mut text = String::with_capacity(self.input.len() + repeat.len() * n + suffix.len());
        text.push_str(&self.input);
        for _ in 0..n {
            text.push_str(repeat);
        }
        text.push_str(suffix);
        text
    }

    /// The names of the expectations this entry sets.
    fn expectations(&self) -> Vec<&'static str> {
        [
            ("html", self.html.is_some()),
            ("html_lacks", self.html_lacks.is_some()),
            ("markdown", self.markdown.is_some()),
            ("markdown_lacks", self.markdown_lacks.is_some()),
            ("work", self.work.is_some()),
            ("output", self.output.is_some()),
        ]
        .into_iter()
        .filter_map(|(name, set)| set.then_some(name))
        .collect()
    }

    /// Every way this entry breaks the catalogue's shape, empty when it is
    /// complete.
    fn faults(&self) -> Vec<String> {
        let mut faults = Vec::new();
        let slug_ok = !self.id.is_empty()
            && self
                .id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        if !slug_ok {
            faults.push("`id` is not a lowercase slug".to_owned());
        }
        if !LIBRARIES.contains(&self.library.as_str()) {
            faults.push(format!(
                "`library` {:?} is not one the goal lists",
                self.library
            ));
        }
        if !self.source.starts_with("https://") || self.source.contains(char::is_whitespace) {
            faults.push("`source` is not an https URL".to_owned());
        }
        if !CLASSES.contains(&self.class.as_str()) {
            faults.push(format!(
                "`class` {:?} is not in the closed list",
                self.class
            ));
        }
        let to_html = match self.direction.as_str() {
            "to-html" => true,
            "from-html" => false,
            other => {
                faults.push(format!("`direction` {other:?} is not to-html or from-html"));
                true
            }
        };
        let repeated = self.repeat.as_deref().is_some_and(|unit| !unit.is_empty());
        if self.input.is_empty() && !repeated {
            faults.push("neither `input` nor `repeat` has text".to_owned());
        }
        if self.suffix.is_some() && !repeated {
            faults.push("`suffix` without `repeat`".to_owned());
        }
        let expectations = self.expectations();
        let [expectation] = expectations[..] else {
            faults.push(format!(
                "sets {} expectations ({}), not exactly one",
                expectations.len(),
                expectations.join(", ")
            ));
            return faults;
        };
        match expectation {
            "work" | "output" if !repeated => {
                faults.push(format!(
                    "`{expectation}` is a bound over `repeat`, which is missing"
                ));
            }
            "work" if self.work.as_deref() != Some("linear") => {
                faults.push("`work` is not \"linear\"".to_owned());
            }
            "output" if self.output.as_deref() != Some("bounded") => {
                faults.push("`output` is not \"bounded\"".to_owned());
            }
            "html_lacks" if !to_html => {
                faults.push("`html_lacks` on a from-html entry".to_owned());
            }
            "markdown" | "markdown_lacks" if to_html => {
                faults.push(format!("`{expectation}` on a to-html entry"));
            }
            _ => {}
        }
        let lacks = self.html_lacks.iter().chain(&self.markdown_lacks);
        if lacks.flatten().any(String::is_empty) {
            faults.push("an empty substring in a `_lacks` list".to_owned());
        }
        faults
    }
}

fn catalogue() -> Catalogue {
    let path = nvs_repo::path("tests/data/markdown/known-bugs.toml");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("reading {}: {err}", path.display()));
    toml::from_str(&text).unwrap_or_else(|err| panic!("parsing {}: {err}", path.display()))
}

#[test]
fn the_catalogue_covers_every_library_the_goal_lists() {
    let catalogue = catalogue();
    let named: BTreeSet<&str> = catalogue
        .bug
        .iter()
        .map(|bug| bug.library.as_str())
        .collect();
    let missing: Vec<&str> = LIBRARIES
        .iter()
        .copied()
        .filter(|library| !named.contains(library))
        .collect();
    assert!(
        missing.is_empty(),
        "the catalogue has no entry for: {missing:?}"
    );
}

#[test]
fn every_catalogue_entry_has_a_source_a_class_an_input_and_an_expectation() {
    let catalogue = catalogue();
    let mut seen = BTreeSet::new();
    let mut report = Vec::new();
    for bug in &catalogue.bug {
        let mut faults = bug.faults();
        if !seen.insert(bug.id.as_str()) {
            faults.push("`id` is used by an earlier entry".to_owned());
        }
        report.extend(
            faults
                .into_iter()
                .map(|fault| format!("{}: {fault}", bug.id)),
        );
    }
    assert!(
        report.is_empty(),
        "{} fault(s):\n{}",
        report.len(),
        report.join("\n")
    );
}

/// Whether `bug` renders as the catalogue expects, and what went wrong if not.
fn outcome(bug: &Bug) -> Result<(), String> {
    if bug.direction == "from-html" {
        return from_html_outcome(bug);
    }
    if bug.output.is_some() {
        let (short, long) = (bug.text(N), bug.text(4 * N));
        let (small, large) = (markdown(&short).len(), markdown(&long).len());
        if large > 4 * small + ADDED_OUTPUT_SLACK {
            return Err(format!(
                "the output grows faster than the input: {small} bytes at n = {N}, {large} at 4n"
            ));
        }
        if large > MARKUP_PER_BYTE * long.len() + ADDED_OUTPUT_SLACK {
            return Err(format!(
                "{large} bytes of output for {} bytes of input",
                long.len()
            ));
        }
        return Ok(());
    }
    let html = markdown(&bug.text(1));
    if let Some(want) = &bug.html {
        let (got, want) = (html_sanitized(&html), html_sanitized(want));
        if got != want {
            return Err(format!("renders {got:?}, want {want:?}"));
        }
    }
    let tags = html_tags(&html);
    for lack in bug.html_lacks.iter().flatten() {
        if tags.contains(lack.as_str()) {
            return Err(format!("{lack:?} is in the tags of {html:?}"));
        }
    }
    Ok(())
}

/// [`outcome`] for an entry whose input is HTML: the Markdown
/// `Core\Markdown::fromHtml` writes, and the HTML that Markdown renders back
/// to.
fn from_html_outcome(bug: &Bug) -> Result<(), String> {
    if bug.work.is_some() {
        let _ = from_html(&bug.text(N));
        return Ok(());
    }
    let written = from_html(&bug.text(1));
    for lack in bug.markdown_lacks.iter().flatten() {
        if written.contains(lack.as_str()) {
            return Err(format!("{lack:?} is in {written:?}"));
        }
    }
    if let Some(want) = &bug.markdown
        && written != *want
    {
        return Err(format!("writes {written:?}, want {want:?}"));
    }
    if let Some(want) = &bug.html {
        let (got, want) = (html_sanitized(&markdown(&written)), html_sanitized(want));
        if got != want {
            return Err(format!("{written:?} renders {got:?}, want {want:?}"));
        }
    }
    Ok(())
}

#[test]
fn every_known_bug_renders_as_the_catalogue_expects() {
    let catalogue = catalogue();
    let report: Vec<String> = catalogue
        .bug
        .iter()
        .filter_map(|bug| {
            outcome(bug)
                .err()
                .map(|why| format!("{} ({}): {why}", bug.id, bug.source))
        })
        .collect();
    assert!(
        report.is_empty(),
        "{} entries fail:\n{}",
        report.len(),
        report.join("\n")
    );
}
