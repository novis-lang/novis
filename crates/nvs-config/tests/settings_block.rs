//! `rule:config/settings-and-secrets-are-a-programs-own-values`: `[settings]` and `[secrets]`, and
//! their `[app.settings]` and `[app.secrets]` copies, are typed, merged, refused where malformed and
//! redacted in the dump.
//!
//! Every case runs against an in-memory [`Files`], as `tests/snapshot.rs` does, so a secret file's
//! content is what the case wrote and an `[[app]]` root resolves without a real directory.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use nvs_config::audit::Audit;
use nvs_config::resolve::{Files, Resolved, Roots, resolve};
use nvs_config::snapshot::Snapshot;
use nvs_config::trust::Untrusted;
use nvs_diagnostics::{Diagnostic, SourceMap, code};

/// A path written with `/` separators, as a path the host spells its own way.
fn p(path: &str) -> PathBuf {
    path.split('/').collect()
}

/// The filesystem the cases describe: a name-to-text map with directories implied by it.
#[derive(Default)]
struct Fake {
    files: BTreeMap<PathBuf, String>,
}

impl Fake {
    fn with(entries: &[(&str, &str)]) -> Self {
        Self {
            files: entries
                .iter()
                .map(|(path, text)| (p(path), (*text).to_string()))
                .collect(),
        }
    }
}

impl Files for Fake {
    fn trust(&self, path: &Path) -> Result<PathBuf, Untrusted> {
        if self.exists(path) {
            Ok(path.to_path_buf())
        } else {
            Err(Untrusted::Unreadable("no such file".to_string()))
        }
    }

    fn canonical(&self, path: &Path) -> Result<PathBuf, String> {
        let mut out = PathBuf::new();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    out.pop();
                }
                other => out.push(other.as_os_str()),
            }
        }
        if self.exists(&out) {
            Ok(out)
        } else {
            Err("no such file or directory".to_string())
        }
    }

    fn read(&self, path: &Path) -> Result<String, String> {
        self.files
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
            .files
            .keys()
            .filter(|path| path.parent() == Some(dir))
            .cloned()
            .collect())
    }

    fn exists(&self, path: &Path) -> bool {
        self.files.contains_key(path) || self.files.keys().any(|file| file.starts_with(path))
    }
}

/// Resolves `nvs.toml` in `fs`, panicking with the refusal when it does not.
fn tree_of(fs: &Fake) -> Resolved {
    let mut sources = SourceMap::new();
    resolve(&Roots::Files(vec![p("nvs.toml")]), &mut sources, fs)
        .unwrap_or_else(|err| panic!("refused: {} [{:?}]", err.message, err.notes))
}

/// The refusal resolving `nvs.toml` produces, with the line its label points at, counted from 1.
fn refusal(fs: &Fake) -> (Diagnostic, Option<usize>) {
    let mut sources = SourceMap::new();
    let err = resolve(&Roots::Files(vec![p("nvs.toml")]), &mut sources, fs)
        .expect_err("this tree should have been refused");
    let line = err.labels.first().map(|label| {
        let text = sources.file(label.span.file).text();
        let start = usize::try_from(label.span.start).expect("a span fits a usize");
        text[..start].matches('\n').count() + 1
    });
    (err, line)
}

/// The snapshot `entry` gets out of `fs`, panicking with the refusal when there is none.
fn snapshot_of(fs: &Fake, entry: &str) -> Arc<Snapshot> {
    Snapshot::build(&tree_of(fs), &p(entry), fs)
        .unwrap_or_else(|err| panic!("refused: {} [{:?}]", err.message, err.notes))
}

/// The value at a dotted key under `[settings]` in a merged table.
fn setting<'a>(table: &'a toml::Table, dotted: &str) -> Option<&'a toml::Value> {
    let mut at = table.get("settings")?;
    for segment in dotted.split('.') {
        at = at.as_table()?.get(segment)?;
    }
    Some(at)
}

#[test]
fn a_settings_value_keeps_its_toml_type() {
    let fs = Fake::with(&[(
        "nvs.toml",
        "[settings]\n\
         title = \"Shop\"\n\
         page_size = 20\n\
         ratio = 0.5\n\
         beta = true\n\
         sizes = [10, 20, 50]\n\
         \n\
         [settings.api]\n\
         base = \"https://api.example.com\"\n",
    )]);
    let resolved = tree_of(&fs);
    let table = &resolved.table;
    assert_eq!(
        setting(table, "title"),
        Some(&toml::Value::String("Shop".into()))
    );
    assert_eq!(setting(table, "page_size"), Some(&toml::Value::Integer(20)));
    assert_eq!(setting(table, "ratio"), Some(&toml::Value::Float(0.5)));
    assert_eq!(setting(table, "beta"), Some(&toml::Value::Boolean(true)));
    assert_eq!(
        setting(table, "sizes"),
        Some(&toml::Value::Array(vec![10.into(), 20.into(), 50.into()]))
    );
    assert_eq!(
        setting(table, "api.base"),
        Some(&toml::Value::String("https://api.example.com".into()))
    );
    let typed = resolved
        .config
        .settings
        .as_ref()
        .expect("the typed tree carries the block");
    assert_eq!(typed.get("page_size"), Some(&toml::Value::Integer(20)));
}

/// The global block in `nvs.toml`, and an `[[app]]` block in a file it includes, so the two origins
/// are two files.
fn shop_with_apps() -> Fake {
    Fake::with(&[
        (
            "nvs.toml",
            "[[include]]\npath = \"apps.toml\"\n\n\
             [settings]\npage_size = 10\ntitle = \"Store\"\n\n\
             [secrets]\napi_key_file = \"run/api_key\"\n",
        ),
        (
            "apps.toml",
            "[[app]]\nroot = \"srv/shop\"\n\n\
             [app.settings]\npage_size = 50\n\n\
             [app.secrets]\napi_key = \"shop-key\"\n",
        ),
        ("run/api_key", "global-key\n"),
        ("srv/shop/index.nvs", ""),
        ("srv/blog/index.nvs", ""),
    ])
}

#[test]
fn an_app_settings_value_overrides_the_global_one_and_both_origins_are_recorded() {
    let fs = shop_with_apps();
    let shop = snapshot_of(&fs, "srv/shop/index.nvs");
    assert_eq!(
        setting(&shop.table, "page_size"),
        Some(&toml::Value::Integer(50))
    );
    assert_eq!(
        setting(&shop.table, "title"),
        Some(&toml::Value::String("Store".into())),
        "a key the block does not write keeps the global value",
    );
    let record = shop
        .overrides
        .iter()
        .find(|record| record.key == "settings.page_size")
        .expect("the override is recorded");
    assert_eq!(record.replaced.path, p("nvs.toml"));
    assert_eq!(record.winner.path, p("apps.toml"));

    // The block's inline secret replaces the global file, and the file's name leaves the table.
    let secret = shop
        .secrets
        .get("secrets.api_key")
        .expect("the secret is in force");
    assert_eq!(secret.value, "shop-key");
    assert!(
        !shop.secrets.keys().any(|key| key.starts_with("app.")),
        "{:?}",
        shop.secrets.keys()
    );
    let record = shop
        .overrides
        .iter()
        .find(|record| record.key == "secrets.api_key")
        .expect("the secret's override is recorded");
    assert_eq!(record.replaced.path, p("nvs.toml"));
    assert_eq!(record.winner.path, p("apps.toml"));
    assert_eq!(
        setting(&shop.table, "api_key_file"),
        None,
        "{:?}",
        shop.table
    );
    let in_secrets = shop
        .table
        .get("secrets")
        .and_then(|block| block.get("api_key_file"));
    assert_eq!(in_secrets, None);

    let blog = snapshot_of(&fs, "srv/blog/index.nvs");
    assert_eq!(
        setting(&blog.table, "page_size"),
        Some(&toml::Value::Integer(10))
    );
    assert_eq!(blog.secrets["secrets.api_key"].value, "global-key");
}

#[test]
fn a_later_file_overrides_a_setting_and_both_origins_are_recorded() {
    let fs = Fake::with(&[
        (
            "nvs.toml",
            "[[include]]\npath = \"machine.toml\"\n\n[settings]\npage_size = 10\n",
        ),
        ("machine.toml", "[settings]\npage_size = 25\n"),
    ]);
    let resolved = tree_of(&fs);
    assert_eq!(
        setting(&resolved.table, "page_size"),
        Some(&toml::Value::Integer(25))
    );
    let record = resolved
        .overrides
        .iter()
        .find(|record| record.key == "settings.page_size")
        .expect("the override is recorded");
    assert_eq!(record.replaced.path, p("nvs.toml"));
    assert_eq!(record.winner.path, p("machine.toml"));
}

#[test]
fn a_settings_key_that_is_not_a_plain_name_is_refused() {
    for (written, line) in [
        ("[settings]\ntitle = \"x\"\n\"api.base\" = 1\n", 3),
        ("[settings]\n2fa = true\n", 2),
        ("[settings.api]\n\"base url\" = \"x\"\n", 2),
        ("[secrets]\n\"db-password\" = \"x\"\n", 2),
        (
            "[[app]]\nroot = \"srv\"\n\n[app.settings]\n\"a.b\" = 1\n",
            5,
        ),
    ] {
        let fs = Fake::with(&[("nvs.toml", written), ("srv/index.nvs", "")]);
        let (err, at) = refusal(&fs);
        assert_eq!(err.code, Some(code::E_BAD_DIRECTIVE), "{written}: {err:?}");
        assert!(err.message.contains("not a valid name"), "{}", err.message);
        assert_eq!(at, Some(line), "{written}: {err:?}");
    }
}

#[test]
fn a_datetime_in_settings_is_refused() {
    for (written, line) in [
        ("[settings]\nlaunch = 2026-01-01\n", 2),
        ("[settings]\ntimes = [\"a\", 09:30:00]\n", 2),
        (
            "[[app]]\nroot = \"srv\"\n\n[app.settings.sale]\nends = 2026-12-24T18:00:00Z\n",
            5,
        ),
    ] {
        let fs = Fake::with(&[("nvs.toml", written), ("srv/index.nvs", "")]);
        let (err, at) = refusal(&fs);
        assert_eq!(err.code, Some(code::E_BAD_DIRECTIVE), "{written}: {err:?}");
        assert!(err.message.contains("date or a time"), "{}", err.message);
        assert_eq!(at, Some(line), "{written}: {err:?}");
    }
    let fs = Fake::with(&[("nvs.toml", "[settings]\nlaunch = 2026-01-01\n")]);
    let (err, _) = refusal(&fs);
    assert!(
        err.notes.iter().any(|note| note.contains("\"2026-01-01\"")),
        "the help shows the value as a string: {err:?}"
    );
}

#[test]
fn an_unknown_block_beside_settings_is_still_refused() {
    let fs = Fake::with(&[(
        "nvs.toml",
        "[settings]\npage_size = 10\n\n[setings]\npage_size = 20\n",
    )]);
    let (err, at) = refusal(&fs);
    assert_eq!(err.code, Some(code::E_BAD_DIRECTIVE), "{err:?}");
    assert_eq!(at, Some(4), "{err:?}");
}

#[test]
fn a_secret_is_read_from_its_file_and_never_enters_the_table() {
    let fs = Fake::with(&[
        (
            "nvs.toml",
            "[secrets]\napi_key_file = \"run/api_key\"\ntoken = \"inline-token\"\n",
        ),
        ("run/api_key", "file-key\n"),
    ]);
    let resolved = tree_of(&fs);
    let api_key = &resolved.secrets["secrets.api_key"];
    assert_eq!(api_key.value, "file-key", "one trailing newline is removed");
    assert_eq!(api_key.file, p("run/api_key"));
    let token = &resolved.secrets["secrets.token"];
    assert_eq!(token.value, "inline-token");
    assert_eq!(token.file, p("nvs.toml"));

    let block = resolved.table["secrets"]
        .as_table()
        .expect("the `_file` half stays in the table");
    assert_eq!(
        block.keys().collect::<Vec<_>>(),
        ["api_key_file"],
        "only the file's name is in the table"
    );
    let typed = resolved
        .config
        .secrets
        .as_ref()
        .expect("the block is typed");
    assert!(typed.get("token").is_none(), "{typed:?}");
    let dumped = toml::to_string(&resolved.table).expect("the table serializes");
    assert!(!dumped.contains("inline-token"), "{dumped}");
    assert!(!dumped.contains("file-key"), "{dumped}");
}

#[test]
fn a_secret_with_both_a_value_and_a_file_is_refused() {
    let fs = Fake::with(&[
        (
            "nvs.toml",
            "[secrets]\ntoken = \"x\"\ntoken_file = \"run/token\"\n",
        ),
        ("run/token", "y\n"),
    ]);
    let (err, _) = refusal(&fs);
    assert_eq!(err.code, Some(code::E_BAD_SECRET_FILE), "{err:?}");
    assert!(err.message.contains("`[secrets]`"), "{}", err.message);

    // One half in each file is the same two sources for one value.
    let fs = Fake::with(&[
        (
            "nvs.toml",
            "[[include]]\npath = \"machine.toml\"\n\n[secrets]\ntoken = \"x\"\n",
        ),
        ("machine.toml", "[secrets]\ntoken_file = \"run/token\"\n"),
        ("run/token", "y\n"),
    ]);
    let (err, _) = refusal(&fs);
    assert_eq!(err.code, Some(code::E_BAD_SECRET_FILE), "{err:?}");
}

#[test]
fn a_secret_that_is_not_a_string_is_refused() {
    for (written, line) in [
        ("[secrets]\npin = 1234\n", 2),
        ("[secrets.mail]\npassword = \"x\"\n", 1),
        ("[secrets]\nkeys = [\"a\", \"b\"]\n", 2),
        (
            "[[app]]\nroot = \"srv\"\n\n[app.secrets]\nenabled = true\n",
            5,
        ),
    ] {
        let fs = Fake::with(&[("nvs.toml", written), ("srv/index.nvs", "")]);
        let (err, at) = refusal(&fs);
        assert_eq!(err.code, Some(code::E_BAD_DIRECTIVE), "{written}: {err:?}");
        assert!(err.message.contains("must be a string"), "{}", err.message);
        assert_eq!(at, Some(line), "{written}: {err:?}");
    }
}

#[test]
fn dump_renders_every_secret_as_secret() {
    let fs = Fake::with(&[
        (
            "nvs.toml",
            "[settings]\npage_size = 10\n\n\
             [secrets]\napi_key_file = \"run/api_key\"\ntoken = \"inline-token\"\n\n\
             [[app]]\nroot = \"srv\"\n\n[app.secrets]\nshop_key = \"shop-secret\"\n",
        ),
        ("run/api_key", "file-key\n"),
        ("srv/index.nvs", ""),
    ]);
    let resolved = tree_of(&fs);
    let dump = Audit::of_files(&resolved).render(true);
    for value in ["inline-token", "file-key", "shop-secret"] {
        assert!(!dump.contains(value), "the dump prints `{value}`:\n{dump}");
    }
    let row = |key: &str| {
        dump.lines()
            .find(|line| line.starts_with(&format!("{key} ")))
            .unwrap_or_else(|| panic!("no `{key}` row:\n{dump}"))
            .to_string()
    };
    assert!(row("settings.page_size").contains("= 10"), "{dump}");
    let api_key = row("secrets.api_key");
    assert!(api_key.contains("<secret>"), "{dump}");
    assert!(
        api_key.contains(&p("run/api_key").display().to_string()),
        "the row names the file the value came from: {api_key}"
    );
    let token = row("secrets.token");
    assert!(token.contains("<secret>"), "{dump}");
    assert!(token.contains("nvs.toml"), "{token}");
    assert!(row("app.0.secrets.shop_key").contains("<secret>"), "{dump}");
}
