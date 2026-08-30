# Handoff

## State

**ADR 0042's read half is on disk** beside its writer, in `crates/nvs-cli/src/cache.rs`. Four of
stage 5's six `cargo-named` tests now pass: the two the writer closed, plus
`an_artifact_is_verified_whole_before_any_page_is_executable` and `a_tampered_artifact_is_rejected`.
`Cache::load` maps the file read-only through `memmap2` (new in the workspace), checks magic,
`format_version`, `env_hash` and `payload_len`, hashes the payload, and yields a `Verified` — a type
constructed on exactly one code path, so § 3's ordering claim is held by the type rather than by a
caller. Which failures delete the file and which do not is the module doc's § 3 section.

**What a cache payload is, is decided and recorded** in that module's *Known gaps*: a
`cranelift-object` relocatable image, not a dump of the JIT's pages. Two independent reasons, both
in the doc — `cranelift_jit::JITModule` has no serialization, and `nvs-codegen`'s `emit.rs` bakes
class-descriptor and callee addresses in as `iconst` immediates with no relocation record, so a
byte-perfect dump would be wrong in the next process anyway. ADR 0048 is **not** the other half:
its § 2 makes a bundle carry source, feeding *into* this cache rather than out of it. The redesign
that follows — a second `Module` in `nvs-codegen`, and a named symbol for every baked address — is
in `## Backlog` and was not started, per the goal's standing decision.

**`nvs-cli` left the workspace's `unsafe_code = "forbid"`** for its own `deny` plus one
`#[expect]`, exactly as `nvs-config` did for ADR 0103 § 6; the playbook bullet has the shape and
the trap. `Cargo.toml`'s lint-policy comment now names all six such crates.

`orient.py`'s `[context] modules` still names `crates/nvs-host/src/budget.rs`, which never existed,
and `[context] adrs` should gain **0042 §§ 5-6** for the group below — this session had § 3 only.

## Next group

**ADR 0042's last two sections, which are the last two failing acceptance tests.** File set:
`crates/nvs-cli/src/cache.rs` (whole, ~530 lines) and `crates/nvs-config/src/trust.rs:110`, whose
`trust::check(&Path) -> Result<PathBuf, Untrusted>` is already public and already implements
0103 § 6's Unix *and* Windows halves — this is a call site, not a second implementation.

- [ ] **The cache directory's trust check** — ADR 0042 § 5, which is 0103 § 6 applied to
      `[cache] dir`: refuse the directory rather than the entry, once, before anything is read
      from it. Decide whether it sits in `Cache::new` (which today deliberately checks nothing —
      its doc says so) or in the caller that builds one; the test the driver spells is
      `a_world_writable_cache_directory_is_refused`.
      Anchors: `crates/nvs-cli/src/cache.rs:267`, `crates/nvs-config/src/trust.rs:110`.
- [ ] **Eviction, piggybacked and off the request path** — ADR 0042 § 6. The fan-out directory is
      the only thing there is to walk and § 1's module doc already says eviction is its one
      caller. Test: `eviction_is_piggybacked_and_off_the_request_path`.
      Anchors: `crates/nvs-cli/src/cache.rs:276`, `crates/nvs-cli/src/cache.rs:404`.
- [ ] **The warm-start bench**, once § 5 lands — `tools/bench.py --warm-start --max-ms 10` is
      stage 5's second check and nothing has run it against a real cache yet.
      Anchors: `crates/nvs-cli/src/cache.rs:307`.

## Backlog

- A relocatable payload: a `cranelift-object` `Module` beside the JIT's, and a symbol for every
  address `emit.rs` bakes in — `crates/nvs-cli/src/cache.rs`'s *Known gaps* states the whole of it.
- § 3's `mprotect` step, which that payload is the precondition for — same *Known gaps* entry.
- The compile-pipeline call sites for `store`/`load`, which `#![allow(dead_code)]` names.
- Item 18's `Core\Secret::reveal()` is not in the registry — `docs/plan/m6.md`.
- `Live::admit`'s same-class check is asked of the answer, not the argument —
  `crates/nvs-runtime/src/graph.rs` § *Known gaps*.
