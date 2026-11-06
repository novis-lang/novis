//! The known-bug catalogue, `tests/data/markdown/known-bugs.toml`, held as the
//! source of truth for what `Core\Markdown` must not repeat.
//!
//! Each entry is a bug a major Markdown library shipped: the library, the
//! public advisory, issue or test file that records it, the class of the bug,
//! a minimal input and what the right result is. This file reads the catalogue
//! three ways. The first two hold its shape: every library the `markdown` goal
//! names has entries, and every entry is complete. The third renders each
//! entry and fails naming every entry whose result is not the expected one;
//! it is ignored until `Core\Markdown::toHtml` exists.
//!
//! An entry's input is `input`, then `repeat` written `n` times, then
//! `suffix`. A `work` or `output` expectation is a bound over `n`, so it needs
//! a `repeat`; every other expectation renders the entry once at `n = 1`.

use std::collections::BTreeSet;

use serde::Deserialize;

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
///
/// `Core\Markdown` does not exist yet, so every entry is unrendered. Stage 3
/// of the `markdown` goal writes this body and removes the `#[ignore]` below.
fn outcome(_bug: &Bug) -> Result<(), String> {
    Err("not rendered: `Core\\Markdown::toHtml` does not exist yet".to_owned())
}

#[test]
#[ignore = "stage 3 of the markdown goal builds Core\\Markdown::toHtml"]
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
