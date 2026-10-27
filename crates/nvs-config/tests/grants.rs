//! `rule:security/grants-are-keyed-on-a-namespace` and `rule:security/longest-prefix-wins`: the
//! `[grants]` table of the application's `package.toml`, read from a file each test writes.

use std::path::Path;

use nvs_config::cache::env_hash;
use nvs_config::capability::Cap;
use nvs_config::grants::{self, Grants};
use nvs_config::tree::Config;
use nvs_diagnostics::{Diagnostic, SourceMap, code};

/// The table of `text`, written as `package.toml` at the root of a fresh directory and read
/// back from `below`, a directory under it.
fn written(text: &str, below: &str) -> Result<Grants, Diagnostic> {
    let dir = nvs_repo::scratch("grants");
    std::fs::write(dir.join(grants::FILE), text).expect("the test writes its package.toml");
    let start = dir.join(below);
    std::fs::create_dir_all(&start).expect("the test makes its working directory");
    let read = Grants::read(&start, &mut SourceMap::new());
    if let Ok(table) = &read {
        assert_eq!(table.file(), Some(dir.join(grants::FILE).as_path()));
    }
    read
}

fn table(text: &str) -> Grants {
    Grants::parse(&mut SourceMap::new(), "package.toml", text)
        .unwrap_or_else(|refused| panic!("the table reads: {}", refused.message))
}

fn refused(text: &str) -> Diagnostic {
    match Grants::parse(&mut SourceMap::new(), "package.toml", text) {
        Ok(table) => panic!("the table was read: {table:?}"),
        Err(refused) => refused,
    }
}

/// The key that holds for `namespace`, as written.
fn winner(table: &Grants, namespace: &str) -> Option<String> {
    table.lookup(namespace).map(grants::Grant::key)
}

#[test]
fn the_table_is_found_by_walking_up_from_the_working_directory() {
    let table = written(
        "[package]\nname = 'Acme/App'\n\n[grants]\n'Acme\\Http' = ['net.connect']\n",
        "src/deep/er",
    )
    .expect("the table reads");
    let grant = table.lookup("Acme\\Http").expect("the key matches");
    assert_eq!(grant.caps(), [Cap::NetConnect]);
}

#[test]
fn a_tree_with_no_package_toml_has_the_empty_table() {
    let dir = nvs_repo::scratch("grants-none");
    let table = Grants::read(dir.path(), &mut SourceMap::new()).expect("nothing to read");
    assert!(table.is_empty());
    assert_eq!(table.file(), None);
    assert_eq!(grants::find(Path::new(dir.path())), None);
}

#[test]
fn a_file_without_grants_has_the_empty_table_and_its_other_tables_are_not_read() {
    let table = table("[package]\nname = 'Acme/App'\nanything = { else = 1 }\n");
    assert!(table.is_empty());
}

#[test]
fn an_unknown_capability_is_an_error_naming_it() {
    let refused = refused("[grants]\n'Acme\\Http' = ['net.connect', 'fs.delete']\n");
    assert_eq!(refused.code, Some(code::E_BAD_GRANT));
    assert!(
        refused.message.contains("`fs.delete`"),
        "{}",
        refused.message
    );
}

#[test]
fn a_value_that_is_not_a_list_of_names_is_an_error() {
    for text in [
        "[grants]\n'Acme\\Http' = 'net.connect'\n",
        "[grants]\n'Acme\\Http' = [1]\n",
        "grants = 'all'\n",
        "[grants\n",
    ] {
        assert_eq!(refused(text).code, Some(code::E_BAD_GRANT), "{text}");
    }
}

#[test]
fn a_key_that_is_not_a_namespace_is_an_error() {
    for key in [
        "*",
        "\\*",
        "",
        "Acme\\",
        "Acme\\\\Http",
        "Acme\\*\\Http",
        "Acme Http",
        "9Acme",
    ] {
        let refused = refused(&format!("[grants]\n'{key}' = []\n"));
        assert_eq!(refused.code, Some(code::E_BAD_GRANT), "{key:?}");
    }
}

#[test]
fn a_bare_key_grants_only_that_namespace() {
    let table = table("[grants]\n'Acme\\Http' = ['net.connect']\n");
    assert_eq!(winner(&table, "Acme\\Http"), Some("Acme\\Http".to_owned()));
    assert_eq!(
        winner(&table, "\\Acme\\Http"),
        Some("Acme\\Http".to_owned())
    );
    assert_eq!(winner(&table, "Acme\\Http\\Client"), None);
    assert_eq!(winner(&table, "Acme\\HttpClient"), None);
    assert_eq!(winner(&table, "Acme"), None);
    assert_eq!(winner(&table, ""), None);
}

#[test]
fn a_subtree_key_grants_its_namespace_and_every_depth_below_it() {
    let table = table("[grants]\n'Acme\\Http\\*' = ['net.connect']\n");
    for namespace in [
        "Acme\\Http",
        "Acme\\Http\\Client",
        "Acme\\Http\\Client\\Pool",
    ] {
        assert_eq!(
            winner(&table, namespace),
            Some("Acme\\Http\\*".to_owned()),
            "{namespace}"
        );
    }
    assert_eq!(winner(&table, "Acme\\HttpClient"), None);
    assert_eq!(winner(&table, "Acme"), None);
}

#[test]
fn names_are_compared_exactly() {
    let table = table("[grants]\n'Acme\\Http\\*' = ['net.connect']\n");
    assert_eq!(winner(&table, "acme\\http"), None);
}

// covers: tools:packages/grants-what-each-namespace-may-do
#[test]
fn the_longest_key_wins_and_an_exact_key_beats_a_subtree_key_of_its_length() {
    let table = table(concat!(
        "[grants]\n",
        "'Acme\\*' = ['fs.read']\n",
        "'Acme\\Http\\*' = ['net.connect']\n",
        "'Acme\\Http' = ['net.connect', 'fs.read']\n",
        "'Acme\\Http\\Cache' = []\n",
    ));
    assert_eq!(winner(&table, "Acme"), Some("Acme\\*".to_owned()));
    assert_eq!(winner(&table, "Acme\\Mail"), Some("Acme\\*".to_owned()));
    assert_eq!(winner(&table, "Acme\\Http"), Some("Acme\\Http".to_owned()));
    assert_eq!(
        winner(&table, "Acme\\Http\\Client"),
        Some("Acme\\Http\\*".to_owned())
    );
    let cache = table.lookup("Acme\\Http\\Cache").expect("the key matches");
    assert!(
        cache.caps().is_empty(),
        "a longer key granting nothing wins"
    );
    assert!(
        !table
            .lookup("Acme\\Http\\Client")
            .expect("matches")
            .grants(Cap::FsRead)
    );
}

#[test]
fn two_keys_that_match_identically_are_an_error_naming_both() {
    let tie = refused("[grants]\n'Acme\\Http' = []\n'\\Acme\\Http' = ['net.connect']\n");
    assert_eq!(tie.code, Some(code::E_GRANT_KEYS_TIE));
    assert!(
        tie.message.contains("`Acme\\Http`") && tie.message.contains("`\\Acme\\Http`"),
        "{}",
        tie.message
    );
    let subtree = refused("[grants]\n'Acme\\Http\\*' = []\n'\\Acme\\Http\\*' = []\n");
    assert_eq!(subtree.code, Some(code::E_GRANT_KEYS_TIE));
}

#[test]
fn a_key_under_core_is_an_error() {
    for key in [
        "Core",
        "Core\\*",
        "Core\\Http",
        "\\Core\\Http",
        "core\\Http",
    ] {
        let refused = refused(&format!("[grants]\n'{key}' = ['net.connect']\n"));
        assert_eq!(refused.code, Some(code::E_GRANT_UNDER_CORE), "{key}");
    }
    assert!(
        table("[grants]\n'Acme\\Core' = []\n'Cores' = []\n")
            .lines()
            .len()
            == 2
    );
}

#[test]
fn an_edited_grant_line_changes_the_unit_key_and_nothing_else_in_the_file_does() {
    let config = Config::default();
    let key = |text: &str| env_hash(&config, &table(text));
    let first = key("[grants]\n'Acme\\Http' = ['net.connect']\n");
    assert_ne!(
        first,
        key("[grants]\n'Acme\\Http' = ['net.connect', 'fs.read']\n")
    );
    assert_ne!(first, key("[grants]\n'Acme\\Http\\*' = ['net.connect']\n"));
    assert_ne!(first, key("[grants]\n'Acme\\Http' = []\n"));
    assert_ne!(first, env_hash(&config, &Grants::default()));
    assert_eq!(
        first,
        key(concat!(
            "# a comment\n[package]\nversion = '2.0.0'\n\n",
            "[grants]\n'\\Acme\\Http' = ['net.connect', 'net.connect']\n",
        ))
    );
}
