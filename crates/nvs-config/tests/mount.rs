//! `rule:http-server/a-path-is-never-derived-from-a-url` and `rule:http-server/a-mount-table-expands-at-boot`: the `[[server.mount]]` blocks read into the literal set of entry files a server
//! may execute, with every glob already expanded against the disk.
//!
//! Every case runs against an in-memory [`Files`] for `tests/app.rs`'s reason — the containment
//! rule is asserted against a symlink, and planting one is a privileged operation on Windows, so a
//! real directory would skip the case on the platform the check matters most on. The reader here
//! differs from that one in a single way: `list` reports a directory as well as a file, because a
//! glob's `*` is a directory listing and a reader that only ever returned files would expand
//! nothing.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use nvs_config::mount::{Mounted, expand, expand_again};
use nvs_config::resolve::Files;
use nvs_config::tree::Config;
use nvs_config::trust::Untrusted;
use nvs_diagnostics::{Diagnostic, code};

/// A path written the way an ADR writes one, as a path the host spells its own way.
///
/// Spelled as a separator substitution rather than as a component collection, because these paths
/// are compared against ones the tree itself produced through `Path::new("/www")` — and a collected
/// leading empty component drops the root on Windows, so the two would never be the same path.
fn p(path: &str) -> PathBuf {
    PathBuf::from(path.replace('/', std::path::MAIN_SEPARATOR_STR))
}

/// The filesystem the cases describe: a set of file paths, with directories implied by them.
#[derive(Default)]
struct Fake {
    files: Vec<PathBuf>,
    links: BTreeMap<PathBuf, PathBuf>,
}

impl Fake {
    fn with(paths: &[&str]) -> Self {
        Self {
            files: paths.iter().map(|path| p(path)).collect(),
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

    fn read(&self, _path: &Path) -> Result<String, String> {
        Err("no case here reads a mounted file".to_string())
    }

    fn read_bytes(&self, _path: &Path) -> Result<Vec<u8>, String> {
        Err("no case here reads a mounted file".to_string())
    }

    fn exposure(&self, _path: &Path) -> Option<String> {
        None
    }

    /// Both halves of a listing: the files directly under `dir`, and the directories the deeper
    /// ones imply. A glob's `*` walks the second half, so a reader without it expands nothing.
    fn list(&self, dir: &Path) -> Result<Vec<PathBuf>, String> {
        let mut out: Vec<PathBuf> = Vec::new();
        for path in &self.files {
            let Ok(rest) = path.strip_prefix(dir) else {
                continue;
            };
            let Some(first) = rest.components().next() else {
                continue;
            };
            let child = dir.join(first.as_os_str());
            if !out.contains(&child) {
                out.push(child);
            }
        }
        Ok(out)
    }

    fn exists(&self, path: &Path) -> bool {
        self.files.iter().any(|file| file.starts_with(path))
    }
}

/// A tree, as an operator would write it.
fn tree(text: &str) -> Config {
    toml::from_str(text).expect("the fixture did not deserialize")
}

/// The table `text` expands to over `fs`, panicking with the refusal when it does not.
fn table(fs: &Fake, text: &str) -> Vec<Mounted> {
    expand(&tree(text), &BTreeMap::new(), fs)
        .unwrap_or_else(|err| panic!("refused: {} [{:?}]", err.message, err.notes))
}

/// The refusal expanding `text` over `fs` produces, panicking when it is accepted instead.
fn refusal(fs: &Fake, text: &str) -> Diagnostic {
    expand(&tree(text), &BTreeMap::new(), fs).expect_err("this tree should have been refused")
}

/// The refusal `nvs_config::server::validate` produces — the disk-free half, which is what
/// `nvs config check` runs and what every boot runs before it lists anything.
fn checked(text: &str) -> Diagnostic {
    nvs_config::server::validate(&tree(text), &BTreeMap::new())
        .expect_err("this tree should have been refused")
}

/// `rule:http-server/a-mount-table-expands-at-boot`'s own example, over a disk that holds module directories and one that is not
/// a module at all.
///
/// The count is the assertion that matters: § 2's claim is that the executable set is *enumerated*,
/// so a reading that expanded a `*` per request — or one that kept a candidate whose entry is not
/// there — would satisfy every per-mount assertion below while leaving an extra entry in the
/// table.
// covers: tools:server/mounts-which-file-answers-a-request
#[test]
fn a_mount_globs_is_expanded_against_disk_at_boot() {
    let fs = Fake::with(&[
        "/www/Blog/public/index.nvs",
        "/www/Blog/public/style.css",
        "/www/Blog/src/Post.nvs",
        "/www/Shop/public/index.nvs",
        // A directory under the root with no entry where the glob looks: it is not a mount, and
        // the count below is what says so.
        "/www/Notes/README.md",
    ]);
    let mounts = table(
        &fs,
        "[server]\nroot = \"/www\"\n\n[[server.mount]]\nscan = \"*/public/index.nvs\"\n\
         prefix = \"/{1}\"\norigin = \"https://{1}.example.com\"\n",
    );

    assert_eq!(
        mounts.len(),
        2,
        "one mount per directory that holds an entry"
    );
    assert_eq!(
        mounts
            .iter()
            .map(|one| one.prefix.clone())
            .collect::<Vec<_>>(),
        vec!["/Blog".to_string(), "/Shop".to_string()]
    );
    assert_eq!(mounts[0].entry, p("/www/Blog/public/index.nvs"));
    // § 4's "the mount root": the directory the entry sits in, which is where steps 3 and 4 look
    // for a static file and for a dispatched `.nvs`.
    assert_eq!(mounts[0].root, p("/www/Blog/public"));
    assert_eq!(mounts[0].captures, vec!["Blog".to_string()]);
    assert_eq!(
        mounts[0].origin.as_deref(),
        Some("https://Blog.example.com")
    );
    assert_eq!(mounts[0].host, None);
    // Every mount the table holds names a file the disk holds: that equality is § 2's governing
    // rule, and it is the one a per-request expansion could not state at all.
    assert!(mounts.iter().all(|one| fs.exists(&one.entry)));
}

/// A mount table with no `[server] root` written is refused by the disk-free half, so `nvs config
/// check` and every boot say so: the root is where every mounted file is found, and it is never
/// the directory the process happened to start in.
#[test]
fn a_mount_table_with_no_root_written_is_refused() {
    let text = "[[server.mount]]\nscan = \"*/public/index.nvs\"\nprefix = \"/{1}\"\n";
    let err = checked(text);
    assert_eq!(err.code, Some(code::E_BAD_MOUNT));
    assert!(err.message.contains("`[server] root`"), "{}", err.message);

    // The disk half says the same to a caller that only has it, and to a tree that writes no
    // mount at all: the implicit mount is under the root too.
    let fs = Fake::with(&["/www/Blog/public/index.nvs", "/public/index.nvs"]);
    assert!(refusal(&fs, text).message.contains("`[server] root`"));
    assert!(refusal(&fs, "").message.contains("`[server] root`"));
}

/// § 3's other spelling of the same glob: a capture in `host` rather than in `prefix`, which is
/// what makes a module relocatable without the route table naming a host.
#[test]
fn a_capture_reads_the_same_in_a_host_as_in_a_prefix() {
    let fs = Fake::with(&["/www/blog/public/index.nvs", "/www/shop/public/index.nvs"]);
    let mounts = table(
        &fs,
        "[server]\nroot = \"/www\"\n\n[[server.mount]]\nscan = \"*/public/index.nvs\"\n\
         host = \"{1}.example.com\"\n",
    );
    assert_eq!(
        mounts
            .iter()
            .map(|one| one.host.clone())
            .collect::<Vec<_>>(),
        vec![
            Some("blog.example.com".to_string()),
            Some("shop.example.com".to_string()),
        ]
    );
    // A host mount with no `prefix` written answers at the root of that host, which is the only
    // reading that makes `host` alone a complete match under § 4 step 1.
    assert!(mounts.iter().all(|one| one.prefix == "/"));
}

/// § 3's override, asserted from both ends: an explicit `entry` replaces the scanned mount at its
/// key whichever block was read first, and two explicit ones at a key is a boot error. A case
/// asserting only the first order would pass against a reading that let the later block win.
// covers: tools:server/mounts-which-file-answers-a-request
#[test]
fn an_explicit_entry_overrides_a_scanned_mount_at_the_same_key() {
    // `Backoffice` is the irregular module § 3's second block is for: its entry is not where the
    // glob looks, so it reaches the table through the explicit block or not at all.
    let fs = Fake::with(&[
        "/www/admin/public/index.nvs",
        "/www/shop/public/index.nvs",
        "/www/Backoffice/app.nvs",
    ]);
    let scan = "[[server.mount]]\nscan = \"*/public/index.nvs\"\nprefix = \"/{1}\"\n";
    let literal = "[[server.mount]]\nprefix = \"/admin\"\nentry = \"Backoffice/app.nvs\"\n";
    for text in [
        format!("[server]\nroot = \"/www\"\n\n{scan}\n{literal}"),
        format!("[server]\nroot = \"/www\"\n\n{literal}\n{scan}"),
    ] {
        let mounts = table(&fs, &text);
        assert_eq!(
            mounts.len(),
            2,
            "the override is a replacement, not an addition"
        );
        let admin = mounts
            .iter()
            .find(|one| one.prefix == "/admin")
            .expect("the explicit mount is missing");
        assert_eq!(admin.entry, p("/www/Backoffice/app.nvs"));
        assert!(
            admin.captures.is_empty(),
            "a literal entry captures nothing"
        );
    }

    let refused = refusal(
        &fs,
        &format!("[server]\nroot = \"/www\"\n\n{literal}\n{literal}"),
    );
    assert_eq!(refused.code, Some(code::E_BAD_MOUNT));
}

/// § 3's last bullet: a tree with no `[[server.mount]]` at all is one mount and not none. An
/// absent block is the single-module deployment, which is the shape a first `nvs serve` has.
#[test]
fn a_tree_with_no_mount_block_has_one_implicit_mount_at_the_root() {
    let fs = Fake::with(&["/www/public/index.nvs"]);
    for text in [
        "[server]\nroot = \"/www\"\n",
        "[server]\nroot = \"/www\"\nstatic = true\n",
    ] {
        let mounts = table(&fs, text);
        assert_eq!(
            mounts,
            vec![Mounted {
                prefix: "/".to_string(),
                host: None,
                entry: p("/www/public/index.nvs"),
                root: p("/www/public"),
                origin: None,
                captures: Vec::new(),
            }]
        );
    }
    // And it is refused rather than empty when the file it names is not there: a server whose
    // whole executable set is missing answers every request with nothing, which § 2 makes a boot
    // question rather than a per-request one.
    let refused = refusal(
        &Fake::with(&["/www/index.nvs"]),
        "[server]\nroot = \"/www\"\n",
    );
    assert_eq!(refused.code, Some(code::E_BAD_MOUNT));
}

/// § 3: every resolved path is checked to resolve inside `[server] root`, once, at boot. Both
/// sides are named together because a reading that only refused a written `..` would accept the
/// symlink, which is the same escape with the lexical half already done for it.
#[test]
fn an_entry_that_resolves_outside_server_root_is_refused() {
    let fs = Fake::with(&[
        "/www/shop/public/index.nvs",
        "/secret/public/index.nvs",
        "/elsewhere/public/index.nvs",
    ])
    .linking("/www/away", "/elsewhere");

    let inside = table(
        &fs,
        "[server]\nroot = \"/www\"\n\n[[server.mount]]\nprefix = \"/shop\"\n\
         entry = \"shop/public/index.nvs\"\n",
    );
    assert_eq!(inside.len(), 1);

    for escape in ["../secret/public/index.nvs", "away/public/index.nvs"] {
        let refused = refusal(
            &fs,
            &format!(
                "[server]\nroot = \"/www\"\n\n[[server.mount]]\nprefix = \"/x\"\nentry = \"{escape}\"\n"
            ),
        );
        assert_eq!(refused.code, Some(code::E_BAD_MOUNT), "for {escape:?}");
    }
}

/// § 3's capture charset, which is `rule:errors/path-component-refusals` on every platform. The refusals are named
/// together because each is a different half of that rule, and a reading that held only the
/// charset would mount a directory called `CON`.
#[test]
fn a_captured_segment_that_does_not_spell_itself_is_refused() {
    for directory in ["CON", "lpt1", "shop~1", ".hidden"] {
        let fs = Fake::with(&[
            "/www/shop/public/index.nvs",
            &format!("/www/{directory}/public/index.nvs"),
        ]);
        let refused = refusal(
            &fs,
            "[server]\nroot = \"/www\"\n\n[[server.mount]]\nscan = \"*/public/index.nvs\"\n\
             prefix = \"/{1}\"\n",
        );
        assert_eq!(refused.code, Some(code::E_BAD_MOUNT), "for {directory:?}");
    }
    // A name that only *contains* a reserved word is a name, not a device: refusing it would be
    // `rule:errors/ambiguous-input-refused`'s repair rather than its refusal, applied to the wrong string.
    let fs = Fake::with(&["/www/console/public/index.nvs"]);
    assert_eq!(
        table(
            &fs,
            "[server]\nroot = \"/www\"\n\n[[server.mount]]\nscan = \"*/public/index.nvs\"\n\
             prefix = \"/{1}\"\n"
        )
        .len(),
        1
    );
}

/// A running server's expansion leaves out each match a boot would refuse and keeps the rest: a
/// capture that names a device, an explicit entry that is not on disk, and a second mount at a key.
/// What was left out comes back as the same `E0621` a boot would have stopped on.
#[test]
fn an_expansion_after_boot_leaves_out_a_refused_match_and_keeps_the_rest() {
    let fs = Fake::with(&[
        "/www/shop/public/index.nvs",
        "/www/CON/public/index.nvs",
        "/www/blog/public/index.nvs",
    ]);
    let text = "[server]\nroot = \"/www\"\n\n[[server.mount]]\nscan = \"*/public/index.nvs\"\n\
                prefix = \"/{1}\"\n\n[[server.mount]]\nprefix = \"/admin\"\n\
                entry = \"admin/public/index.nvs\"\n\n[[server.mount]]\nprefix = \"/x\"\n\
                entry = \"blog/public/index.nvs\"\n\n[[server.mount]]\nprefix = \"/x\"\n\
                entry = \"shop/public/index.nvs\"\n";
    let (mounts, left_out) = expand_again(&tree(text), &BTreeMap::new(), &fs)
        .unwrap_or_else(|err| panic!("refused: {} [{:?}]", err.message, err.notes));
    let prefixes: Vec<&str> = mounts.iter().map(|mount| mount.prefix.as_str()).collect();
    assert_eq!(prefixes, ["/blog", "/shop", "/x"]);
    assert_eq!(
        mounts[2].entry,
        p("/www/blog/public/index.nvs"),
        "the first mount at a key is kept"
    );
    assert_eq!(left_out.len(), 3, "{left_out:?}");
    assert!(
        left_out
            .iter()
            .all(|refused| refused.code == Some(code::E_BAD_MOUNT))
    );
    // A boot over the same disk stops on the first of them.
    assert_eq!(refusal(&fs, text).code, Some(code::E_BAD_MOUNT));
}

/// The half that needs no disk, through the function `nvs config check` calls. Every shape a block
/// can be wrong about is asserted in one case because they are one rule — a block that does not
/// name exactly one file and at least one thing to match on is not a mount — and because the
/// wiring into `server::validate` is what makes any of them reachable without a `/www`.
#[test]
fn a_block_that_names_no_mount_is_refused_without_asking_the_disk() {
    for text in [
        // Both sources, and neither.
        "[server]\nroot = \"/www\"\n\n[[server.mount]]\nprefix = \"/\"\nscan = \"*/index.nvs\"\nentry = \"a.nvs\"\n",
        "[server]\nroot = \"/www\"\n\n[[server.mount]]\nprefix = \"/\"\n",
        // Nothing to match on.
        "[server]\nroot = \"/www\"\n\n[[server.mount]]\nentry = \"a.nvs\"\n",
        // A prefix a request path can never begin with.
        "[server]\nroot = \"/www\"\n\n[[server.mount]]\nprefix = \"admin\"\nentry = \"a.nvs\"\n",
        // A reference past the last `*`, and one counting from zero.
        "[server]\nroot = \"/www\"\n\n[[server.mount]]\nscan = \"*/index.nvs\"\nprefix = \"/{2}\"\n",
        "[server]\nroot = \"/www\"\n\n[[server.mount]]\nscan = \"*/index.nvs\"\nprefix = \"/{0}\"\n",
        // A capture in an `origin` is numbered by the same `*`s as one in a prefix.
        "[server]\nroot = \"/www\"\n\n[[server.mount]]\nscan = \"*/index.nvs\"\nprefix = \"/{1}\"\n\
         origin = \"https://{2}.example.com\"\n",
        // A literal `entry` captures nothing at all, so any reference in it is out of range.
        "[server]\nroot = \"/www\"\n\n[[server.mount]]\nprefix = \"/{1}\"\nentry = \"a.nvs\"\n",
    ] {
        let refused = checked(text);
        assert_eq!(refused.code, Some(code::E_BAD_MOUNT), "for {text:?}");
    }
}

/// A brace that is not `{n}` or `{n:lower}` is refused in every field that expands, and without a
/// disk. Kept as text it would be a mount no request reaches — a prefix is matched against the path
/// as it was sent, and neither a URL path nor a host name carries a brace unescaped — so each of
/// these used to pass `nvs config check`, boot, and answer nothing.
#[test]
fn a_brace_that_is_not_a_capture_reference_is_refused() {
    for written in [
        // A transform this table does not have, in every spelling an operator might try.
        "{1:upper}",
        "{1:Lower}",
        "{1:}",
        "{1|lower}",
        "{lower(1)}",
        "{1,lower}",
        // Not a number at all, and a number `usize` would parse but no operator means.
        "{name}",
        "{}",
        "{+1}",
        "{ 1 }",
        // Braces that pair with nothing, and one inside another.
        "{1",
        "1}",
        "{{1}}",
    ] {
        for block in [
            format!("prefix = \"/sites/{written}.web\"\n"),
            format!("prefix = \"/{{1}}\"\nhost = \"{written}.dev\"\n"),
            format!("prefix = \"/{{1}}\"\norigin = \"https://{written}.dev\"\n"),
        ] {
            let text = format!(
                "[server]\nroot = \"/www\"\n\n[[server.mount]]\nscan = \"*/index.nvs\"\n{block}"
            );
            let refused = checked(&text);
            assert_eq!(refused.code, Some(code::E_BAD_MOUNT), "for {text:?}");
        }
    }
    // The refusal names what was written, so the operator is not left to find the brace.
    let refused = checked(
        "[server]\nroot = \"/www\"\n\n[[server.mount]]\nscan = \"*/index.nvs\"\nprefix = \"/sites/{1:upper}.web\"\n",
    );
    assert!(
        refused.message.contains("`{1:upper}`"),
        "{}",
        refused.message
    );
    // A reference past the last `*` is still that refusal with a transform on it.
    let refused = checked(
        "[server]\nroot = \"/www\"\n\n[[server.mount]]\nscan = \"*/index.nvs\"\nprefix = \"/sites/{2:lower}.web\"\n",
    );
    assert!(refused.message.contains("`{2}`"), "{}", refused.message);
}

/// `{n:lower}` writes the capture in ASCII lower case, which is what lets a directory named for the
/// namespace it holds be served at a lower-case URL by one glob.
///
/// The capture itself is untouched: `Core\Request::mount()` returns the directory's own spelling,
/// and the entry path is the disk's. Only the text the transform was written in changes.
#[test]
fn a_lower_transform_writes_the_capture_in_lower_case() {
    let fs = Fake::with(&["/www/Blog/public/index.nvs", "/www/Shop/public/index.nvs"]);
    let mounts = table(
        &fs,
        "[server]\nroot = \"/www\"\n\n[[server.mount]]\nscan = \"*/public/index.nvs\"\n\
         prefix = \"/sites/{1:lower}.web\"\norigin = \"https://{1:lower}.example.com/{1}\"\n",
    );
    assert_eq!(
        mounts
            .iter()
            .map(|one| one.prefix.clone())
            .collect::<Vec<_>>(),
        vec!["/sites/blog.web".to_string(), "/sites/shop.web".to_string()]
    );
    assert_eq!(mounts[0].captures, vec!["Blog".to_string()]);
    assert_eq!(mounts[0].entry, p("/www/Blog/public/index.nvs"));
    // Both spellings in one template: the transform belongs to the reference, not to the field.
    assert_eq!(
        mounts[0].origin.as_deref(),
        Some("https://blog.example.com/Blog")
    );
}

/// Two directories a transform folds onto one prefix are two mounts at one key, which § 3 already
/// makes a boot error — a case-sensitive disk can hold both, and a table that kept either would
/// serve one module's URL from the other.
#[test]
fn two_captures_a_transform_folds_together_are_a_duplicate() {
    let fs = Fake::with(&["/www/Blog/public/index.nvs", "/www/blog/public/index.nvs"]);
    let refused = refusal(
        &fs,
        "[server]\nroot = \"/www\"\n\n[[server.mount]]\nscan = \"*/public/index.nvs\"\n\
         prefix = \"/{1:lower}\"\n",
    );
    assert_eq!(refused.code, Some(code::E_BAD_MOUNT));
}

/// A host is held in ASCII lower case, because that is how a request's `Host` is compared. Two
/// blocks whose hosts differ only in case are therefore one key: a boot error when both are
/// explicit, and § 3's override when one is scanned.
#[test]
fn a_host_is_one_key_whatever_case_it_is_written_in() {
    let fs = Fake::with(&["/www/Blog/public/index.nvs", "/www/other/app.nvs"]);
    let mounts = table(
        &fs,
        "[server]\nroot = \"/www\"\n\n[[server.mount]]\nscan = \"*/public/index.nvs\"\n\
         host = \"{1}.Local.Test\"\n",
    );
    assert_eq!(mounts[0].host.as_deref(), Some("blog.local.test"));
    assert_eq!(mounts[0].captures, vec!["Blog".to_string()]);

    // An explicit block in another case overrides the scanned one rather than sitting beside it.
    let mounts = table(
        &fs,
        "[server]\nroot = \"/www\"\n\n[[server.mount]]\nscan = \"*/public/index.nvs\"\n\
         host = \"{1}.local.test\"\n\n[[server.mount]]\nhost = \"blog.LOCAL.test\"\n\
         entry = \"other/app.nvs\"\n",
    );
    assert_eq!(mounts.len(), 1);
    assert_eq!(mounts[0].entry, p("/www/other/app.nvs"));

    // Two explicit blocks a request could not tell apart.
    let refused = refusal(
        &fs,
        "[server]\nroot = \"/www\"\n\n[[server.mount]]\nhost = \"A.example.com\"\n\
         entry = \"other/app.nvs\"\n\n[[server.mount]]\nhost = \"a.example.com\"\n\
         entry = \"Blog/public/index.nvs\"\n",
    );
    assert_eq!(refused.code, Some(code::E_BAD_MOUNT));
}
