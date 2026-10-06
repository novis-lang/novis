//! `rule:packaging/extension-loading-is-root-controlled`'s boot and reload refusal: an
//! `[[extension]]` entry with no `path`, no `sha256`, or a pin that is not 64 hexadecimal digits
//! does not resolve, and the refusal points at the entry's own line in the file that wrote it.
//!
//! Every case resolves a whole tree, not one file, because the line is found through the merge's
//! origins: an entry an include appended has to be named in the include, at its own header, and a
//! per-file check could not get that wrong in the way this one could.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use nvs_config::Setting;
use nvs_config::cache::env_hash;
use nvs_config::extension::{self, Granted, is_pin};
use nvs_config::resolve::{Files, Resolved, Roots, resolve};
use nvs_config::tree::Grants;
use nvs_config::trust::Untrusted;
use nvs_diagnostics::{Diagnostic, SourceMap, code};

/// A pin of the right shape. Which file it is the digest of is the loader's question, not this one.
const PIN: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";

fn p(path: &str) -> PathBuf {
    path.split('/').collect()
}

/// The configuration files the cases write, by name.
struct Fake(BTreeMap<PathBuf, String>);

impl Fake {
    fn with(entries: &[(&str, &str)]) -> Self {
        Self(
            entries
                .iter()
                .map(|(path, text)| (p(path), (*text).to_string()))
                .collect(),
        )
    }
}

impl Files for Fake {
    fn trust(&self, path: &Path) -> Result<PathBuf, Untrusted> {
        self.canonical(path).map_err(Untrusted::Unreadable)
    }

    fn canonical(&self, path: &Path) -> Result<PathBuf, String> {
        if self.exists(path) {
            Ok(path.to_path_buf())
        } else {
            Err("no such file".to_string())
        }
    }

    fn read(&self, path: &Path) -> Result<String, String> {
        self.0
            .get(path)
            .cloned()
            .ok_or_else(|| "no such file".to_string())
    }

    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, String> {
        self.read(path).map(String::into_bytes)
    }

    fn exposure(&self, _path: &Path) -> Option<String> {
        None
    }

    fn list(&self, dir: &Path) -> Result<Vec<PathBuf>, String> {
        Ok(self
            .0
            .keys()
            .filter(|path| path.parent() == Some(dir))
            .cloned()
            .collect())
    }

    fn exists(&self, path: &Path) -> bool {
        self.0.contains_key(path) || self.0.keys().any(|file| file.starts_with(path))
    }
}

/// The tree `nvs.toml` roots, panicking with the refusal when it does not resolve.
fn resolved(files: &[(&str, &str)]) -> Resolved {
    let mut sources = SourceMap::new();
    resolve(
        &Roots::Files(vec![p("nvs.toml")]),
        &mut sources,
        &Fake::with(files),
    )
    .unwrap_or_else(|err| panic!("refused: {} [{:?}]", err.message, err.notes))
}

/// The refusal, and the file name and 1-based line its label points at.
fn refused(files: &[(&str, &str)]) -> (Diagnostic, String, usize) {
    let mut sources = SourceMap::new();
    let err = resolve(
        &Roots::Files(vec![p("nvs.toml")]),
        &mut sources,
        &Fake::with(files),
    )
    .expect_err("this tree should have been refused");
    let span = err
        .primary_span()
        .unwrap_or_else(|| panic!("`{}` points at no line", err.message));
    let file = sources
        .get(span.file)
        .expect("the span's file is in the map");
    let (line, _) = file.line_col(span.start);
    let name = file.name().to_owned();
    (err, name, line + 1)
}

#[test]
fn an_extension_entry_with_a_path_a_pin_and_a_memory_ceiling_resolves() {
    let tree = resolved(&[(
        "nvs.toml",
        &format!("[[extension]]\npath = \"geo.nvsx\"\nsha256 = \"{PIN}\"\nmemory = \"64M\"\n"),
    )]);

    assert_eq!(tree.config.extension.len(), 1);
    assert_eq!(tree.config.extension[0].sha256.as_deref(), Some(PIN));
    assert_eq!(
        tree.config.extension[0].memory,
        Some(Setting::Text("64M".to_string()))
    );
}

/// `sha256sum` prints lower case and PowerShell's `Get-FileHash` upper case, and both are the
/// operator's own tool, so both are a pin. Anything else of 64 characters is not.
#[test]
fn a_pin_is_64_hexadecimal_digits_in_either_case() {
    assert!(is_pin(PIN));
    assert!(is_pin(&PIN.to_uppercase()));
    assert!(!is_pin(&PIN[1..]), "63 digits");
    assert!(!is_pin(&format!("{PIN}0")), "65 digits");
    assert!(!is_pin(&PIN.replace('9', "g")), "a letter past `f`");
    assert!(!is_pin(&format!("sha256:{}", &PIN[7..])), "a prefix");
}

/// The refusal `text` resolves to is `E0651`, says `said`, and points at `line` of `nvs.toml`.
fn assert_refused_at(text: &str, line: usize, said: &str) {
    let (err, file, at) = refused(&[("nvs.toml", text)]);
    assert_eq!(err.code, Some(code::E_BAD_EXTENSION_ENTRY), "{text}");
    assert!(
        err.message.contains(said),
        "{text}\n-- said: {}",
        err.message
    );
    assert_eq!((file.as_str(), at), ("nvs.toml", line), "{text}");
}

/// A missing key is counted at the entry's header, here below another block.
// covers: directive:extension
#[test]
fn an_extension_entry_without_a_pin_is_refused_naming_its_file_and_line() {
    assert_refused_at(
        "[limits]\nmemory = \"128M\"\n\n[[extension]]\npath = \"geo.nvsx\"\n",
        4,
        "`geo.nvsx` has no `sha256`",
    );
}

#[test]
fn an_extension_entry_without_a_path_is_refused_naming_its_file_and_line() {
    assert_refused_at(
        &format!("[[extension]]\nsha256 = \"{PIN}\"\n"),
        1,
        "has no `path`",
    );
}

/// A pin of the wrong shape is counted at its own `sha256` line, not the header.
#[test]
fn an_extension_pin_that_is_not_64_hex_digits_is_refused_naming_its_file_and_line() {
    assert_refused_at(
        "[[extension]]\npath = \"geo.nvsx\"\n# the pin\nsha256 = \"abc\"\n",
        4,
        "it has 3 characters",
    );
}

/// The second entry of an included file is the merged array's third, and the refusal names the
/// include at that entry's header, not the root and not the include's first entry.
#[test]
fn an_appended_entry_is_refused_in_the_file_that_wrote_it() {
    let root = format!(
        "[[include]]\npath = \"conf.d/shop.toml\"\n\n\
         [[extension]]\npath = \"blog.nvsx\"\nsha256 = \"{PIN}\"\n"
    );
    let include = format!(
        "[[extension]]\npath = \"shop.nvsx\"\nsha256 = \"{PIN}\"\n\n\
         [[extension]]\npath = \"cart.nvsx\"\n"
    );
    let (err, file, at) = refused(&[("nvs.toml", &root), ("conf.d/shop.toml", &include)]);

    assert_eq!(err.code, Some(code::E_BAD_EXTENSION_ENTRY));
    assert!(err.message.contains("`cart.nvsx`"), "{}", err.message);
    assert!(file.ends_with("shop.toml"), "named `{file}`");
    assert_eq!(at, 5);
}

/// `rule:security/extension-grants-are-an-intersection`: the operator's third of a guest's
/// effective set is read as written, and a key that is not `read`, `write` or `connect` is the
/// unknown-key refusal every other block gives.
#[test]
fn an_extension_entry_reads_its_grants_and_refuses_an_unknown_grant_key() {
    let tree = resolved(&[(
        "nvs.toml",
        &format!(
            "[[extension]]\npath = \"geo.nvsx\"\nsha256 = \"{PIN}\"\n\
             grants = {{ read = [\"data/geo/\"], write = [\"cache\"], connect = [\"tiles.example.com\"] }}\n"
        ),
    )]);
    let grants = &tree.config.extension[0].grants;
    assert_eq!(grants.read, ["data/geo/"]);
    assert_eq!(grants.write, ["cache"]);
    assert_eq!(grants.connect, ["tiles.example.com"]);

    let mut sources = SourceMap::new();
    let err = resolve(
        &Roots::Files(vec![p("nvs.toml")]),
        &mut sources,
        &Fake::with(&[(
            "nvs.toml",
            &format!(
                "[[extension]]\npath = \"geo.nvsx\"\nsha256 = \"{PIN}\"\n\
                 grants = {{ conect = [\"tiles.example.com\"] }}\n"
            ),
        )]),
    )
    .expect_err("a misspelt grant key should be refused");
    assert_eq!(err.code, Some(code::E_BAD_DIRECTIVE));
    assert!(err.message.contains("unknown field"), "{}", err.message);
    assert!(err.message.contains("conect"), "{}", err.message);
}

/// No `grants` and no I/O: every list is empty, whatever a manifest will later request.
#[test]
fn an_extension_entry_with_no_grants_holds_none() {
    let tree = resolved(&[(
        "nvs.toml",
        &format!("[[extension]]\npath = \"geo.nvsx\"\nsha256 = \"{PIN}\"\n"),
    )]);

    assert_eq!(tree.config.extension[0].grants, Grants::default());
    assert_eq!(
        extension::grants(0, &tree.config.extension[0].grants, &tree.origins),
        Granted::default(),
    );
}

/// `rule:config/a-relative-path-resolves-against-the-file-it-is-written-in`: a root an include
/// grants is beside the include, and one the root file grants is beside the root file. A host is
/// not a path and is kept as written.
#[test]
fn a_relative_extension_grant_root_resolves_against_the_file_that_wrote_it() {
    let root = format!(
        "[[include]]\npath = \"conf.d/geo.toml\"\n\n\
         [[extension]]\npath = \"shop.nvsx\"\nsha256 = \"{PIN}\"\ngrants = {{ write = [\"cache\"] }}\n"
    );
    let include = format!(
        "[[extension]]\npath = \"geo.nvsx\"\nsha256 = \"{PIN}\"\n\
         grants = {{ read = [\"data/geo/\", \"../shared\"], connect = [\"tiles.example.com\"] }}\n"
    );
    let tree = resolved(&[("nvs.toml", &root), ("conf.d/geo.toml", &include)]);

    let by_path = |name: &str| {
        let index = tree
            .config
            .extension
            .iter()
            .position(|entry| entry.path.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("no entry loads `{name}`"));
        extension::grants(index, &tree.config.extension[index].grants, &tree.origins)
    };

    let geo = by_path("geo.nvsx");
    assert_eq!(geo.read, [p("conf.d/data/geo"), p("shared")]);
    assert!(geo.write.is_empty());
    assert_eq!(geo.connect, ["tiles.example.com"]);

    let shop = by_path("shop.nvsx");
    assert_eq!(shop.write, [p("cache")]);
    assert!(shop.read.is_empty());
}

/// `rule:config/the-extension-set-is-in-every-unit-key` keys on the pins alone. A grant changes
/// what a guest may reach at a call, not what any unit compiles to, so a reload that narrows one
/// keeps every compiled unit.
#[test]
fn an_extension_grant_does_not_change_the_env_hash() {
    let with = |grants: &str| {
        let tree = resolved(&[(
            "nvs.toml",
            &format!("[[extension]]\npath = \"geo.nvsx\"\nsha256 = \"{PIN}\"\n{grants}"),
        )]);
        env_hash(&tree.config)
    };

    let none = with("");
    assert_eq!(
        with("grants = { read = [\"data/geo/\"], connect = [\"tiles.example.com\"] }\n"),
        none
    );
    assert_eq!(with("grants = { write = [\"cache\"] }\n"), none);

    let other_pin = resolved(&[(
        "nvs.toml",
        &format!(
            "[[extension]]\npath = \"geo.nvsx\"\nsha256 = \"{}\"\n",
            PIN.replace('9', "8")
        ),
    )]);
    assert_ne!(
        env_hash(&other_pin.config),
        none,
        "the pin still moves it, so the equality above is not a hash that ignores the entry",
    );
}

/// The ceiling is a size in `[limits] memory`'s unit, so a value that is not one is that unit's
/// own refusal.
#[test]
fn a_ceiling_that_is_not_a_size_is_refused() {
    let mut sources = SourceMap::new();
    let err = resolve(
        &Roots::Files(vec![p("nvs.toml")]),
        &mut sources,
        &Fake::with(&[(
            "nvs.toml",
            &format!(
                "[[extension]]\npath = \"geo.nvsx\"\nsha256 = \"{PIN}\"\nmemory = \"12 bananas\"\n"
            ),
        )]),
    )
    .expect_err("a ceiling that is not a size should be refused");

    assert_eq!(err.code, Some(code::E_BAD_DIRECTIVE));
    assert!(
        err.message.contains("extension.0.memory"),
        "{}",
        err.message
    );
}
