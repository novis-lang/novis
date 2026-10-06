//! The compile cache, driven through the built binary: `nvs run` with an
//! `[opcache] file_cache_dir` stores the compiled program on its first start
//! and loads it on the next, and the unit tests in `src/cache.rs` pin each
//! half of that on its own.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A fresh directory under this test binary's scratch space, writable by
/// this account alone: `[opcache] file_cache_dir` is not used when it, or the
/// directory holding it, is writable by others.
fn private_scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("compile-cache-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the scratch directory");
    #[cfg(windows)]
    {
        let me = nvs_repo::spawn("whoami", &[])
            .output()
            .expect("`whoami` starts");
        let me = String::from_utf8_lossy(&me.stdout).trim().to_owned();
        let ran = nvs_repo::spawn("icacls", &[])
            .arg(&dir)
            .args(["/inheritance:r", "/grant:r", &format!("{me}:(OI)(CI)F")])
            .output()
            .expect("`icacls` starts");
        assert!(
            ran.status.success(),
            "`icacls` could not make the scratch private"
        );
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
            .expect("the scratch directory can be made private");
    }
    dir
}

/// `nvs --config <config> run <program>`, as `(stdout, stderr)`, asserting
/// that it succeeds.
fn run(config: &Path, program: &Path) -> (String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--config")
        .arg(config)
        .arg("run")
        .arg(program)
        // A recorded run of this test binary sets the switch, and this case
        // is about what happens without it.
        .env_remove("NOVIS_NO_FILE_CACHE")
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    let stdout = String::from_utf8(out.stdout).expect("the output is UTF-8");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(out.status.success(), "`nvs run` succeeds: {stderr}");
    (stdout, stderr)
}

/// Every `*.nvsc` file under `dir`, sorted.
fn artifacts(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(shards) = std::fs::read_dir(dir) else {
        return out;
    };
    for shard in shards {
        let shard = shard.expect("a readable shard").path();
        for entry in std::fs::read_dir(&shard).expect("a shard is a directory") {
            let path = entry.expect("a readable entry").path();
            if path.extension().is_some_and(|ext| ext == "nvsc") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// The attack written against the cache, run twice over one cache directory:
/// the first start writes one artifact per compiled unit, the second finds
/// them and writes nothing, and both print the same lines — a program loaded
/// from the cache behaves as the one compiled for it.
// covers: tools:cli/the-compile-cache
#[test]
fn a_program_loaded_from_the_cache_prints_what_the_compiled_one_printed() {
    let program = nvs_repo::path("tests")
        .join("hostile")
        .join("tools")
        .join("cli")
        .join("the-compile-cache")
        .join("01-a-program-that-is-hard-to-store.nvs");
    let dir = private_scratch("warm");
    let cache = dir.join("artifacts");
    // Created here, so the check reaches the private directory above it and
    // no further: a cache directory that does not exist yet is checked at
    // its nearest ancestor, and that ancestor's own parent is checked too.
    std::fs::create_dir(&cache).expect("the cache directory");
    let config = dir.join("nvs.toml");
    // A TOML literal string, since a Windows path is mostly backslashes.
    std::fs::write(
        &config,
        format!("[opcache]\nfile_cache_dir = '{}'\n", cache.display()),
    )
    .expect("the configuration is written");

    let (cold, err) = run(&config, &program);
    assert!(
        !err.contains("file_cache_dir"),
        "the private directory is not refused: {err}"
    );
    let stored = artifacts(&cache);
    assert!(
        !stored.is_empty(),
        "the first start stored the compiled program"
    );

    let (warm, _) = run(&config, &program);
    assert_eq!(
        warm, cold,
        "the loaded program prints what the compiled one did"
    );
    assert_eq!(
        artifacts(&cache),
        stored,
        "the second start loaded the artifact and wrote no other"
    );
    assert!(
        cold.lines()
            .any(|line| line == "abcdefghijklmnopqrstuvwxyz"),
        "the program ran to its last steps: {cold}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// One program, the same name and the same text, in two folders: each run
/// prints the path its own folder gives, because a relative path given as a
/// string literal compiles to an absolute path joined to the file's folder
/// (`rule:programs/relative-paths-resolve-from-their-file`). Each is run as
/// `nvs run main.nvs` from inside its folder, so the name a diagnostic prints
/// is the same for both and only the folder tells the two compiles apart.
// covers: tools:cli/the-compile-cache
#[test]
fn the_same_file_in_two_folders_is_two_cache_entries() {
    let dir = private_scratch("moved");
    let cache = dir.join("artifacts");
    std::fs::create_dir(&cache).expect("the cache directory");
    let config = dir.join("nvs.toml");
    std::fs::write(
        &config,
        format!("[opcache]\nfile_cache_dir = '{}'\n", cache.display()),
    )
    .expect("the configuration is written");
    let text = "<?nvs\nclass Here {\n    public static function path(#[Core\\Path] string $path): string {\n        return $path;\n    }\n}\necho Here::path('data.txt'), \"\\n\";\n";

    let mut printed = Vec::new();
    for folder in ["one", "two"] {
        let home = dir.join(folder);
        std::fs::create_dir(&home).expect("the program's folder");
        std::fs::write(home.join("main.nvs"), text).expect("the program is written");
        let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
            .arg("--config")
            .arg(&config)
            .arg("run")
            .arg("main.nvs")
            .current_dir(&home)
            .env_remove("NOVIS_NO_FILE_CACHE")
            .output()
            .expect("the `nvs` binary this test was built beside runs");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "`nvs run` succeeds: {stderr}");
        let stdout = String::from_utf8(out.stdout).expect("the output is UTF-8");
        let expected = std::path::absolute(home.join("data.txt")).expect("an absolute path");
        assert_eq!(
            Path::new(stdout.trim_end()),
            expected,
            "the program in `{folder}` prints its own folder's path"
        );
        printed.push(stdout);
    }
    assert_ne!(printed[0], printed[1]);
    assert_eq!(
        artifacts(&cache).len(),
        2,
        "each folder stored its own compiled program"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// `[opcache] file_cache = false` turns the cache off: the directory is never
/// created, and the program still runs.
// covers: tools:cli/the-compile-cache
#[test]
fn file_cache_false_stores_nothing() {
    let dir = private_scratch("off");
    let cache = dir.join("artifacts");
    let config = dir.join("nvs.toml");
    std::fs::write(
        &config,
        format!(
            "[opcache]\nfile_cache = false\nfile_cache_dir = '{}'\n",
            cache.display()
        ),
    )
    .expect("the configuration is written");
    let program = dir.join("main.nvs");
    std::fs::write(&program, "<?nvs\necho 'ran', \"\\n\";\n").expect("the program is written");

    let (out, _) = run(&config, &program);
    assert_eq!(out, "ran\n");
    assert!(!cache.exists(), "nothing was stored: {}", cache.display());

    let _ = std::fs::remove_dir_all(&dir);
}

/// One program run under two configurations that differ only in their
/// `[[extension]]` array: each writes its own artifact, and a run under either
/// one again writes nothing. A unit compiled under one set of extensions is
/// never loaded under another (`rule:config/the-extension-set-is-in-every-unit-key`).
/// The entry is the conformance fixture `ledger`, under its real pin, because
/// `nvs run` reads the manifest of every entry before it compiles.
// covers: tools:cli/the-compile-cache
#[test]
fn a_unit_compiled_under_one_extension_set_is_never_reused_under_another() {
    let dir = private_scratch("extension-set");
    let cache = dir.join("artifacts");
    std::fs::create_dir(&cache).expect("the cache directory");
    let opcache = format!("[opcache]\nfile_cache_dir = '{}'\n", cache.display());
    let plain = dir.join("plain.toml");
    std::fs::write(&plain, &opcache).expect("the configuration is written");
    let fixtures = nvs_repo::path("tests/conformance/ext/fixtures");
    std::fs::copy(fixtures.join("ledger.nvsx"), dir.join("shop.nvsx"))
        .expect("the fixture is copied");
    let pin = std::fs::read_to_string(fixtures.join("ledger.sha256")).expect("the fixture's pin");
    let shop = dir.join("shop.toml");
    std::fs::write(
        &shop,
        format!(
            "{opcache}\n[[extension]]\npath = 'shop.nvsx'\nsha256 = \"{}\"\n",
            pin.trim()
        ),
    )
    .expect("the configuration is written");
    let program = dir.join("main.nvs");
    std::fs::write(&program, "<?nvs\necho 'ran', \"\\n\";\n").expect("the program is written");

    assert_eq!(run(&plain, &program).0, "ran\n");
    let first = artifacts(&cache);
    assert_eq!(first.len(), 1, "the first run stored its program");

    assert_eq!(run(&shop, &program).0, "ran\n");
    let both = artifacts(&cache);
    assert_eq!(
        both.len(),
        2,
        "the run with an extension compiled the program again"
    );

    run(&plain, &program);
    run(&shop, &program);
    assert_eq!(
        artifacts(&cache),
        both,
        "each configuration loaded its own artifact and wrote no other"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
