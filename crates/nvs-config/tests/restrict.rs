//! `rule:packaging/a-restriction-limits-where-a-declaration-is-used`: the `[[restrict]]` table of
//! the application's `package.toml` and of a package's own, read from text each test writes.

use std::path::Path;

use nvs_config::cache::env_hash;
use nvs_config::grants::Grants;
use nvs_config::package::Package;
use nvs_config::restrict::Target;
use nvs_config::tree::Config;
use nvs_diagnostics::{Diagnostic, SourceMap, code};

/// The application's table of `text`.
fn application(text: &str) -> Result<Grants, Diagnostic> {
    Grants::parse(&mut SourceMap::new(), "package.toml", text)
}

/// The package `text` declares, as a package's `package.toml` in `modules/Framework`.
fn package(text: &str) -> Result<Option<Package>, Diagnostic> {
    Package::parse(
        &mut SourceMap::new(),
        "modules/Framework/package.toml",
        text,
        Path::new("modules/Framework"),
    )
}

/// The refusal of the application's table `text`.
fn refused(text: &str) -> Diagnostic {
    match application(text) {
        Ok(table) => panic!("the table was read: {:?}", table.restrictions()),
        Err(refused) => refused,
    }
}

const SIGNED: &str = concat!(
    "[grants]\n'Framework\\Core\\*' = []\n\n",
    "[[restrict]]\n",
    "use     = 'Core\\Router::urlSigned'\n",
    "allow   = ['Framework\\Core\\Signing\\*', 'App\\Links', 'App\\Mail::send']\n",
    "note    = \"Use `Framework\\\\Core\\\\Url::signed`.\"\n",
    "replace = 'Framework\\Core\\Url::signed($route, $params)'\n",
);

// covers: tools:packages/restrictions-where-a-class-or-method-may-be-used
#[test]
fn a_restrict_table_is_read_from_the_application_file() {
    let table = application(SIGNED).expect("the table reads");
    let entries = table.restrictions().entries();
    assert_eq!(entries.len(), 1);
    let entry = &entries[0];
    assert_eq!(
        *entry.target(),
        Target::Member {
            class: "Core\\Router".to_owned(),
            member: "urlSigned".to_owned(),
        }
    );
    assert_eq!(
        entry.allow(),
        [
            Target::Subtree("Framework\\Core\\Signing".to_owned()),
            Target::Name("App\\Links".to_owned()),
            Target::Member {
                class: "App\\Mail".to_owned(),
                member: "send".to_owned(),
            },
        ]
    );
    assert_eq!(entry.note(), Some("Use `Framework\\Core\\Url::signed`."));
    assert_eq!(
        entry.replace(),
        Some("Framework\\Core\\Url::signed($route, $params)")
    );
    assert_eq!(entry.package(), None);
    assert_eq!(entry.file(), "package.toml");
    assert_eq!(entry.line(), 5, "the line of the entry's `use`");
    assert_eq!(table.lines().len(), 1, "`[grants]` is still read beside it");

    let empty = application("[[restrict]]\nuse = '\\Acme\\Secret'\nallow = []\n")
        .expect("an empty list reads");
    assert!(empty.restrictions().entries()[0].allow().is_empty());
    assert_eq!(
        *empty.restrictions().entries()[0].target(),
        Target::Name("Acme\\Secret".to_owned()),
        "a leading `\\` means nothing"
    );
}

// covers: tools:packages/restrictions-where-a-class-or-method-may-be-used
#[test]
fn a_restrict_table_is_read_from_a_package_manifest() {
    let text = concat!(
        "[package]\nname = 'Framework\\Core'\n\n",
        "[[restrict]]\nuse = 'Core\\Router::urlSigned'\nallow = ['Framework\\Core\\Signing\\*']\n\n",
        "[[restrict]]\nuse = 'Framework\\Core\\Db\\*'\nallow = ['Framework\\Core\\Db']\n",
    );
    let found = package(text)
        .expect("the manifest reads")
        .expect("a named manifest is a package's");
    let entries = found.restrictions();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].package(), Some("Framework\\Core"));
    assert_eq!(entries[0].file(), "modules/Framework/package.toml");
    assert_eq!(
        *entries[1].target(),
        Target::Subtree("Framework\\Core\\Db".to_owned())
    );
    assert_eq!(entries[1].line(), 9);

    let mut grants = application("").expect("an empty file reads");
    assert!(grants.restrictions().is_empty());
    grants.set_restrictions(entries.iter().cloned());
    grants.set_restrictions(entries.iter().cloned());
    assert_eq!(
        grants.restrictions().entries().len(),
        2,
        "a second walk replaces the packages' entries"
    );
}

// covers: tools:packages/restrictions-where-a-class-or-method-may-be-used
#[test]
fn a_target_that_is_not_a_declaration_name_is_an_error() {
    for (written, line) in [
        ("'Core\\Router::'", 2),
        ("'Acme\\*\\Http'", 2),
        ("'Acme\\Http\\'", 2),
        ("''", 2),
        ("'Acme Http'", 2),
        ("'*'", 2),
        ("'Acme\\*::send'", 2),
    ] {
        let refusal = refused(&format!("[[restrict]]\nuse = {written}\nallow = []\n"));
        assert_eq!(refusal.code, Some(code::E_BAD_RESTRICTION), "{written}");
        assert!(
            refusal.message.contains("is not the name of a declaration"),
            "{written}"
        );
        let span = refusal
            .primary_span()
            .expect("the refusal points at the name");
        assert_eq!(span.start, 19, "{written} at line {line}");
    }
    let place = refused("[[restrict]]\nuse = 'Acme\\Http'\nallow = ['Acme\\**']\n");
    assert_eq!(place.code, Some(code::E_BAD_RESTRICTION));
    assert!(place.message.contains("`Acme\\**`"), "{}", place.message);

    let replace = refused("[[restrict]]\nuse = 'Acme\\*'\nallow = []\nreplace = 'x()'\n");
    assert_eq!(replace.code, Some(code::E_BAD_RESTRICTION));
    assert!(
        replace.message.contains("no replacement"),
        "{}",
        replace.message
    );
}

// covers: tools:packages/restrictions-where-a-class-or-method-may-be-used
#[test]
fn a_core_target_that_names_nothing_is_an_error() {
    let table = application(concat!(
        "[[restrict]]\nuse = 'App\\Unknown'\nallow = []\n\n",
        "[[restrict]]\nuse = 'Core\\Router::urlSigned'\nallow = []\n\n",
        "[[restrict]]\nuse = 'Core\\Router::urlSined'\nallow = []\n",
    ))
    .expect("the table reads");
    let known = |target: &Target| target.to_string() == "Core\\Router::urlSigned";
    let refusal = table
        .restrictions()
        .check_core(known)
        .expect_err("a `Core` typo is an error");
    assert_eq!(refusal.code, Some(code::E_BAD_RESTRICTION));
    assert_eq!(
        refusal.message,
        "`Core\\Router::urlSined` is not part of `Core`"
    );
    assert!(
        refusal.primary_span().is_some(),
        "the refusal points at the name"
    );

    let fine = application("[[restrict]]\nuse = 'Core\\Router::urlSigned'\nallow = []\n")
        .expect("the table reads");
    assert!(fine.restrictions().check_core(known).is_ok());
}

// covers: tools:packages/restrictions-where-a-class-or-method-may-be-used
#[test]
fn a_package_allow_outside_its_own_name_is_an_error() {
    for outside in [
        "App\\Links",
        "Framework\\Corel",
        "Framework\\*",
        "Other\\Core::send",
    ] {
        let text = format!(
            "[package]\nname = 'Framework\\Core'\n\n[[restrict]]\nuse = 'Core\\Router'\nallow = ['{outside}']\n"
        );
        let refusal = package(&text).expect_err(outside);
        assert_eq!(refusal.code, Some(code::E_BAD_RESTRICTION), "{outside}");
        assert!(
            refusal
                .message
                .contains("is not inside the package `Framework\\Core`"),
            "{}",
            refusal.message
        );
        assert!(refusal.primary_span().is_some(), "{outside}");
    }
    for inside in [
        "Framework\\Core",
        "Framework\\Core\\Signing\\*",
        "Framework\\Core\\Url::signed",
    ] {
        let text = format!(
            "[package]\nname = 'Framework\\Core'\n\n[[restrict]]\nuse = 'Core\\Router'\nallow = ['{inside}']\n"
        );
        assert!(package(&text).is_ok(), "{inside}");
    }
    assert!(
        application("[[restrict]]\nuse = 'Core\\Router'\nallow = ['App\\Links']\n").is_ok(),
        "the application's own entries may allow any namespace"
    );
}

// covers: tools:packages/restrictions-where-a-class-or-method-may-be-used
#[test]
fn a_package_use_outside_its_own_name_or_core_is_an_error() {
    for outside in ["Shop\\Cart", "Shop\\Cart::total", "Framework\\Corel", "Shop\\*"] {
        let text = format!(
            "[package]\nname = 'Framework\\Core'\n\n[[restrict]]\nuse = '{outside}'\nallow = []\n"
        );
        let refusal = package(&text).expect_err(outside);
        assert_eq!(refusal.code, Some(code::E_BAD_RESTRICTION), "{outside}");
        assert!(
            refusal
                .message
                .contains("is not inside the package `Framework\\Core`"),
            "{}",
            refusal.message
        );
    }
    for inside in [
        "Core\\Str::upper",
        "Core\\Router",
        "Framework\\Core\\Db\\*",
        "Framework\\Core\\Url::signed",
    ] {
        let text = format!(
            "[package]\nname = 'Framework\\Core'\n\n[[restrict]]\nuse = '{inside}'\nallow = []\n"
        );
        assert!(package(&text).is_ok(), "{inside}");
    }
    assert!(
        application("[[restrict]]\nuse = 'Shop\\Cart'\nallow = []\n").is_ok(),
        "the application's own entries may name any declaration"
    );
}

// covers: tools:packages/restrictions-where-a-class-or-method-may-be-used
#[test]
fn an_unknown_restrict_field_is_an_error() {
    let refusal = refused("[[restrict]]\nuse = 'Core\\Router'\nallow = []\nallowed = []\n");
    assert_eq!(refusal.code, Some(code::E_BAD_RESTRICTION));
    assert!(refusal.message.contains("allowed"), "{}", refusal.message);

    let missing = refused("[[restrict]]\nuse = 'Core\\Router'\n");
    assert_eq!(missing.code, Some(code::E_BAD_RESTRICTION));
    assert!(missing.message.contains("allow"), "{}", missing.message);

    let in_package = package(
        "[package]\nname = 'Framework\\Core'\n[[restrict]]\nuse = 'Core\\Router'\nallow = []\nreason = ''\n",
    )
    .expect_err("an unknown field in a package's file");
    assert_eq!(in_package.code, Some(code::E_BAD_RESTRICTION));
}

// covers: tools:packages/restrictions-where-a-class-or-method-may-be-used
#[test]
fn an_edited_restriction_changes_the_cache_key() {
    let config = Config::default();
    let key = |text: &str| env_hash(&config, &application(text).expect("the table reads"));
    let base = "[[restrict]]\nuse = 'Core\\Router::urlSigned'\nallow = ['App\\Links']\n";
    let first = key(base);
    assert_ne!(
        first,
        key("[[restrict]]\nuse = 'Core\\Router::urlSigned'\nallow = ['App\\Mail']\n")
    );
    assert_ne!(
        first,
        key("[[restrict]]\nuse = 'Core\\Router::url'\nallow = ['App\\Links']\n")
    );
    assert_ne!(
        first,
        key("[[restrict]]\nuse = 'Core\\Router::urlSigned'\nallow = []\n")
    );
    assert_ne!(first, key(&format!("{base}note = 'Use another method.'\n")));
    assert_ne!(first, key(&format!("{base}replace = 'f()'\n")));
    assert_eq!(
        first,
        key(&format!(
            "{base}# a comment\n[package]\nversion = '2.0.0'\n"
        )),
        "a comment and another table that move no entry change no key"
    );
    // A refused reflective act throws a text that names the entry's line.
    assert_ne!(
        first,
        key(&format!("# a comment\n{base}")),
        "an entry moved to another line changes the key"
    );

    let found = package(
        "[package]\nname = 'Framework\\Core'\n[[restrict]]\nuse = 'Core\\Router'\nallow = []\n",
    )
    .expect("the manifest reads")
    .expect("a package");
    let mut grants = Grants::default();
    let none = env_hash(&config, &grants);
    grants.set_restrictions(found.restrictions().iter().cloned());
    assert_ne!(
        none,
        env_hash(&config, &grants),
        "a package's entry is in the key too"
    );
}

// covers: tools:packages/restrictions-where-a-class-or-method-may-be-used
#[test]
fn a_program_with_no_restriction_keeps_its_cache_key() {
    let config = Config::default();
    let grants = "[grants]\n'Acme\\Http' = ['net.connect']\n";
    let table = application(grants).expect("the table reads");
    assert!(table.restrictions().digest().is_none());
    assert_eq!(
        env_hash(&config, &table),
        env_hash(
            &config,
            &application(&format!("{grants}[package]\nversion = '1.0.0'\n")).expect("reads")
        ),
    );
    let mut empty = Grants::default();
    let before = env_hash(&config, &empty);
    empty.set_restrictions(Vec::new());
    assert_eq!(
        before,
        env_hash(&config, &empty),
        "no package entry adds nothing"
    );
}
