//! The completion files are found, read, merged and read again only when they change, and what is
//! wrong with one is found (`rule:ide/completion-files-offer-values-at-named-parameters`).
//!
//! Each test writes a workspace of its own under the repository's `.agent-tmp/` and deletes it when
//! it ends.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use nvs_lsp::completion_files::{CompletionFiles, FindingKind, Reload};

/// A scratch workspace that deletes itself.
struct Workspace {
    root: PathBuf,
}

impl Workspace {
    fn new(name: &str) -> Self {
        let root = nvs_repo::path(".agent-tmp")
            .join(format!("completion-files-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("a scratch directory");
        Self { root }
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.root.join(name);
        fs::create_dir_all(path.parent().expect("a file sits in a folder")).expect("a folder");
        fs::write(&path, text).expect("a writable scratch file");
        path
    }

    fn load(&self) -> CompletionFiles {
        CompletionFiles::load(std::slice::from_ref(&self.root))
    }

    /// Every file loaded, relative to the root and written with `/`.
    fn loaded(&self, files: &CompletionFiles) -> Vec<String> {
        files.files().map(|path| self.relative(path)).collect()
    }

    fn relative(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .expect("under the root")
            .to_string_lossy()
            .replace('\\', "/")
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

const ICONS: &str = r#"{
  "sets": { "icons": ["home", { "value": "arrow-left", "label": "Arrow left", "kind": "constant" }] },
  "parameters": [{ "method": "App\\Ui\\Icon::render", "parameter": "name", "set": "icons" }]
}"#;

/// The values offered at `App\Ui\Icon::render`'s `$name`, in order, from every attachment.
fn icon_values(files: &CompletionFiles) -> Vec<String> {
    files
        .attachments("App\\Ui\\Icon", "render", "name")
        .iter()
        .flat_map(|attachment| attachment.values.iter().map(|value| value.value.clone()))
        .collect()
}

/// The kinds of finding shown for the file at `name`.
fn kinds(workspace: &Workspace, files: &CompletionFiles, name: &str) -> Vec<FindingKind> {
    files
        .findings()
        .filter(|(path, _)| workspace.relative(path) == name)
        .flat_map(|(_, findings)| findings.into_iter().map(|finding| finding.kind))
        .collect()
}

#[test]
fn a_completion_file_is_found_under_every_novis_folder_and_in_vendor() {
    let workspace = Workspace::new("found");
    workspace.write(".novis/completion/icons.json", ICONS);
    workspace.write(
        "modules/blog/.novis/completion/deep/more.json",
        r#"{ "sets": {} }"#,
    );
    workspace.write(
        "vendor/shop/ui/.novis/completion/shop.json",
        r#"{ "sets": {} }"#,
    );
    // Folders the walk skips, and a `.novis` folder of another kind.
    workspace.write(".git/.novis/completion/hidden.json", "{}");
    workspace.write("node_modules/a/.novis/completion/a.json", "{}");
    workspace.write("target/.novis/completion/t.json", "{}");
    workspace.write(".idea/.novis/completion/i.json", "{}");
    workspace.write(".novis/other/x.json", "{}");

    let files = workspace.load();
    assert_eq!(
        workspace.loaded(&files),
        [
            ".novis/completion/icons.json",
            "modules/blog/.novis/completion/deep/more.json",
            "vendor/shop/ui/.novis/completion/shop.json",
        ]
    );
    assert_eq!(icon_values(&files), ["home", "arrow-left"]);
}

#[test]
fn a_json_file_outside_a_completion_folder_is_not_read() {
    let workspace = Workspace::new("outside");
    workspace.write("icons.json", ICONS);
    workspace.write("config/completion/icons.json", ICONS);
    workspace.write(".novis/icons.json", ICONS);
    workspace.write(".novis/completion/README.md", "Values for the icon picker.");

    let mut files = workspace.load();
    assert!(workspace.loaded(&files).is_empty());
    assert!(icon_values(&files).is_empty());
    // A watcher event for one of them is ignored as well.
    assert_eq!(
        files.refresh(&workspace.root.join(".novis/icons.json")),
        Reload::Ignored
    );
    assert_eq!(
        files.refresh(&workspace.root.join(".novis/completion/README.md")),
        Reload::Ignored
    );
}

#[test]
fn two_files_for_one_parameter_merge_and_the_first_value_is_kept() {
    let workspace = Workspace::new("merge");
    workspace.write(
        ".novis/completion/a.json",
        r#"{ "parameters": [{ "method": "App\\Ui\\Icon::render", "parameter": "name",
              "values": [{ "value": "home", "title": "from a" }, "star"] }] }"#,
    );
    workspace.write(
        ".novis/completion/b.json",
        r#"{ "parameters": [{ "method": "\\App\\Ui\\Icon::render", "parameter": "name",
              "values": [{ "value": "home", "title": "from b" }, "user"] }] }"#,
    );

    let files = workspace.load();
    let attachments = files.attachments("App\\Ui\\Icon", "render", "name");
    assert_eq!(attachments.len(), 2);
    assert_eq!(icon_values(&files), ["home", "star", "home", "user"]);

    // The two files' sets merge the same way, the first value read kept.
    workspace.write(
        ".novis/completion/c.json",
        r#"{ "sets": { "icons": [{ "value": "home", "title": "from c" }] } }"#,
    );
    workspace.write(
        ".novis/completion/d.json",
        r#"{ "sets": { "icons": [{ "value": "home", "title": "from d" }, "star"] } }"#,
    );
    let files = workspace.load();
    let set = files.set("icons").expect("the set is defined");
    let values: Vec<(&str, Option<&str>)> = set
        .iter()
        .map(|value| (value.value.as_str(), value.title.as_deref()))
        .collect();
    assert_eq!(values, [("home", Some("from c")), ("star", None)]);
}

#[test]
fn a_set_defined_in_one_file_attaches_in_another() {
    let workspace = Workspace::new("across");
    workspace.write(
        ".novis/completion/generated/icons.json",
        r#"{ "sets": { "icons": ["home", "star"] } }"#,
    );
    workspace.write(
        "app/.novis/completion/attach.json",
        r#"{ "parameters": [{ "method": "App\\Ui\\Icon::render", "parameter": "name",
              "set": "icons", "values": ["user", "home"] }] }"#,
    );

    let files = workspace.load();
    assert_eq!(icon_values(&files), ["home", "star", "user"]);
    assert!(kinds(&workspace, &files, "app/.novis/completion/attach.json").is_empty());
}

#[test]
fn a_changed_completion_file_is_read_again_and_an_unchanged_one_is_not() {
    let workspace = Workspace::new("reload");
    let path = workspace.write(".novis/completion/icons.json", ICONS);
    let mut files = workspace.load();
    assert_eq!(files.refresh(&path), Reload::Unchanged);

    // The same size, so only the modification time says it changed.
    let changed = ICONS.replace("\"home\"", "\"star\"");
    fs::write(&path, &changed).expect("a writable scratch file");
    let later = SystemTime::now() + Duration::from_secs(5);
    fs::File::options()
        .write(true)
        .open(&path)
        .and_then(|file| file.set_modified(later))
        .expect("a settable modification time");
    assert_eq!(files.refresh(&path), Reload::Read);
    assert_eq!(icon_values(&files), ["star", "arrow-left"]);
    assert_eq!(files.refresh(&path), Reload::Unchanged);

    // A new file is read, and a deleted one is dropped.
    let added = workspace.write(
        ".novis/completion/more.json",
        r#"{ "parameters": [{ "method": "App\\Ui\\Icon::render", "parameter": "name", "values": ["user"] }] }"#,
    );
    assert_eq!(files.refresh(&added), Reload::Read);
    assert_eq!(icon_values(&files), ["star", "arrow-left", "user"]);
    fs::remove_file(&path).expect("a deletable scratch file");
    assert_eq!(files.refresh(&path), Reload::Dropped);
    assert_eq!(icon_values(&files), ["user"]);
    assert_eq!(files.refresh(&path), Reload::Unchanged);
}

#[test]
fn a_completion_file_with_an_unknown_field_contributes_nothing_and_is_reported() {
    let workspace = Workspace::new("unknown-field");
    workspace.write(
        ".novis/completion/icons.json",
        r#"{ "sets": { "icons": ["home", { "value": "star", "command": "run" }] },
             "parameters": [{ "method": "App\\Ui\\Icon::render", "parameter": "name", "set": "icons" }] }"#,
    );

    let files = workspace.load();
    assert!(icon_values(&files).is_empty());
    let (_, findings) = files.findings().next().expect("the file is reported");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].kind, FindingKind::Invalid);
    assert!(
        findings[0].message.contains("`command`"),
        "{}",
        findings[0].message
    );
    assert!(
        findings[0].message.contains("`documentation`"),
        "{}",
        findings[0].message
    );
}

#[test]
fn a_completion_file_that_is_not_json_contributes_nothing_and_is_reported() {
    let workspace = Workspace::new("not-json");
    workspace.write(".novis/completion/icons.json", ICONS);
    workspace.write(
        ".novis/completion/broken.json",
        "{ \"sets\": { \"more\": [\"home\",, ] } }",
    );

    let files = workspace.load();
    assert_eq!(icon_values(&files), ["home", "arrow-left"]);
    assert!(files.set("more").is_none());
    assert_eq!(
        kinds(&workspace, &files, ".novis/completion/broken.json"),
        [FindingKind::Invalid]
    );
    assert!(kinds(&workspace, &files, ".novis/completion/icons.json").is_empty());
}

#[test]
fn an_attachment_to_a_set_no_file_defines_is_reported() {
    let workspace = Workspace::new("missing-set");
    workspace.write(
        ".novis/completion/icons.json",
        r#"{ "parameters": [{ "method": "App\\Ui\\Icon::render", "parameter": "name",
              "set": "glyphs", "values": ["home"] }] }"#,
    );

    let files = workspace.load();
    assert_eq!(icon_values(&files), ["home"]);
    assert_eq!(
        kinds(&workspace, &files, ".novis/completion/icons.json"),
        [FindingKind::MissingSet]
    );
}

#[test]
fn a_completion_file_under_vendor_is_not_reported() {
    let workspace = Workspace::new("vendor");
    workspace.write("vendor/shop/.novis/completion/broken.json", "not json");
    workspace.write(
        "vendor/shop/.novis/completion/sets.json",
        r#"{ "parameters": [{ "method": "Shop\\Cart::add", "parameter": "sku", "set": "skus" }] }"#,
    );
    workspace.write(".novis/completion/broken.json", "not json");

    let files = workspace.load();
    let reported: Vec<String> = files
        .findings()
        .filter(|(_, findings)| !findings.is_empty())
        .map(|(path, _)| workspace.relative(path))
        .collect();
    assert_eq!(reported, [".novis/completion/broken.json"]);
}

/// One attachment as a test reads it back: each value with its separator, its `when`'s strings, and
/// its `strict`.
type Loaded<'a> = (Vec<(&'a str, Option<char>)>, Option<Vec<String>>, bool);

#[test]
fn a_list_written_with_a_separator_loads_its_whole_values() {
    let workspace = Workspace::new("separator");
    workspace.write(
        ".novis/completion/keys.json",
        r#"{ "sets": { "shop-keys": { "separator": ".", "values": ["shop.cart.title", "shop.checkout.title"] } },
             "parameters": [
               { "method": "App\\I18n\\Text::translate", "parameter": "key", "set": "shop-keys",
                 "when": { "parameter": "domain", "equals": "shop" }, "strict": true },
               { "method": "App\\I18n\\Text::translate", "parameter": "key",
                 "values": { "separator": "/", "values": ["common/yes"] },
                 "when": { "parameter": "domain", "equals": ["shop", "blog"] } },
               { "method": "App\\I18n\\Text::translate", "parameter": "key", "values": ["ok"] }
             ] }"#,
    );

    let files = workspace.load();
    assert!(kinds(&workspace, &files, ".novis/completion/keys.json").is_empty());
    let attachments = files.attachments("App\\I18n\\Text", "translate", "key");
    let read: Vec<Loaded<'_>> = attachments
        .iter()
        .map(|attachment| {
            (
                attachment
                    .values
                    .iter()
                    .map(|value| (value.value.as_str(), value.separator))
                    .collect(),
                attachment.when.as_ref().map(|when| {
                    assert_eq!(when.parameter, "domain");
                    when.equals.clone()
                }),
                attachment.strict,
            )
        })
        .collect();
    assert_eq!(
        read,
        [
            (
                vec![
                    ("shop.cart.title", Some('.')),
                    ("shop.checkout.title", Some('.'))
                ],
                Some(vec!["shop".to_owned()]),
                true,
            ),
            (
                vec![("common/yes", Some('/'))],
                Some(vec!["shop".to_owned(), "blog".to_owned()]),
                false,
            ),
            (vec![("ok", None)], None, false),
        ]
    );
}

#[test]
fn a_list_with_an_unknown_separator_contributes_nothing_and_is_reported() {
    let workspace = Workspace::new("bad-separator");
    workspace.write(
        ".novis/completion/keys.json",
        r#"{ "parameters": [{ "method": "App\\I18n\\Text::translate", "parameter": "key",
              "values": { "separator": ",", "values": ["shop,cart"] } }] }"#,
    );

    let files = workspace.load();
    assert!(
        files
            .attachments("App\\I18n\\Text", "translate", "key")
            .is_empty()
    );
    let (_, findings) = files.findings().next().expect("the file is reported");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].kind, FindingKind::Invalid);
    assert!(
        findings[0].message.contains("`,`"),
        "{}",
        findings[0].message
    );
}

#[test]
fn a_value_location_whose_file_does_not_exist_is_reported() {
    let workspace = Workspace::new("location");
    workspace.write(".novis/completion/svg/home.svg", "<svg/>");
    let path = workspace.write(
        ".novis/completion/icons.json",
        r#"{ "sets": { "icons": [
              { "value": "home", "location": { "file": "svg/home.svg", "line": 1 } },
              { "value": "star", "title": "A star", "location": { "file": "svg/star.svg", "line": 3 } }
            ] } }"#,
    );

    let files = workspace.load();
    let set = files.set("icons").expect("the set is defined");
    let (file, line) = set[0].location.clone().expect("home's file exists");
    assert_eq!(file, path.parent().expect("a folder").join("svg/home.svg"));
    assert_eq!(line.get(), 1);
    // The value keeps everything but its location.
    assert_eq!(set[1].title.as_deref(), Some("A star"));
    assert!(set[1].location.is_none());

    let text = fs::read_to_string(&path).expect("the file");
    let (_, findings) = files.findings().next().expect("the file is reported");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].kind, FindingKind::MissingLocation);
    let (start, end) = findings[0].span;
    assert!(text[start..end].starts_with("{ \"value\": \"star\""));
}

#[test]
fn a_replacement_on_a_value_that_is_not_deprecated_is_reported() {
    let workspace = Workspace::new("replacement");
    workspace.write(
        ".novis/completion/icons.json",
        r#"{ "sets": { "icons": [
              { "value": "trash", "deprecated": true, "replacement": "delete" },
              { "value": "bin", "replacement": "delete" },
              "delete"
            ] } }"#,
    );

    let files = workspace.load();
    let set = files.set("icons").expect("the set is defined");
    assert_eq!(set[0].replacement.as_deref(), Some("delete"));
    assert!(set[0].deprecated);
    assert_eq!(set[1].replacement, None);
    assert_eq!(
        kinds(&workspace, &files, ".novis/completion/icons.json"),
        [FindingKind::ReplacementNotDeprecated]
    );
}
