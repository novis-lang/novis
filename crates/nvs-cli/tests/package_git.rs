//! A root git dependency through `nvs pkg fetch` (`rule:packaging/a-git-dependency-is-root-only`).
//!
//! The repository is a `file:` remote the test builds with `gix`, so nothing reaches the network.
//! The clone's limits, its refusals and the `https:` fetch are unit tests in `src/package/git.rs`;
//! this test shows the commit's package reaches `vendor/`, the lock and `[grants]`.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

/// A repository at `dir` with one commit of `files`, each a path and its contents. The commit's
/// hash.
fn repository(dir: &Path, files: &[(&str, &[u8])]) -> String {
    let repo = gix::init_bare(dir).expect("the test makes a repository");
    let mut editor = repo
        .edit_tree(gix::ObjectId::empty_tree(gix::hash::Kind::Sha1))
        .expect("the test edits a tree");
    for (path, contents) in files {
        let blob = repo.write_blob(contents).expect("the test writes a file");
        editor
            .upsert(*path, gix::object::tree::EntryKind::Blob, blob)
            .expect("the test adds a file");
    }
    let tree = editor.write().expect("the test writes the tree");
    let person = gix::actor::Signature {
        name: "Test".into(),
        email: "test@example.com".into(),
        time: gix::date::Time::new(0, 0),
    };
    let mut committer = gix::date::parse::TimeBuf::default();
    let mut author = gix::date::parse::TimeBuf::default();
    let commit = repo
        .new_commit_as(
            person.to_ref(&mut committer),
            person.to_ref(&mut author),
            "one",
            tree,
            std::iter::empty::<gix::ObjectId>(),
        )
        .expect("the test commits");
    commit.id.to_hex().to_string()
}

/// `nvs pkg fetch` in `app`, with `data` as the data folder.
fn fetch(app: &Path, data: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(data)
        .args(["--no-init", "pkg", "fetch"])
        .current_dir(app)
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

fn shown(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

// covers: tools:packages/nvs-pkg-fetch-and-nvs-pkg-vendor
#[test]
fn a_git_dependency_is_fetched_at_its_commit() {
    let root = nvs_repo::scratch("package-git-fetch");
    let data = nvs_repo::scratch_private("package-git-fetch-data");
    let rev = repository(
        &root.join("repo"),
        &[
            (
                "package.toml",
                b"[package]\nname = 'Acme\\Git'\nversion = \"1.0.0\"\n",
            ),
            ("LICENSE", b"MIT\n"),
            (
                "src/Main.nvs",
                b"<?nvs\nnamespace Acme\\Git;\n\nfinal class Main\n{\n}\n",
            ),
            ("docs/notes.txt", b"not in the archive\n"),
        ],
    );
    let app = root.join("app");
    fs::create_dir_all(&app).expect("the test makes the application");
    fs::write(
        app.join("package.toml"),
        format!("[dependencies]\n'Acme\\Git' = {{ git = \"file:../repo\", rev = \"{rev}\" }}\n"),
    )
    .expect("the test writes the manifest");

    let out = fetch(&app, &data);
    assert!(out.status.success(), "{}", shown(&out));
    let package = app.join("vendor/acme/git");
    assert!(package.join("src/Main.nvs").is_file());
    assert!(
        !package.join("docs").exists(),
        "a path outside the archive was written"
    );
    let lock = fs::read_to_string(app.join("package.lock")).expect("the lock was written");
    assert!(lock.contains("source = \"git+file:../repo\""), "{lock}");
    assert!(lock.contains(&format!("commit = \"{rev}\"")), "{lock}");
    let manifest = fs::read_to_string(app.join("package.toml")).expect("the manifest reads");
    assert!(manifest.contains("'Acme\\Git\\*' = []"), "{manifest}");

    let out = fetch(&app, &data);
    assert!(out.status.success(), "{}", shown(&out));
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("0 written"),
        "a second fetch keeps the folder: {}",
        shown(&out)
    );
    let left: Vec<_> = fs::read_dir(data.join("tmp"))
        .map(|dir| dir.flatten().map(|entry| entry.path()).collect())
        .unwrap_or_default();
    assert!(left.is_empty(), "the clone's folders are removed: {left:?}");
}
