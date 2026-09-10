# Handoff

## State

**Goal `config-is-written` — the configuration is written down, and every key in it is read — has just started;
nothing of it has landed yet.** Goal `resource-ceilings`'s whole acceptance list is this goal's floor.

Two halves, and neither is honest without the other.

**The configuration is implicit.** `nvs_config::resolve::roots` takes `--config`, else `./nvs.toml`
in the working directory and never a walk upward, else the shipped defaults — and the third of those
leaves nothing on disk. A deployment running on deny-all capabilities and `production` mode has no
file saying so, so the only way to inspect what it is under is to read the rulebook.

**And part of the surface is inert.** `crates/nvs-config/src/tree.rs` parses 174 leaf keys under
`deny_unknown_fields`; a hand-audit found nine that reach no reader. `[cache] dir` is classified
`System`/`Boot` in `directive.rs` and read by nothing — the artifact cache reads
`opcache.file_cache_dir` in `crates/nvs-cli/src/cache.rs:@from_config`, and `Cache::dir`'s only
mentions outside `tree.rs` are in `crates/nvs-config/tests/`. `[server] socket_mode` reaches no code
in the workspace at all. `[metrics] listen`, `[metrics] endpoint` and `[trace] endpoint` name
addresses nothing binds or pushes to, which `crates/nvs-server/src/metrics.rs`'s own module doc says
in four words. `capabilities.debug.trace` and `debug.profile` are grantable — `Cap::DebugTrace` and
`Cap::DebugProfile` have names, rows and lookups — and `Core\Debug` registers only `dump` and
`render`, so no door ever asks either. `[[extension]] path` and `sha256` are folded into the artifact
cache's `env_hash` and acted on nowhere else.

**That list is what one hand-audit found, not the answer**, which is why stage 0 is a tool and not a
document. Both spellings of a reader are load-bearing: `Core\Storage` reads its disk root as
`config.get("storage.{disk}.root")`, so a search for field access alone reports a live key as dead.

The three design decisions are settled and are not to be re-litigated: **every key in the generated
file is commented out** (so the shipped defaults still apply and a later tightening still reaches a
deployment that took the file), **only project commands write it** (`run`, `serve`, `test`, `build`,
`check` — never an audit, never the LSP), and **an unimplemented key is emitted and marked** rather
than hidden or removed from the parser.

## Next group

**Stage 0 alone.** It is one Python tool over one Rust file, it shares its file set with nothing
above it, and every stage after it consumes its output — so a stage 1 taken before it exists is a
hand-audit that stage 0 then has to re-derive.

- [ ] **Write `tools/directives.py`** on `tools/lints.py`'s shape: derive the leaf-key roster by
      walking `Config`'s field graph in `crates/nvs-config/src/tree.rs`, carrying the dotted key and
      giving a map block (`[db.<name>]`, `[mail.<name>]`, `[storage.<name>]`) a `<name>` segment.
- [ ] **Count a reader either way** — a field touched outside `tree.rs`, *or* the dotted key present
      as a string literal. Put the reason in the tool's own doc comment; a future reader will
      otherwise delete the second half as redundant and silently mark eight live keys dead.
- [ ] **Define the `[unread: … owner: …]` trailer** and read it out of the field's own doc comment.
      The field is the key, so the field's comment is the one home; do not add a manifest file.
- [ ] **Make `--check` fail both ways.** No reader and no trailer is a key that landed silently; a
      trailer over a key that now *has* a reader is the failure that matters more, because that is
      the one that makes stage 2's generated file lie.
- [ ] **Wire it into `tools/verify.py`** beside `lints` and `reference`, before the compile steps, on
      the argument that doc comment already makes.
- [ ] **Expect the first `--check` to be red, and leave it red.** Fixing what it finds is stage 1;
      landing the tool green by pre-marking every key is how the gate becomes a rubber stamp.
- [ ] **Stage 1 is the next group after this one** and shares this file set: `tree.rs`,
      `directive.rs`, `capability.rs` and `crates/nvs-cli/src/cache.rs`. Its first item is the
      `cache.dir` decision — which spelling survives — and that one is a decision record with a
      migration note, not a cleanup, because removing a key from a `deny_unknown_fields` struct turns
      a silently-accepted file into a refused one.
