//! The reference chapter and the extension's manifest name one contribution set.
//!
//! `docs/reference/tools/40-editor.md` § *The VS Code extension* is where a
//! person reads what settings and commands the extension gives them, and
//! `editors/vscode/package.json` is what the editor actually reads. Nothing
//! links the two: a setting renamed in the manifest leaves the chapter
//! describing a key that no longer exists, and the reader has no way to tell.
//! The extension's own headless suite freezes the manifest against a roster it
//! carries in TypeScript and never opens the chapter, so this is the half of
//! that agreement nothing else checks.
//!
//! It lives in this crate because the chapter it reads is this server's chapter
//! and `tests/stdout_policy.rs` already reads files outside this package from
//! here. It links nothing of the client: the manifest is JSON and the chapter is
//! a table, and both are read as text, each found through `nvs_repo::path`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Read a file this test knows must exist, or fail naming it.
fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

/// The extension's manifest, parsed.
fn manifest() -> serde_json::Value {
    let path = nvs_repo::path("editors/vscode/package.json");
    serde_json::from_str(&read(&path)).unwrap_or_else(|err| panic!("{}: {err}", path.display()))
}

/// The rows of the chapter's table under `heading`, as first cell to second.
///
/// The table is read positionally — the leading `| ` rows that follow the
/// heading, minus the header and the separator — because the alternative is a
/// Markdown library in a test that already knows exactly which two tables it
/// wants. A cell's `` ` `` fences are stripped: the chapter writes an
/// identifier as code and the manifest writes it bare.
fn documented(heading: &str) -> BTreeMap<String, String> {
    let path = nvs_repo::path("docs/reference/tools/40-editor.md");
    let chapter = read(&path);
    let section = chapter
        .split_once(heading)
        .unwrap_or_else(|| panic!("{} has no {heading}", path.display()))
        .1;

    section
        .lines()
        .skip_while(|line| !line.starts_with('|'))
        .take_while(|line| line.starts_with('|'))
        .skip(2)
        .map(|row| {
            let mut cells = row.trim_matches('|').split('|').map(|cell| cell.trim());
            let key = cells
                .next()
                .unwrap_or_default()
                .trim_matches('`')
                .to_owned();
            let second = cells
                .next()
                .unwrap_or_default()
                .trim_matches('`')
                .to_owned();
            (key, second)
        })
        .collect()
}

/// A JSON default as the chapter's Default column writes it: a string keeps its
/// quotes, so `""` and `"off"` are visibly strings and `150` is visibly not.
fn as_written(default: &serde_json::Value) -> String {
    default.to_string()
}

// covers: tools:editor/the-vs-code-extension
#[test]
fn the_chapter_documents_every_contributed_setting_and_no_other() {
    let manifest = manifest();
    let properties = manifest["contributes"]["configuration"]["properties"]
        .as_object()
        .expect("the manifest contributes a configuration block");

    let contributed: BTreeMap<String, String> = properties
        .iter()
        .map(|(key, schema)| (key.clone(), as_written(&schema["default"])))
        .collect();
    let documented = documented("## The settings it contributes");

    assert_eq!(
        contributed.keys().collect::<Vec<_>>(),
        documented.keys().collect::<Vec<_>>(),
        "the chapter's settings table and the manifest name different settings"
    );
    for (key, default) in &contributed {
        assert_eq!(
            documented[key], *default,
            "{key}'s default is documented as {} and contributed as {default}",
            documented[key]
        );
    }
}

// covers: tools:editor/the-vs-code-extension
#[test]
fn the_chapter_documents_every_contributed_command_and_no_other() {
    let manifest = manifest();
    let commands = manifest["contributes"]["commands"]
        .as_array()
        .expect("the manifest contributes commands");

    let contributed: BTreeMap<String, String> = commands
        .iter()
        .map(|command| {
            assert_eq!(
                command["category"], "Novis",
                "the chapter says every command is under one category"
            );
            let id = command["command"].as_str().expect("a command has an id");
            let title = command["title"].as_str().expect("a command has a title");
            (id.to_owned(), title.to_owned())
        })
        .collect();
    let documented = documented("## The commands it contributes");

    assert_eq!(
        contributed.keys().collect::<Vec<_>>(),
        documented.keys().collect::<Vec<_>>(),
        "the chapter's commands table and the manifest name different commands"
    );
    for (id, title) in &contributed {
        assert_eq!(
            documented[id], *title,
            "{id} is titled {} in the chapter and {title} in the manifest",
            documented[id]
        );
    }
}

/// The extension wakes on a Novis document or a workspace with an `nvs.toml`,
/// and claims no PHP file.
///
/// The second activation event is not a claim: an `nvs.toml` gets one
/// completion provider selected by its file name and no language
/// (`rule:ide/the-extension-claims-nvs-only`).
///
/// What `rule:ide/the-extension-claims-nvs-only` refuses is the *claim* — a
/// language id, a file extension, an activation event — and not the three
/// letters: `nvs.completion.phpNames` is on the frozen roster of
/// `rule:ide/contributions-are-frozen-and-only-ever-added` and names PHP
/// because the setting is about PHP's names. So what is scanned for is the file
/// extension and the language id, which are the two spellings that would make
/// VS Code hand this extension a `.php` buffer and lose that fight silently
/// against every PHP extension the user already has.
// covers: tools:editor/the-vs-code-extension
#[test]
fn the_chapter_and_the_manifest_agree_the_extension_claims_nvs_alone() {
    let manifest = manifest();
    let raw = read(&nvs_repo::path("editors/vscode/package.json")).to_lowercase();

    assert_eq!(
        manifest["activationEvents"],
        serde_json::json!(["onLanguage:nvs", "workspaceContains:**/nvs.toml"]),
        "the chapter says the extension wakes on a Novis document or a workspace with an nvs.toml"
    );
    for language in manifest["contributes"]["languages"]
        .as_array()
        .expect("the manifest contributes a language")
    {
        let id = language["id"]
            .as_str()
            .expect("a contributed language has an id");
        assert!(
            matches!(id, "nvs" | "nvst"),
            "the manifest contributes the language `{id}`, which is not one of Novis's own"
        );
    }
    // A quoted token and not the three letters: `nvs.completion.phpNames` is a
    // setting name and `".php"` is a claim on somebody's file.
    for claim in [".php\"", "\"php\"", ":php", "phtml"] {
        assert!(
            !raw.contains(claim),
            "the manifest claims `{claim}`, and the chapter says it claims no PHP file \
             (`rule:ide/the-extension-claims-nvs-only`)"
        );
    }
}
