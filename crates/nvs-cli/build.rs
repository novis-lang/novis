//! Captures the handful of facts `nvs info` reports that only exist at build
//! time — the target triple, the profile, the compiler, the commit and its
//! date, and the code generator's version — and, for a Windows target, links
//! the icon and the version information that say the same things to Explorer.
//!
//! The facts are passed through `cargo:rustc-env`, so `src/info.rs` reads them
//! with `env!` and holds no build logic of its own. Nothing in this file may
//! fail the build: a fact that cannot be determined becomes the string
//! `unknown`, because a source tarball with no `.git` is a perfectly ordinary
//! way to build Novis and is not an error, and a resource that cannot be
//! written is a warning and a binary without one.
//!
//! No build timestamp is recorded. A build date would make two builds of the
//! same commit differ, and `nvs info` is the wrong place to spend a
//! reproducible build on a cosmetic field — the commit already answers "which
//! source is this?" exactly. The commit's own committer date is recorded in
//! its place: it answers how old a binary is, and because it is derived from
//! the commit rather than read off the clock, every rebuild of one commit
//! still produces the same bytes.

use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "build/winres.rs"]
mod winres;

fn main() {
    let manifest = PathBuf::from(env("CARGO_MANIFEST_DIR"));
    // crates/nvs-cli -> the workspace root.
    let workspace = manifest
        .parent()
        .and_then(Path::parent)
        .map_or_else(|| manifest.clone(), Path::to_path_buf);

    emit("NVS_TARGET", &env("TARGET"));
    emit("NVS_HOST", &env("HOST"));
    emit("NVS_PROFILE", &env("PROFILE"));
    emit("NVS_RUSTC", &rustc_version());
    let commit = commit(&workspace);
    emit("NVS_COMMIT", &commit);
    emit("NVS_COMMIT_DATE", &commit_date(&workspace));
    emit(
        "NVS_CRANELIFT",
        &locked_version(&workspace, "cranelift-codegen"),
    );

    // The attribution file is `include_str!`d by `src/info.rs`; Cargo does
    // not track a file included from outside the crate directory on its own.
    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("THIRD-PARTY-LICENSES.txt").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("LICENSE").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("Cargo.lock").display()
    );
    println!("cargo:rerun-if-env-changed=NVS_BUILD_COMMIT");
    println!("cargo:rerun-if-env-changed=NVS_BUILD_COMMIT_DATE");

    windows_resource(&manifest, &workspace, &commit);
}

/// Links `nvs.exe`'s icon and version information, on a Windows target whose
/// linker takes a `.res` file as an input.
///
/// That is the MSVC linker, which is every Windows target a release is built
/// for; a GNU one links the binary without the resource. The debug profile
/// gets its own icon and says so in its description, so the two builds are
/// told apart in a folder and in Task Manager. Every string is a fact this
/// tree already states somewhere — the manifests, `LICENSE`, the commit — and
/// `rule:packaging/the-windows-binary-says-what-it-is` owns which goes where.
fn windows_resource(manifest: &Path, workspace: &Path, commit: &str) {
    if env("CARGO_CFG_TARGET_OS") != "windows" || env("CARGO_CFG_TARGET_ENV") != "msvc" {
        return;
    }
    let debug = env("PROFILE") == "debug";
    let icon = manifest
        .join("assets")
        .join(if debug { "nvs-debug.ico" } else { "nvs.ico" });
    println!("cargo:rerun-if-changed={}", icon.display());
    println!(
        "cargo:rerun-if-changed={}",
        workspace.join("Cargo.toml").display()
    );

    let version = env("CARGO_PKG_VERSION");
    // The commit rides as semver build metadata, so the version a user reads
    // in a file's properties names the source the way `nvs info` does.
    let product_version = if commit == unknown() {
        version.clone()
    } else {
        format!("{version}+{commit}")
    };
    let copyright = format!(
        "{}. Released under the {} license.",
        copyright(workspace),
        env("CARGO_PKG_LICENSE")
    );
    let comments = format!(
        "{} {}",
        workspace_description(workspace),
        env("CARGO_PKG_HOMEPAGE")
    );
    // Task Manager and the firewall prompt show this one as the program's name.
    let description = if debug {
        "Novis (debug build)"
    } else {
        "Novis"
    };
    let info = winres::VersionInfo {
        version: [
            env("CARGO_PKG_VERSION_MAJOR").parse().unwrap_or(0),
            env("CARGO_PKG_VERSION_MINOR").parse().unwrap_or(0),
            env("CARGO_PKG_VERSION_PATCH").parse().unwrap_or(0),
        ],
        debug,
        prerelease: !env("CARGO_PKG_VERSION_PRE").is_empty(),
        strings: &[
            ("CompanyName", &env("CARGO_PKG_AUTHORS").replace(':', ", ")),
            ("FileDescription", description),
            ("FileVersion", &version),
            ("InternalName", "nvs"),
            ("LegalCopyright", &copyright),
            ("OriginalFilename", "nvs.exe"),
            ("ProductName", "Novis"),
            ("ProductVersion", &product_version),
            ("Comments", comments.trim()),
        ],
    };

    let written = std::fs::read(&icon)
        .ok()
        .and_then(|ico| winres::resource(&ico, &info))
        .and_then(|res| {
            let path = PathBuf::from(env("OUT_DIR")).join("nvs.res");
            std::fs::write(&path, res).ok().map(|()| path)
        });
    match written {
        Some(path) => println!("cargo:rustc-link-arg-bins={}", path.display()),
        None => println!(
            "cargo:warning=nvs.exe is linked without its icon and version information: {} could not be compiled into a resource",
            icon.display()
        ),
    }
}

/// `LICENSE`'s copyright line, with its `(c)` as the sign it stands for.
fn copyright(workspace: &Path) -> String {
    std::fs::read_to_string(workspace.join("LICENSE"))
        .ok()
        .and_then(|license| {
            license
                .lines()
                .map(str::trim)
                .find(|line| line.starts_with("Copyright"))
                .map(|line| line.replace("(c)", "\u{a9}"))
        })
        .unwrap_or_default()
}

/// The `description` under `[workspace.package]` in the root manifest, which
/// describes Novis where this package's own describes one crate of it.
///
/// Read by hand for the reason [`locked_version`] gives.
fn workspace_description(workspace: &Path) -> String {
    let Ok(manifest) = std::fs::read_to_string(workspace.join("Cargo.toml")) else {
        return String::new();
    };
    let mut in_table = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_table = line == "[workspace.package]";
        } else if in_table && let Some(value) = line.strip_prefix("description = ") {
            return value.trim_matches('"').to_owned();
        }
    }
    String::new()
}

fn env(key: &str) -> String {
    std::env::var(key).unwrap_or_default()
}

fn emit(key: &str, value: &str) {
    // A newline here would forge a second instruction to Cargo, and every
    // value below comes from a subprocess or the environment.
    let sanitised = value.replace(['\n', '\r'], " ");
    println!("cargo:rustc-env={key}={sanitised}");
}

/// `rustc --version`, via the compiler Cargo is actually driving.
///
/// `RUSTC` rather than a bare `rustc`: under `cargo +nightly`, a rustup
/// override or a distribution wrapper, the two are different programs and
/// only the first one built this binary.
fn rustc_version() -> String {
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    run(Command::new(rustc).arg("--version")).unwrap_or_else(unknown)
}

/// The short commit hash, with `-dirty` appended if the tree was modified.
///
/// `NVS_BUILD_COMMIT` wins if set, so a distribution packaging Novis from a
/// tarball can supply the revision it built from without a `.git` present.
fn commit(workspace: &Path) -> String {
    if let Ok(supplied) = std::env::var("NVS_BUILD_COMMIT")
        && !supplied.trim().is_empty()
    {
        return supplied.trim().to_owned();
    }

    let Some(git) = git_dir(workspace) else {
        return unknown();
    };
    // Re-run when the checked-out commit moves. `index` covers a `git add`,
    // which is what flips the dirty marker below. Both are named in the real
    // git directory: a path that does not exist makes Cargo re-run this
    // script, and relink `nvs`, on every build.
    println!("cargo:rerun-if-changed={}", git.join("HEAD").display());
    println!("cargo:rerun-if-changed={}", git.join("index").display());

    let Some(hash) =
        run(Command::new("git")
            .current_dir(workspace)
            .args(["rev-parse", "--short=9", "HEAD"]))
    else {
        return unknown();
    };

    let dirty = run(Command::new("git").current_dir(workspace).args([
        "status",
        "--porcelain",
        "--untracked-files=no",
    ]))
    .is_some_and(|status| !status.is_empty());

    if dirty { format!("{hash}-dirty") } else { hash }
}

/// The directory that holds this checkout's `HEAD` and `index`, or `None`
/// when the workspace is not a git checkout.
///
/// In a clone, `.git` is that directory. In a linked worktree, `.git` is a
/// file whose one line is `gitdir: <path>`, and the path it names holds the
/// worktree's own `HEAD` and `index`. A relative path is taken from the
/// workspace, as git takes it.
fn git_dir(workspace: &Path) -> Option<PathBuf> {
    let dot = workspace.join(".git");
    if dot.is_dir() {
        return Some(dot);
    }
    let text = std::fs::read_to_string(&dot).ok()?;
    let named = text
        .lines()
        .find_map(|line| line.strip_prefix("gitdir:"))?
        .trim();
    let path = Path::new(named);
    let dir = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace.join(path)
    };
    dir.is_dir().then_some(dir)
}

/// The commit's own committer date, as `YYYY-MM-DD`.
///
/// The commit's date and never the build's, which is what lets the banner say
/// how old a binary is without costing the reproducible build this file's
/// module doc protects: the value is derived from the commit, so every
/// rebuild of one commit emits the same string. `NVS_BUILD_COMMIT_DATE` is
/// the tarball escape hatch `NVS_BUILD_COMMIT` already is for the hash.
fn commit_date(workspace: &Path) -> String {
    if let Ok(supplied) = std::env::var("NVS_BUILD_COMMIT_DATE")
        && !supplied.trim().is_empty()
    {
        return supplied.trim().to_owned();
    }
    if git_dir(workspace).is_none() {
        return unknown();
    }
    // `commit` above already registered the rerun triggers for a moved HEAD.
    run(Command::new("git")
        .current_dir(workspace)
        .args(["log", "-1", "--format=%cs", "HEAD"]))
    .unwrap_or_else(unknown)
}

/// The version of `name` recorded in the workspace `Cargo.lock`.
///
/// Cranelift exposes no version constant of its own and a build script
/// cannot read a dependency's `CARGO_PKG_VERSION`, so the lockfile — which
/// is committed, and is what resolved the build — is the honest source.
/// Parsed by hand rather than with a TOML dependency: the two lines that
/// matter have a fixed shape, and this is a build script for a compiler
/// whose supply chain is the thing it advertises.
fn locked_version(workspace: &Path, name: &str) -> String {
    let Ok(lock) = std::fs::read_to_string(workspace.join("Cargo.lock")) else {
        return unknown();
    };
    let mut in_package = false;
    for line in lock.lines() {
        let line = line.trim();
        if line == "[[package]]" {
            in_package = false;
        } else if let Some(value) = line.strip_prefix("name = ") {
            in_package = value.trim_matches('"') == name;
        } else if in_package && let Some(value) = line.strip_prefix("version = ") {
            return value.trim_matches('"').to_owned();
        }
    }
    unknown()
}

/// Runs a command, returning its trimmed stdout, or `None` for any failure.
///
/// "Any failure" is the point: the program missing, a non-zero exit and
/// non-UTF-8 output are all just facts we do not have.
fn run(command: &mut Command) -> Option<String> {
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

fn unknown() -> String {
    "unknown".to_owned()
}
