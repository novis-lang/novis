//! `nvs cache gc` and `nvs cache clear` —
//! `rule:packaging/nvs-cache-gc-and-clear-are-the-deterministic-escape`'s two commands over the
//! artifact cache in [`crate::cache`].
//!
//! **The store is the one a run reads.** Both commands resolve the configuration tree the way
//! `nvs tmp clean` does and hand it to [`cache::from_config`], so `opcache.file_cache_dir`, § 5's
//! ownership check and its one warning are decided in one place. A tree that places no cache — the
//! cache is off, or the written directory was refused — has nothing to remove, and the command says
//! so with the same line as an empty store.
//!
//! **Which files go is [`Cache::gc`]'s and [`Cache::clear`]'s answer.** Only published `.nvsc`
//! entries, never a `.tmp-` file a writer may still have open. An entry the platform keeps — on
//! Windows, a file another process has open — is counted as skipped, and the command still exits
//! `0`: the next `gc` takes it, and a cron job that fails over it fails over nothing.
//!
//! Cost: one walk of the cache directory per command, which is what § 6's roll already pays on a
//! miss. Nothing here runs on a request.

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use nvs_diagnostics::{Diagnostics, SourceMap};

use crate::cache::{self, Cache, Swept};
use crate::config::{LocalFiles, working_directory};
use crate::render_diagnostics;

/// Remove compiled programs from the compile cache.
///
/// Novis keeps each compiled program in a cache folder, so the next run starts
/// without compiling. These commands only change how much disk the cache uses.
/// The next run of a removed program compiles it again.
#[derive(clap::Args)]
pub(crate) struct Args {
    #[command(subcommand)]
    command: Command,
}

/// `nvs cache`'s own subcommands.
#[derive(clap::Subcommand)]
enum Command {
    /// Remove the oldest entries if the cache is larger than its size limit.
    ///
    /// The limit is `opcache.file_cache_max_size`. If the cache is larger, this
    /// removes the oldest entries until the cache is at 80% of the limit.
    /// Otherwise it removes nothing.
    Gc {
        /// Print the result as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Remove every entry from the cache.
    Clear {
        /// Print the result as JSON.
        #[arg(long)]
        json: bool,
    },
}

impl Args {
    /// Runs the `nvs cache` subcommand clap parsed, over the run's `--config` list.
    pub(crate) fn run(self, config: &[PathBuf]) -> ExitCode {
        let (json, walk): (bool, fn(&Cache) -> std::io::Result<Swept>) = match self.command {
            Command::Gc { json } => (json, Cache::gc),
            Command::Clear { json } => (json, Cache::clear),
        };
        let cache = match placed(config) {
            Ok(cache) => cache,
            Err(code) => return code,
        };
        sweep(cache.as_ref(), walk, json, &mut std::io::stdout())
    }
}

/// The cache a run over `config` reads, with any diagnostic or warning already printed. `None`
/// is a tree that places no cache.
fn placed(config: &[PathBuf]) -> Result<Option<Cache>, ExitCode> {
    let files = LocalFiles;
    let mut sources = SourceMap::new();
    let resolved = working_directory().and_then(|cwd| {
        let roots = crate::config::roots_in(config, &cwd);
        nvs_config::resolve::resolve(&roots, &mut sources, &files)
    });
    match resolved {
        Ok(resolved) => Ok(cache::from_config(&resolved.config)),
        Err(diagnostic) => {
            let mut diags = Diagnostics::new();
            diags.report(diagnostic);
            render_diagnostics(&mut diags, &sources);
            Err(ExitCode::FAILURE)
        }
    }
}

/// `walk` over `cache`, and its one line or JSON document on `out`. The whole of what the
/// command does once the cache is decided, so a test can point it at a directory it built.
fn sweep(
    cache: Option<&Cache>,
    walk: fn(&Cache) -> std::io::Result<Swept>,
    json: bool,
    out: &mut impl Write,
) -> ExitCode {
    let swept = match cache.map(walk).transpose() {
        Ok(swept) => swept.unwrap_or_default(),
        Err(error) => {
            let dir = cache.map_or_else(String::new, |cache| cache.dir().display().to_string());
            eprintln!("error: could not read the cache folder `{dir}`: {error}");
            return ExitCode::FAILURE;
        }
    };
    let _ = writeln!(out, "{}", report(swept, json));
    ExitCode::SUCCESS
}

/// The line `nvs cache` prints for `swept`, or with `json` its document. The document is written
/// by hand because `serde_json`'s map sorts its keys, and the order here is the one the rule states.
fn report(swept: Swept, json: bool) -> String {
    let Swept {
        removed,
        freed,
        skipped,
    } = swept;
    if json {
        format!(r#"{{"schema":1,"removed":{removed},"freed_bytes":{freed},"skipped":{skipped}}}"#)
    } else {
        format!("removed {removed} entries, freed {freed} bytes, skipped {skipped}")
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::time::{Duration, SystemTime};

    use nvs_config::Config;
    use nvs_config::cache::{artifact_key, content_hash, env_hash};
    use serde_json::json;

    use super::*;
    use crate::cache::{Eviction, HEADER_LEN};

    /// A cache of `units` one-kilobyte entries under `cap`, oldest first, in a private scratch
    /// directory, and the path of each entry.
    fn filled(name: &str, cap: u64, units: u8) -> (nvs_repo::Scratch, Cache, Vec<PathBuf>) {
        let dir = nvs_repo::scratch_private(name);
        let cache = Cache::new(
            dir.path(),
            env_hash(&Config::default(), &nvs_config::grants::Grants::default()),
        )
        .expect("a scratch directory of this test's own")
        .with_eviction(Eviction {
            max_size: cap,
            probability: 0,
            divisor: 0,
        });
        let mut paths = Vec::new();
        for unit in 0..units {
            let payload = vec![unit; 1024];
            let key = artifact_key(content_hash(&payload), cache.env());
            cache.store(key, &payload).expect("writable");
            let path = cache.path(key);
            fs::File::options()
                .write(true)
                .open(&path)
                .expect("the entry is writable by the account that wrote it")
                .set_modified(SystemTime::now() - Duration::from_secs(u64::from(60 - unit)))
                .expect("the filesystem records a modification time");
            paths.push(path);
        }
        (dir, cache, paths)
    }

    /// What `walk` printed over `cache`, and the exit code.
    fn printed(
        cache: Option<&Cache>,
        walk: fn(&Cache) -> std::io::Result<Swept>,
        json: bool,
    ) -> (String, ExitCode) {
        let mut out = Vec::new();
        let code = sweep(cache, walk, json, &mut out);
        (String::from_utf8(out).expect("the report is UTF-8"), code)
    }

    /// A half-written entry beside the published ones, named as a writer names its temp file.
    fn temp_beside(entry: &Path) -> PathBuf {
        let temp = entry.with_file_name(".tmp-0123456789abcdef");
        fs::write(&temp, [0u8; 4096]).expect("writable");
        temp
    }

    /// Over the cap, `gc` removes the oldest entries down to the floor and keeps the newest.
    #[test]
    fn gc_over_the_cap_removes_the_oldest_entries_down_to_the_floor() {
        let (dir, cache, paths) = filled("cache-cmd-gc-over", 4 * 1024, 6);
        let (line, code) = printed(Some(&cache), Cache::gc, false);
        assert_eq!(code, ExitCode::SUCCESS);
        // Six entries of 1 KiB and a header each, against a 4 KiB cap and a floor at 80% of it.
        // Two entries fit under the floor, so four go.
        let removed = paths.iter().filter(|path| !path.exists()).count();
        assert_eq!(removed, 4, "{line}");
        assert!(!paths[0].exists() && paths[5].exists(), "oldest first");
        let freed = 4 * (1024 + HEADER_LEN);
        assert_eq!(
            line,
            format!("removed 4 entries, freed {freed} bytes, skipped 0\n")
        );
        drop(fs::remove_dir_all(dir.path()));
    }

    /// Under the cap, `gc` removes nothing and still exits `0`.
    #[test]
    fn gc_under_the_cap_removes_nothing_and_succeeds() {
        let (dir, cache, paths) = filled("cache-cmd-gc-under", 64 * 1024, 3);
        let (line, code) = printed(Some(&cache), Cache::gc, false);
        assert_eq!(code, ExitCode::SUCCESS);
        assert_eq!(line, "removed 0 entries, freed 0 bytes, skipped 0\n");
        assert!(paths.iter().all(|path| path.exists()));
        drop(fs::remove_dir_all(dir.path()));
    }

    /// `clear` removes every published entry and leaves a writer's `.tmp-` file where it is, and
    /// `--json` prints the same counts as a document.
    #[test]
    fn clear_removes_every_entry_and_keeps_a_temp_file() {
        let (dir, cache, paths) = filled("cache-cmd-clear", 64 * 1024, 3);
        let temp = temp_beside(&paths[0]);
        let (text, code) = printed(Some(&cache), Cache::clear, true);
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(paths.iter().all(|path| !path.exists()));
        assert!(temp.exists(), "a `.tmp-` file belongs to a writer");
        let document: serde_json::Value = serde_json::from_str(&text).expect("one JSON document");
        let freed = paths.len() * (1024 + HEADER_LEN);
        assert_eq!(
            document,
            json!({ "schema": 1, "removed": 3, "freed_bytes": freed, "skipped": 0 })
        );
        drop(fs::remove_dir_all(dir.path()));
    }

    /// A `.tmp-` file does not count toward the cap, so it cannot make `gc` evict an entry.
    #[test]
    fn gc_does_not_count_a_temp_file_toward_the_cap() {
        let (dir, cache, paths) = filled("cache-cmd-gc-temp", 4 * 1024, 3);
        let temp = temp_beside(&paths[0]);
        let (line, _) = printed(Some(&cache), Cache::gc, false);
        assert_eq!(line, "removed 0 entries, freed 0 bytes, skipped 0\n");
        assert!(temp.exists() && paths.iter().all(|path| path.exists()));
        drop(fs::remove_dir_all(dir.path()));
    }

    /// A cache folder that does not exist yet, and a tree that places no cache, are both an empty
    /// cache.
    #[test]
    fn a_missing_folder_and_no_cache_remove_nothing() {
        let dir = nvs_repo::scratch_private("cache-cmd-missing");
        let cache = Cache::new(
            dir.path().join("cache"),
            env_hash(&Config::default(), &nvs_config::grants::Grants::default()),
        )
        .expect("a scratch directory of this test's own");
        for cache in [Some(&cache), None] {
            let (text, code) = printed(cache, Cache::clear, true);
            assert_eq!(code, ExitCode::SUCCESS);
            assert_eq!(
                text,
                "{\"schema\":1,\"removed\":0,\"freed_bytes\":0,\"skipped\":0}\n"
            );
        }
        drop(fs::remove_dir_all(dir.path()));
    }
}
