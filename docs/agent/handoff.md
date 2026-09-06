# Handoff

## State

**ADR 0042 § 3 is whole and `nvs run` uses it.** A verified payload is placed, relocated, protected,
bound and assembled into an ordinary `nvs_codegen::Unit` — `Descriptors::bind` and
`Descriptors::into_unit` (`crates/nvs-codegen/src/lib.rs:723`) are the loader's half, reading
addresses through the `nvs_codegen::Placed` trait that `cache::Loaded` implements. `cache::unit_for`
(`crates/nvs-cli/src/cache.rs:1141`) is the compile site's whole decision and `run_run` makes it, so
a second run of one program compiles nothing. Stage 3's six named tests are on disk and green.

**The compile now sits *below* the boot snapshot in `run_run`**, because both halves of the key are
configuration: § 7's `[opcache]` says where artifacts live, and ADR 0078 § 4's `env_hash` covers the
extension set. The module doc's *Known gaps* owns what is still unwired (`nvs test`, `spawn script`,
`serve`) and the one hazard: `env_hash` spells "the compiler build" as the package version, so
`cache::default_dir` keys its directory on the running executable and a *configured*
`opcache.file_cache_dir` does not. Nothing is blocked.

## Next group

**The measurement, then the other three producers of a unit.** One file set —
`crates/nvs-cli/src/cache.rs`, `crates/nvs-cli/src/runner.rs`, `crates/nvs-cli/src/script.rs`.

- [ ] **Stage 4's margin: `a_warm_start_is_faster_than_a_cold_one_by_the_margin_this_test_names`**,
      ADR 0042 § *Verification*'s warm-versus-cold number and the one that would say this cache is
      not worth having. Both halves are already callable in one process: `unit_for` with an empty
      cache is the cold arm and with a populated one the warm arm
      (`crates/nvs-cli/src/cache.rs:1141`), and `lowered` in the tests
      (`crates/nvs-cli/src/cache.rs:1755`) is the front end both share, so the margin measured is
      codegen against place-relocate-bind and nothing else. Write the threshold from what it prints,
      and say in the test's own doc that the front end is inside *neither* arm.
- [ ] **Consult the cache in `nvs test`'s runner**, `crates/nvs-cli/src/runner.rs:400`, whose comment
      already says the suite is compiled before any snapshot is resolved — the same reorder
      `run_run` just took, and then one `cache::unit_for` call with `cache::program_digest`.
- [ ] **Consult it for a `spawn script` isolate**, `crates/nvs-cli/src/script.rs:190`, which already
      holds an `[opcache]` policy and a content digest per path
      (`crates/nvs-cli/src/script.rs:434`) — so the missing
      half is the artifact key rather than the hash, and `Cache` is `Send`-hostile only in that its
      `Loaded` pages are per-process, which is what the isolate wants anyway.

## Backlog

- ADR 0078 § 4's `env_hash` cannot distinguish two builds of one version — `crates/nvs-cli/src/cache.rs`'s module doc.
- `serve.rs:264`'s compiler is the fourth unwired producer — same shape as the runner's.
- `aarch64` is never loaded: `HOST_ARCH` is `Unknown` there — the module doc's *Known gaps*.
