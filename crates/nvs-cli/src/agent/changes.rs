//! `nvs agent changes`: the sections of the changelog this binary carries.
//!
//! `rule:tooling/nvs-agent-changes-prints-the-changelog` is the command. The
//! file is [`CHANGELOG`], `CHANGELOG.md` embedded whole, and a section is a
//! `## [MAJOR.MINOR.PATCH]` heading and the lines under it up to the next one,
//! as `bun nv release` writes it. The text above the first section is the
//! file's own header and is no version's, so it is never printed.
//!
//! The release workflow prepends the version's section before the build
//! compiles, so a released binary carries its own section. A build from a
//! clone carries what the tree has, which is every released section and none
//! for the version it is.

use std::process::ExitCode;

use serde_json::{Value, json};

use super::{SCHEMA_VERSION, pretty};

/// `CHANGELOG.md`, whole. It spends the file's size once per binary, a few KiB
/// today, and nothing per request: only this command reads it.
const CHANGELOG: &str = include_str!("../../../../CHANGELOG.md");

/// The version this binary is, which a section is compared against.
const RUNNING: &str = env!("CARGO_PKG_VERSION");

/// A release version, `MAJOR.MINOR.PATCH`, ordered by its numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Version(u64, u64, u64);

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// `--since`'s parser: three whole numbers joined by `.`, with an optional
/// leading `v` as a tag writes it. Anything else is a usage error, which clap
/// reports with exit status 2.
pub(crate) fn parse_version(text: &str) -> Result<Version, String> {
    let core = text.strip_prefix('v').unwrap_or(text);
    let parts: Vec<&str> = core.split('.').collect();
    let number = |part: &str| -> Option<u64> {
        (!part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
            .then(|| part.parse().ok())
            .flatten()
    };
    match parts.as_slice() {
        [major, minor, patch] => match (number(major), number(minor), number(patch)) {
            (Some(major), Some(minor), Some(patch)) => Ok(Version(major, minor, patch)),
            _ => Err(format!("`{text}` is not a version. Write it as `0.2.0`.")),
        },
        _ => Err(format!("`{text}` is not a version. Write it as `0.2.0`.")),
    }
}

/// One `## [version]` section: its version, the date its heading names, and
/// its text as the file has it, heading included.
#[derive(Debug)]
struct Section<'a> {
    version: Version,
    date: Option<&'a str>,
    text: &'a str,
}

/// Every section of `changelog`, in the file's order, which is newest first.
/// A `## [` heading whose version does not parse is not a section, and its
/// lines stay with the section above it.
fn sections(changelog: &str) -> Vec<Section<'_>> {
    let mut starts: Vec<(usize, Version, Option<&str>)> = Vec::new();
    let mut offset = 0;
    for line in changelog.split_inclusive('\n') {
        let heading = line.trim_end();
        if let Some(rest) = heading.strip_prefix("## [")
            && let Some((version, after)) = rest.split_once(']')
            && let Ok(version) = parse_version(version)
        {
            let date = after
                .trim()
                .strip_prefix('-')
                .map(str::trim)
                .filter(|date| !date.is_empty());
            starts.push((offset, version, date));
        }
        offset += line.len();
    }
    let ends = starts
        .iter()
        .skip(1)
        .map(|(start, ..)| *start)
        .chain([changelog.len()]);
    starts
        .iter()
        .zip(ends)
        .map(|(&(start, version, date), end)| Section {
            version,
            date,
            text: changelog[start..end].trim_end(),
        })
        .collect()
}

/// What `nvs agent changes` prints for `since` over `changelog`, from a binary
/// that is `running`: the document, and a note for standard error when the
/// answer is empty.
fn answer(
    changelog: &str,
    running: Version,
    since: Option<Version>,
    json: bool,
) -> (String, Option<String>) {
    let all = sections(changelog);
    let chosen: Vec<&Section<'_>> = match since {
        Some(since) => all
            .iter()
            .filter(|section| section.version > since)
            .collect(),
        None => all
            .iter()
            .filter(|section| section.version == running)
            .collect(),
    };
    let note = match since {
        Some(since) if since > running => Some(format!(
            "This binary is version {running}, which is older than {since}. Install a newer `nvs` to read what changed after {running}."
        )),
        Some(since) if chosen.is_empty() => Some(format!(
            "Nothing changed after {since} in the changelog of this binary. This binary is version {running}."
        )),
        None if chosen.is_empty() => Some(format!(
            "The changelog of this binary has no section for version {running}. This build is not a release."
        )),
        _ => None,
    };
    if json {
        let records: Vec<Value> = chosen
            .iter()
            .map(|section| {
                json!({
                    "version": section.version.to_string(),
                    "date": section.date,
                    "text": section.text,
                })
            })
            .collect();
        let document = json!({
            "schemaVersion": SCHEMA_VERSION,
            "version": running.to_string(),
            "since": since.map(|since| since.to_string()),
            "sections": records,
        });
        return (pretty(document), note);
    }
    let texts: Vec<&str> = chosen.iter().map(|section| section.text).collect();
    let mut out = texts.join("\n\n");
    if !out.is_empty() {
        out.push('\n');
    }
    (out, note)
}

/// `nvs agent changes`: the sections newer than `since`, or this version's own.
/// It always succeeds; an empty answer says why on standard error.
pub(crate) fn changes(since: Option<Version>, json: bool) -> ExitCode {
    let running = parse_version(RUNNING).expect("the workspace version is MAJOR.MINOR.PATCH");
    let (out, note) = answer(CHANGELOG, running, since, json);
    if let Some(note) = note {
        eprintln!("{note}");
    }
    print!("{out}");
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::{Version, answer, parse_version, sections};

    /// A changelog with its header and three sections, newest first.
    const THREE: &str = "\
# Changelog

The header, which is no version's.

## [0.3.0] - 2026-03-01

### Features

- three

## [0.2.0] - 2026-02-01

- two

## [0.1.0] - 2026-01-01

- one
";

    fn printed(running: Version, since: Option<&str>) -> (String, Option<String>) {
        let since = since.map(|since| parse_version(since).expect("a version"));
        answer(THREE, running, since, false)
    }

    fn versions(out: &str) -> Vec<&str> {
        out.lines()
            .filter_map(|line| line.strip_prefix("## ["))
            .map(|rest| rest.split(']').next().unwrap_or(rest))
            .collect()
    }

    #[test]
    fn the_sections_are_read_newest_first_without_the_header() {
        let found = sections(THREE);
        let names: Vec<String> = found.iter().map(|s| s.version.to_string()).collect();
        assert_eq!(names, ["0.3.0", "0.2.0", "0.1.0"]);
        assert_eq!(found[0].date, Some("2026-03-01"));
        assert!(found[0].text.starts_with("## [0.3.0]") && found[0].text.ends_with("- three"));
        assert!(found.iter().all(|s| !s.text.contains("header")));
    }

    #[test]
    fn since_each_version_prints_every_newer_section_newest_first() {
        let running = Version(0, 3, 0);
        assert_eq!(versions(&printed(running, Some("0.2.0")).0), ["0.3.0"]);
        assert_eq!(
            versions(&printed(running, Some("0.1.0")).0),
            ["0.3.0", "0.2.0"]
        );
        let (out, note) = printed(running, Some("0.3.0"));
        assert_eq!(out, "");
        assert!(note.is_some_and(|note| note.contains("Nothing changed after 0.3.0")));
    }

    #[test]
    fn a_version_older_than_the_oldest_section_prints_all_of_them() {
        let (out, note) = printed(Version(0, 3, 0), Some("v0.0.1"));
        assert_eq!(versions(&out), ["0.3.0", "0.2.0", "0.1.0"]);
        assert_eq!(note, None);
    }

    #[test]
    fn a_version_newer_than_the_binary_prints_nothing_and_names_the_binary() {
        let (out, note) = printed(Version(0, 3, 0), Some("0.4.0"));
        assert_eq!(out, "");
        assert!(note.is_some_and(|note| note.contains("version 0.3.0")));
    }

    #[test]
    fn no_since_prints_the_running_version_s_own_section() {
        let (out, note) = printed(Version(0, 2, 0), None);
        assert_eq!(versions(&out), ["0.2.0"]);
        assert_eq!(note, None);
        let (out, note) = printed(Version(0, 4, 0), None);
        assert_eq!(out, "");
        assert!(note.is_some_and(|note| note.contains("no section for version 0.4.0")));
    }

    #[test]
    fn a_malformed_version_is_not_parsed() {
        for bad in [
            "",
            "1",
            "0.2",
            "0.2.0.1",
            "0.x.0",
            "0.2.0-rc1",
            "v",
            "+1.2.3",
        ] {
            assert!(parse_version(bad).is_err(), "{bad:?} is not a version");
        }
        assert_eq!(parse_version("v1.2.3"), Ok(Version(1, 2, 3)));
    }

    #[test]
    fn json_prints_the_same_sections_with_the_schema_version() {
        let (out, _) = answer(THREE, Version(0, 3, 0), Some(Version(0, 1, 0)), true);
        let document: serde_json::Value = serde_json::from_str(&out).expect("the document is JSON");
        assert_eq!(document["schemaVersion"], 1);
        assert_eq!(document["version"], "0.3.0");
        assert_eq!(document["since"], "0.1.0");
        let sections = document["sections"].as_array().expect("sections");
        let names: Vec<&str> = sections
            .iter()
            .filter_map(|s| s["version"].as_str())
            .collect();
        assert_eq!(names, ["0.3.0", "0.2.0"]);
        assert_eq!(sections[1]["date"], "2026-02-01");
        assert!(
            sections[1]["text"]
                .as_str()
                .is_some_and(|t| t.ends_with("- two"))
        );

        let (out, _) = answer(THREE, Version(0, 4, 0), None, true);
        let document: serde_json::Value = serde_json::from_str(&out).expect("the document is JSON");
        assert_eq!(document["since"], serde_json::Value::Null);
        assert_eq!(document["sections"], serde_json::json!([]));
    }
}
