//! What a route's hover says about the program's grants
//! (`rule:ide/a-route-hover-shows-the-programs-grants`).
//!
//! **The grants are the program's, read off the snapshot the runtime enforces.** The tree is
//! [`crate::document::program_tree`]'s, the one the extension set reads, and every line below is
//! a [`Cap`] of `nvs_config`'s closed roster with what the snapshot writes for it, or its default
//! for server work where it writes nothing (`rule:security/capability-question-is-grant-and-scope`).
//! A default that grants nothing is not listed, and one closing line says so, so the list is as
//! long as what the program may do. Nothing is inferred from the code, and the first line names
//! the files it read and says the grants are the program's, never the route's.
//!
//! **The program is the one the hovered file belongs to.** A handler in a file another program
//! autoloads borrows that program's map ([`crate::document::Analysed::program`]), and its grants
//! are that entry's tree, read from the entry's directory as `nvs run` reads it. A line above the
//! list then names the entry. Below the list come the other grants the program carries: each
//! `[[extension]]` entry's own `grants` (`rule:security/extension-grants-are-an-intersection`),
//! and the `[grants]` table of the `package.toml` found from the entry
//! (`rule:security/grants-are-keyed-on-a-namespace`). `net.internal` is not a capability, so the
//! roster has no line for it, and it is listed after the roster's lines when it is written.
//!
//! **A path is written relative to the document's directory where it is inside it**, with `/`
//! between its parts, and whole otherwise. That keeps the line short for the common layout, and
//! an `.lspt` case's expectation the same on every machine.
//!
//! # What it costs
//!
//! One resolve of the configuration tree and one walk up to `package.toml` per hover on a route,
//! freed with the answer. Nothing at compile time for running, and nothing at run time.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use nvs_config::Cap;
use nvs_config::capability::Grant;
use nvs_config::defaults::Granted;
use nvs_config::tree::{Capabilities, Extension, Setting};
use nvs_diagnostics::{SourceMap, plain_path};

use crate::document::{Tree, program_tree};

/// The hover text for the grants of the program whose entry is `entry`, hovered in the file
/// `here`, with `data` as the data folder's `nvs.toml`: the file `nvs check` reads when the
/// entry's directory has none. `here` is `entry` for a file that is its own program's entry.
#[must_use]
pub fn of_program(entry: &Path, here: &Path, data: Option<&Path>) -> String {
    let dir = plain_path(&canonical(here.parent().unwrap_or(here))).into_owned();
    let home = plain_path(&canonical(entry.parent().unwrap_or(entry))).into_owned();
    let mut text = if canonical(entry) == canonical(here) {
        String::new()
    } else {
        format!(
            "This file is part of the program that starts at `{}`.\n\n",
            shown(&plain_path(&canonical(entry)), &dir)
        )
    };
    match program_tree(entry, data) {
        Tree::None => {
            text.push_str(
                "No `nvs.toml` was found for this program. It runs under the default policy:\n\n",
            );
            list(&mut text, None, &home, &dir);
        }
        Tree::Unread(files) => {
            let _ = write!(
                text,
                "The grants of this program are in {}. This configuration does not load, \
                 and `nvs check` shows the error.",
                names(&files, &dir)
            );
            return text;
        }
        Tree::Read {
            files,
            resolved,
            snapshot,
        } => {
            let folder = snapshot
                .folder()
                .map_or_else(|| home.clone(), |at| plain_path(&at).into_owned());
            let _ = write!(
                text,
                "These are the grants of the whole program, from {}. \
                 Every route of the program has the same grants.\n\n",
                names(&files, &dir)
            );
            list(
                &mut text,
                snapshot.config.capabilities.as_ref(),
                &folder,
                &dir,
            );
            extensions(&mut text, &resolved.config.extension, &dir);
        }
    }
    namespaces(&mut text, &home, &dir);
    text
}

/// The `[grants]` table of the `package.toml` found from the entry's directory `home`, one line
/// per key, or the sentence that says the table does not load. Nothing where there is no table.
fn namespaces(text: &mut String, home: &Path, dir: &Path) {
    match nvs_config::grants::Grants::read(home, &mut SourceMap::new()) {
        Ok(grants) if grants.is_empty() => {}
        Ok(grants) => {
            let file = grants.file().map_or_else(
                || "`package.toml`".to_owned(),
                |file| format!("`{}`", shown(&plain_path(&canonical(file)), dir)),
            );
            let _ = write!(
                text,
                "\n\nThe code in these namespaces has only these grants, from {file}:\n"
            );
            for line in grants.lines() {
                let caps = if line.caps().is_empty() {
                    "nothing".to_owned()
                } else {
                    line.caps()
                        .iter()
                        .map(|cap| format!("`{}`", cap.name()))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                let _ = write!(text, "\n- `{}`: {caps}", line.key());
            }
        }
        Err(_) => text.push_str(
            "\n\nThe `[grants]` table of `package.toml` does not load, \
             and `nvs check` shows the error.",
        ),
    }
}

/// One line per `[[extension]]` entry: what its own `grants` let it read, write and connect to.
fn extensions(text: &mut String, entries: &[Extension], dir: &Path) {
    let quoted = |items: &[String], path_scoped: bool| {
        items
            .iter()
            .map(|item| {
                if path_scoped {
                    format!("`{}`", shown(&plain_path(Path::new(item)), dir))
                } else {
                    format!("`{item}`")
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut first = true;
    for entry in entries {
        let Some(path) = &entry.path else { continue };
        if first {
            text.push_str("\n\nEach extension has its own grants:\n");
            first = false;
        }
        let grants = &entry.grants;
        let parts: Vec<String> = [
            ("reads", quoted(&grants.read, true)),
            ("writes", quoted(&grants.write, true)),
            ("connects to", quoted(&grants.connect, false)),
        ]
        .into_iter()
        .filter(|(_, items)| !items.is_empty())
        .map(|(verb, items)| format!("{verb} {items}"))
        .collect();
        let scope = if parts.is_empty() {
            "nothing".to_owned()
        } else {
            parts.join("; ")
        };
        let _ = write!(text, "\n- `{path}`: {scope}");
    }
}

/// One line per capability that grants anything, then the line that says the rest grant
/// nothing. `written` is the snapshot's `[capabilities]`, and `folder` the app folder a default
/// covers.
fn list(text: &mut String, written: Option<&Capabilities>, folder: &Path, dir: &Path) {
    let run = nvs_config::capability::Run::server(Some(folder));
    let mut left_out = false;
    for cap in Cap::ALL.iter().copied() {
        let scope = match written.and_then(|caps| cap.grant(caps).map(|_| caps.holds(cap))) {
            Some(Grant::Everything) => format!("{} (`true`)", everything(cap)),
            Some(Grant::Nothing) => "nothing".to_owned(),
            Some(Grant::These(entries)) => entries
                .iter()
                .map(|entry| format!("`{}`", entry_of(cap, entry, dir)))
                .collect::<Vec<_>>()
                .join(", "),
            None => match nvs_config::defaults::capability(cap, run) {
                Granted::Nothing => {
                    left_out = true;
                    continue;
                }
                Granted::Everything => format!("{} (default)", everything(cap)),
                Granted::AppFolder => format!(
                    "files in {}, except `nvs.toml` and names that start with `.` (default)",
                    folder_of(folder, dir)
                ),
                Granted::AppCode => format!(
                    "files in {}, outside its temporary directories (default)",
                    folder_of(folder, dir)
                ),
                Granted::Temporary => {
                    "only the directories `Core\\IO::temporaryDir` makes (default)".to_owned()
                }
            },
        };
        let _ = writeln!(text, "- `{}`: {scope}", cap.name());
    }
    let internal = written
        .and_then(|caps| caps.net.as_ref())
        .and_then(|net| net.internal.as_ref());
    if let Some(Setting::List(addresses)) = internal
        && !addresses.is_empty()
    {
        let addresses = addresses
            .iter()
            .map(|address| format!("`{address}`"))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            text,
            "- `net.internal`: {addresses}. `net.connect` may reach these private addresses."
        );
    }
    if left_out {
        text.push_str("\nNo other grant is given.");
    }
}

/// What `true` grants for `cap`, in the words of its scope.
fn everything(cap: Cap) -> &'static str {
    if cap.is_path_scoped() {
        "every path"
    } else if cap.is_host_scoped() {
        "every host"
    } else {
        "everything"
    }
}

/// One written entry of `cap`'s grant: a path as [`shown`] writes it, and anything else as
/// written.
fn entry_of(cap: Cap, entry: &str, dir: &Path) -> String {
    if cap.is_path_scoped() {
        shown(&plain_path(Path::new(entry)), dir)
    } else {
        entry.to_owned()
    }
}

/// The configuration files, each in backticks and joined with "and".
fn names(files: &[PathBuf], dir: &Path) -> String {
    files
        .iter()
        .map(|file| format!("`{}`", shown(&plain_path(&canonical(file)), dir)))
        .collect::<Vec<_>>()
        .join(" and ")
}

/// The app folder as a phrase: "this file's directory" where it is the document's own.
fn folder_of(folder: &Path, dir: &Path) -> String {
    if folder == dir {
        "this file's directory".to_owned()
    } else {
        format!("`{}`", shown(folder, dir))
    }
}

/// `path` relative to `dir`, with `/` between its parts, where it is inside `dir`, and whole
/// otherwise.
fn shown(path: &Path, dir: &Path) -> String {
    match path.strip_prefix(dir) {
        Ok(relative) if !relative.as_os_str().is_empty() => relative
            .components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/"),
        _ => path.display().to_string(),
    }
}

/// `path` canonical where it exists, so it compares with the snapshot's own paths.
fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
