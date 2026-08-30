//! ADR 0104 §§ 1-2: an entry file belongs to every `[[app]]` block whose key covers it.
//!
//! Every case runs against an in-memory [`Files`] for `tests/resolve.rs`'s reason, plus one of its
//! own: § 1's whole claim is that the comparison happens on **canonical** paths, so the cases have
//! to be able to plant a symlink and a `..` and watch them fail to match. A real directory could
//! hold the `..`, but a symlink is a privileged operation on Windows and the case would be skipped
//! on the platform where the trust boundary is hardest — so this reader resolves both, and
//! `crates/nvs-config/src/trust.rs` owns what the real one does.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use nvs_config::Setting;
use nvs_config::app::{layer, matching};
use nvs_config::resolve::{Files, Override, Resolved, Roots, resolve};
use nvs_config::tree::App;
use nvs_config::trust::Untrusted;
use nvs_diagnostics::{Diagnostic, SourceMap, code};

/// A path written the way an ADR writes one, as a path the host spells its own way.
fn p(path: &str) -> PathBuf {
    path.split('/').collect()
}

/// The filesystem the cases describe: a name-to-text map with directories implied by it, plus the
/// symlinks § 1 exists to defeat.
#[derive(Default)]
struct Fake {
    files: BTreeMap<PathBuf, String>,
    links: BTreeMap<PathBuf, PathBuf>,
}

impl Fake {
    fn with(entries: &[(&str, &str)]) -> Self {
        Self {
            files: entries
                .iter()
                .map(|(path, text)| (p(path), (*text).to_string()))
                .collect(),
            links: BTreeMap::new(),
        }
    }

    /// `link` is another name for `target`, resolved the moment a path walks through it.
    fn linking(mut self, link: &str, target: &str) -> Self {
        self.links.insert(p(link), p(target));
        self
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

    /// A real `canonicalize`: `.` dropped, `..` popped, and every prefix that names a link replaced
    /// by its target before the walk goes on. A path that reaches nothing is an error, because that
    /// is what the caller has to distinguish from a path that reaches somewhere unexpected.
    fn canonical(&self, path: &Path) -> Result<PathBuf, String> {
        let mut out = PathBuf::new();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    out.pop();
                }
                other => {
                    out.push(other.as_os_str());
                    if let Some(target) = self.links.get(&out) {
                        out.clone_from(target);
                    }
                }
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

    /// Nothing is exposed: § 7's advisory is about a secret file and no case here has one.
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

/// Resolves `root` in `fs`, panicking with the refusal when it does not.
fn tree_of(fs: &Fake, root: &str) -> Resolved {
    let mut sources = SourceMap::new();
    resolve(&Roots::Files(vec![p(root)]), &mut sources, fs)
        .unwrap_or_else(|err| panic!("refused: {} [{:?}]", err.message, err.notes))
}

/// The refusal resolving `root` produces, panicking when it is accepted instead.
fn refusal(fs: &Fake, root: &str) -> Diagnostic {
    let mut sources = SourceMap::new();
    resolve(&Roots::Files(vec![p(root)]), &mut sources, fs)
        .expect_err("this tree should have been refused")
}

/// The `origin` key of every block matching `entry`, in the order they were returned — one short
/// label per block, so a case asserts the *sequence* § 2 promises rather than a field at a time.
fn blocks_for(fs: &Fake, resolved: &Resolved, entry: &str) -> Vec<String> {
    matching(&resolved.config.app, &p(entry), fs)
        .unwrap_or_else(|err| panic!("refused: {}", err.message))
        .iter()
        .map(|index| {
            resolved.config.app[*index]
                .origin
                .clone()
                .unwrap_or_default()
        })
        .collect()
}

/// ADR 0104 § 1's example, as the tree every matching case reads: a host-wide block, a shop
/// beneath it, and one entry file inside the shop.
const THREE_BLOCKS: &str = "\
[[app]]
root = \"srv/www\"
origin = \"host\"

[[app]]
root = \"srv/www/shop\"
origin = \"shop\"

[[app]]
entry = \"srv/www/shop/bin/import.nvs\"
origin = \"import\"
";

/// Enough of a tree for those three blocks to be keyed on something that exists.
fn shop() -> Fake {
    Fake::with(&[
        ("nvs.toml", THREE_BLOCKS),
        ("srv/www/index.nvs", ""),
        ("srv/www/shop/index.nvs", ""),
        ("srv/www/shop/bin/import.nvs", ""),
        ("srv/www/shopfront/index.nvs", ""),
        ("srv/other/x.nvs", ""),
    ])
}

/// § 1's prefix test is on **path-component boundaries**, which is the difference between a
/// directory and a string: `shopfront` starts with `shop` and is not inside it. Both halves are
/// asserted together because a member that compared strings passes either one alone.
#[test]
fn a_root_matches_on_component_boundaries_and_never_on_part_of_a_name() {
    let fs = shop();
    let resolved = tree_of(&fs, "nvs.toml");

    assert_eq!(
        blocks_for(&fs, &resolved, "srv/www/shop/index.nvs"),
        ["host", "shop"],
    );
    assert_eq!(
        blocks_for(&fs, &resolved, "srv/www/shopfront/index.nvs"),
        ["host"],
        "`srv/www/shopfront` is not inside `srv/www/shop`, however the two names start",
    );
}

/// § 2's ordering, asserted as the whole sequence: least-specific first, and the `entry` block
/// last of all because its path is the longest of the three that match.
#[test]
fn every_matching_block_applies_least_specific_first() {
    let fs = shop();
    let resolved = tree_of(&fs, "nvs.toml");

    assert_eq!(
        blocks_for(&fs, &resolved, "srv/www/shop/bin/import.nvs"),
        ["host", "shop", "import"],
    );
}

/// § 2's last line: an entry file matched by no block gets the global configuration, which is the
/// ordinary case and not a refusal.
#[test]
fn an_entry_matched_by_no_block_is_not_an_error() {
    let fs = shop();
    let resolved = tree_of(&fs, "nvs.toml");

    assert!(blocks_for(&fs, &resolved, "srv/other/x.nvs").is_empty());
}

/// § 1's reason for canonicalizing, in the shape M6's acceptance names it: a path that *reaches*
/// an application's tree through `..` is not inside it, so it inherits none of its capabilities.
#[test]
fn an_entry_reaching_a_root_through_dotdot_does_not_match_it() {
    let fs = shop();
    let resolved = tree_of(&fs, "nvs.toml");

    assert_eq!(
        blocks_for(&fs, &resolved, "srv/www/shop/../../other/x.nvs"),
        Vec::<String>::new(),
        "the path canonicalizes to `srv/other/x.nvs`, which no block covers",
    );
}

/// The other half of the same rule, and the one an attacker writes: a symlink *planted inside* an
/// application's tree does not inherit that application's grants, because the name it resolves to
/// is what is compared.
#[test]
fn a_symlink_planted_inside_a_root_does_not_inherit_it() {
    let fs = shop().linking("srv/www/shop/back-door.nvs", "srv/other/x.nvs");
    let resolved = tree_of(&fs, "nvs.toml");

    assert_eq!(
        blocks_for(&fs, &resolved, "srv/www/shop/back-door.nvs"),
        Vec::<String>::new(),
        "the link resolves out of `srv/www/shop`, so the `shop` block does not cover it",
    );
}

/// ADR 0103 § 5 applies to both of § 1's keys, and the per-block origin is what makes it possible:
/// two files each write a relative `root` and each resolves against **its own** directory.
#[test]
fn a_relative_root_resolves_against_the_file_that_wrote_it() {
    let fs = Fake::with(&[
        (
            "etc/nvs.toml",
            "[[include]]\npath = \"conf.d/host.toml\"\n\n[[app]]\nroot = \"../srv/www\"\norigin = \"host\"\n",
        ),
        (
            "etc/conf.d/host.toml",
            "[[app]]\nroot = \"../../srv/www/shop\"\norigin = \"shop\"\n",
        ),
        ("srv/www/shop/index.nvs", ""),
    ]);
    let resolved = tree_of(&fs, "etc/nvs.toml");

    assert_eq!(
        blocks_for(&fs, &resolved, "srv/www/shop/index.nvs"),
        ["host", "shop"],
        "`../srv/www` is relative to `etc` and `../../srv/www/shop` to `etc/conf.d`",
    );
    assert_eq!(
        resolved.config.app[0].root.as_deref(),
        Some(p("srv/www").to_str().expect("a test path is UTF-8")),
        "the key is canonicalized in place, so a dump prints the path the match will use",
    );
}

/// § 1 gives a block one key. Both is a refusal rather than a precedence rule, because a block
/// naming a directory *and* a file has not said which of the two it means.
#[test]
fn a_block_naming_both_keys_is_refused() {
    let fs = Fake::with(&[
        (
            "nvs.toml",
            "[[app]]\nroot = \"srv/www\"\nentry = \"srv/www/index.nvs\"\n",
        ),
        ("srv/www/index.nvs", ""),
    ]);

    let refusal = refusal(&fs, "nvs.toml");
    assert_eq!(refusal.code, Some(code::E_BAD_APP_BLOCK));
    assert!(refusal.message.contains("both `root` and `entry`"));
}

/// And neither is a refusal for the opposite reason: there is no default application, so a block
/// with no key would carry directives nothing ever reads.
#[test]
fn a_block_naming_neither_key_is_refused() {
    let fs = Fake::with(&[("nvs.toml", "[[app]]\nmode = \"production\"\n")]);

    let refusal = refusal(&fs, "nvs.toml");
    assert_eq!(refusal.code, Some(code::E_BAD_APP_BLOCK));
    assert!(refusal.message.contains("neither `root` nor `entry`"));
}

/// § 2's duplicate, caught on the **canonical** path rather than on the spelling: two blocks that
/// reach one directory by different routes are as much a duplicate as two copies of one string,
/// and specificity has no order to separate them with.
#[test]
fn two_blocks_on_one_path_are_a_duplicate_however_they_are_spelled() {
    let fs = Fake::with(&[
        (
            "nvs.toml",
            "[[app]]\nroot = \"srv/www/shop\"\n\n[[app]]\nroot = \"srv/www/shop/../shop\"\n",
        ),
        ("srv/www/shop/index.nvs", ""),
    ]);

    let refusal = refusal(&fs, "nvs.toml");
    assert_eq!(refusal.code, Some(code::E_BAD_APP_BLOCK));
    assert!(
        refusal.message.contains("block 1") && refusal.message.contains("block 2"),
        "the refusal names both blocks, which have no other name: {}",
        refusal.message,
    );
}

/// A key naming nothing refuses the boot instead of matching nothing, and the module doc owns why:
/// a block that silently covers no file hands the applications it was written for the *global*
/// configuration, including when it was written to narrow them.
#[test]
fn a_key_naming_something_that_does_not_exist_refuses_the_boot() {
    let fs = Fake::with(&[("nvs.toml", "[[app]]\nroot = \"srv/typo\"\n")]);

    let refusal = refusal(&fs, "nvs.toml");
    assert_eq!(refusal.code, Some(code::E_UNREADABLE_CONFIG));
    assert!(refusal.message.contains("typo"));
}

/// ADR 0104 § 1's own example, split across two files so § 2's ordering can be told apart from
/// [ADR 0103] § 3's: the *middle* block by specificity is the *last* one the tree read.
fn layered_shop() -> Fake {
    Fake::with(&[
        (
            "nvs.toml",
            "[[include]]\npath = \"conf.d/shop.toml\"\n\n\
             [[app]]\nroot = \"srv/www\"\norigin = \"https://example.test\"\n\
             [app.limits]\nmemory = \"256M\"\n\n\
             [[app]]\nentry = \"srv/www/shop/bin/import.nvs\"\n\
             [app.limits]\nwall_time = \"600s\"\n",
        ),
        (
            "conf.d/shop.toml",
            "[[app]]\nroot = \"../srv/www/shop\"\nmode = \"production\"\n\
             [app.limits]\nmemory = \"512M\"\n\
             [app.capabilities]\nprocess = { exec = true }\n",
        ),
        ("srv/www/shop/bin/import.nvs", ""),
    ])
}

/// § 2's worked example, asserted whole: `import.nvs` gets `512M` from the `shop` block and
/// `600s` from its own while inheriting everything neither states, and the `256M` the widest block
/// wrote is gone. Asserted together because a fold that dropped the inherited `origin` and one
/// that ignored specificity each pass on a different half.
#[test]
fn every_matching_blocks_directives_layer_least_specific_first() {
    let fs = layered_shop();
    let resolved = tree_of(&fs, "nvs.toml");

    let layered = layer(&resolved, &p("srv/www/shop/bin/import.nvs"), &fs)
        .unwrap_or_else(|err| panic!("refused: {}", err.message));
    let limits = layered.app.limits.clone().expect("the fold sets limits");

    assert_eq!(limits.memory, Some(Setting::Text("512M".to_string())));
    assert_eq!(limits.wall_time, Some(Setting::Text("600s".to_string())));
    assert_eq!(
        layered.app.origin.as_deref(),
        Some("https://example.test"),
        "the widest block's `origin` is inherited: neither of the two below it states one",
    );
    assert_eq!(layered.app.mode.as_deref(), Some("production"));
    assert!(
        layered.app.root.is_none() && layered.app.entry.is_none(),
        "the effective block is not keyed — the blocks it came from are in `blocks`",
    );
    assert_eq!(
        layered.blocks,
        [
            p("srv/www"),
            p("srv/www/shop"),
            p("srv/www/shop/bin/import.nvs")
        ],
        "least-specific first, and the `entry` block last, whatever order the files were read in",
    );
}

/// § 2's condition on itself: layering is [ADR 0103] § 3's later-wins with a different order, so
/// **every override is reported the same way** — both origins, and they are the two real files.
#[test]
fn a_directive_one_block_takes_from_another_is_reported_with_both_origins() {
    let fs = layered_shop();
    let resolved = tree_of(&fs, "nvs.toml");

    let layered = layer(&resolved, &p("srv/www/shop/bin/import.nvs"), &fs)
        .unwrap_or_else(|err| panic!("refused: {}", err.message));

    let memory: Vec<&Override> = layered
        .overrides
        .iter()
        .filter(|record| record.key == "limits.memory")
        .collect();
    assert_eq!(memory.len(), 1, "one block took `memory` from one other");
    assert_eq!(memory[0].replaced.path, p("nvs.toml"));
    assert_eq!(
        memory[0].winner.path,
        p("conf.d/shop.toml"),
        "the `shop` block wins on specificity though its file was read second anyway; the record \
         names where each value was written, which is what makes the ordering auditable",
    );
    assert!(
        !layered
            .overrides
            .iter()
            .any(|record| record.key == "root" || record.key == "entry"),
        "the keys that selected the blocks are not directives and are never reported as overrides",
    );
}

/// An entry file no block matches folds to nothing, which is § 2's ordinary case: the global
/// configuration, unmodified, and no override to report.
#[test]
fn an_entry_matched_by_no_block_layers_to_nothing() {
    let fs = layered_shop();
    let resolved = tree_of(&fs, "nvs.toml");
    let layered = layer(&resolved, &p("nvs.toml"), &fs)
        .unwrap_or_else(|err| panic!("refused: {}", err.message));

    assert_eq!(layered.app, App::default());
    assert!(layered.blocks.is_empty() && layered.overrides.is_empty());
}
