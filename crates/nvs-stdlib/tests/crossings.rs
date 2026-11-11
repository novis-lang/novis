//! Every call of a graph door that lands a value in a program names its
//! receiving side (`rule:security/isolate-values-cross-by-copy`).
//!
//! The doors are read from `crates/nvs-runtime/src/graph.rs` itself: each
//! public function of its shipped half that returns a `Value`. A door whose
//! signature takes a `Receiving` is called well when its receiving argument is
//! a value built by `Receiving` or `Ctx::receiving`, not `None` and not a
//! closure written at the call. A door that takes no `Receiving` lands a value
//! nowhere in particular, so every shipped call of it is named here.
//!
//! The scan reads every shipped `.rs` file under `crates/*/src`, up to its test
//! module, so a new crossing anywhere in the tree is held to the same rule.

use std::path::{Path, PathBuf};

/// One public function of the graph module that returns a value.
struct Door {
    name: String,
    /// Whether its signature takes the receiving side.
    receives: bool,
}

/// The lines of `text` up to its test module, each comment line left empty.
fn shipped(text: &str) -> Vec<&str> {
    let lines: Vec<&str> = text.lines().collect();
    let mut kept = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        if line.trim() == "#[cfg(test)]"
            && lines[at + 1..]
                .iter()
                .find(|next| !next.trim().is_empty())
                .is_some_and(|next| next.split_whitespace().any(|word| word == "mod"))
        {
            break;
        }
        // A comment line is kept empty, so a line number stays the file's own.
        kept.push(if line.trim_start().starts_with("//") {
            ""
        } else {
            line
        });
    }
    kept
}

/// The doors `graph.rs` declares: each `pub fn` of its shipped half whose
/// return type names `Value`.
fn doors(graph: &str) -> Vec<Door> {
    let text = shipped(graph).join("\n");
    let mut found = Vec::new();
    for (at, _) in text.match_indices("\npub fn ") {
        let rest = &text[at + "\npub fn ".len()..];
        let signature = &rest[..rest.find('{').unwrap_or(rest.len())];
        let name: String = signature
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let returns = signature.split("->").nth(1).unwrap_or("");
        if returns.contains("Value") {
            found.push(Door {
                name,
                receives: signature.contains("Receiving"),
            });
        }
    }
    found
}

/// Every `.rs` file under `dir`, depth first.
fn sources(dir: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, into);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            into.push(path);
        }
    }
}

/// Whether `text` imports `name` from the runtime crate or its graph module, so
/// a bare call of it is a call of the door.
fn imports(text: &str, name: &str) -> bool {
    text.split("use ").skip(1).any(|statement| {
        let statement = &statement[..statement.find(';').unwrap_or(statement.len())];
        (statement.starts_with("nvs_runtime::") || statement.starts_with("crate::graph::"))
            && statement
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .any(|word| word == name)
    })
}

/// The arguments of the call whose `(` is at `open`, split at top-level commas.
fn arguments(text: &str, open: usize) -> Vec<String> {
    let mut depth = 0usize;
    let mut args = vec![String::new()];
    for c in text[open..].chars() {
        match c {
            '(' | '[' | '{' => {
                depth += 1;
                if depth == 1 {
                    continue;
                }
            }
            ')' | ']' | '}' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            ',' if depth == 1 => {
                args.push(String::new());
                continue;
            }
            _ => {}
        }
        if let Some(last) = args.last_mut() {
            last.push(c);
        }
    }
    args.into_iter()
        .map(|arg| arg.trim().to_owned())
        .filter(|arg| !arg.is_empty())
        .collect()
}

/// What is wrong with the call of `door` at `open`, or `None` for a good call.
fn fault(door: &Door, text: &str, open: usize) -> Option<String> {
    if !door.receives {
        return Some("names no receiving side".to_owned());
    }
    let args = arguments(text, open);
    let Some(receiving) = args.get(1) else {
        return Some("has no receiving argument".to_owned());
    };
    if receiving == "None" {
        return Some("passes `None` as its receiving side".to_owned());
    }
    if receiving.contains('|') {
        return Some(format!("writes a resolver at the site: `{receiving}`"));
    }
    None
}

#[test]
fn every_crossing_site_names_its_receiving_side() {
    let graph_path = nvs_repo::path("crates/nvs-runtime/src/graph.rs");
    let graph = std::fs::read_to_string(&graph_path).expect("graph.rs is readable");
    let doors = doors(&graph);
    for expected in ["copy_graph_to", "decode"] {
        assert!(
            doors
                .iter()
                .any(|door| door.name == expected && door.receives),
            "the scan of graph.rs found `{expected}` taking a receiving side"
        );
    }

    let mut files = Vec::new();
    for krate in std::fs::read_dir(nvs_repo::path("crates"))
        .expect("the crates folder")
        .flatten()
    {
        sources(&krate.path().join("src"), &mut files);
    }
    let mut faults = Vec::new();
    let mut calls = 0usize;
    for file in &files {
        if *file == graph_path {
            continue;
        }
        let Ok(whole) = std::fs::read_to_string(file) else {
            continue;
        };
        let text = shipped(&whole).join("\n");
        for door in &doors {
            let bare = imports(&text, &door.name);
            let needle = format!("{}(", door.name);
            for (at, _) in text.match_indices(&needle) {
                let before = &text[..at];
                if before.ends_with(|c: char| c.is_alphanumeric() || c == '_') {
                    continue;
                }
                let qualified = before.ends_with("nvs_runtime::") || before.ends_with("graph::");
                if before.ends_with("fn ") || !(qualified || bare && !before.ends_with("::")) {
                    continue;
                }
                calls += 1;
                if let Some(why) = fault(door, &text, at + door.name.len()) {
                    let line = text[..at].matches('\n').count() + 1;
                    let shown = file.strip_prefix(nvs_repo::root()).unwrap_or(file);
                    faults.push(format!("{}:{line}: `{}` {why}", shown.display(), door.name));
                }
            }
        }
    }
    assert!(
        calls > 5,
        "the scan found only {calls} calls of a graph door"
    );
    assert!(
        faults.is_empty(),
        "every call of a graph door names the receiving side it lands in:\n{}",
        faults.join("\n")
    );
}
