# Handoff

## State

**Spec § 15's `Core\Env` is complete** — `get`, `all`, `mode` and the three constants. `mode`
answers ADR 0091's run mode as a `Core\Env\Mode` case, `Production` or `Development`, and that
enum is now in `registry::ENUMS`. Three conformance cases cover it.

**Where the mode comes from is `nvs_config::Request::mode`**, added this session, and its own doc
owns the four-step order — a flip this request made, the `[[app]]` block's mode, the global
`mode.default`, then `production`. It is deliberately not `Core\Config::get("mode.default")`,
which knows nothing about the `[[app]]` block. `started_ceiling` now shares its last two steps
through one private `started` rather than restating them. The two edges that reader has no
opinion about — an unconfigured context, and a mode nobody defined — are settled in `env.rs` on
`mode_ordinal`'s doc, and both are `Production`.

**`crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt` is 42 keys, and only one of them
is this goal's**: `§15 Cap::has`. The other 41 are the thirty § 15 request-class members (goal 6's)
and § 18's eleven `Core\Db` members (goal 5's). **So the ratchet stops being this goal's measure
after the next slice**, and `check-migration --min 74` — the stage-10 gate, reading 37% — becomes
the only open one. `python tools/check-migration.py --report` lists 724 unclassified PHP names;
each one is a *row* in `docs/spec/02-php-migration.md`, not a member, so that gate is now doc work
and the next group is sized for it.

**Two pack gaps.** `[context] adrs` needs ADR `0112` §§ 6 and 8 for the next slice. And
`[context] playbook` did not select the bullet *"A reference card may not cite an ADR"* —
`playbook.md:5312` — although the item named `registry.rs` and `env.rs`; it cost this session a
verify run, and the playbook already says it five times over.

## Next group

**The first slice is `crates/nvs-stdlib` plus `tests/conformance/core/`; the other three are
`docs/spec/02-php-migration.md` alone, one family per slice, and they share every anchor.**

- [ ] **Register `Core\Cap::has`, § 15's last member that needs no request and the last key this
      goal owns in the ratchet.** ADR 0112 §§ 6 and 8: it reports whether the *calling namespace*
      holds a capability at this point in the request, the argument is a roster name and an unknown
      one is a compile error. It grants nothing and needs no capability of its own, so it takes a
      `None` row like `Core\Cache::local` does. A new module beside `env.rs`, then the roster and
      the two registry rosters. `crates/nvs-stdlib/src/registry.rs:1079`,
      `crates/nvs-stdlib/src/registry.rs:1424`, `crates/nvs-stdlib/src/lib.rs:226`,
      `docs/adr/0112-authority-is-keyed-on-the-enclosing-namespace.md:184`,
      `docs/adr/0112-authority-is-keyed-on-the-enclosing-namespace.md:249`.
- [ ] **Migration rows: the filesystem, directories and streams families.** `fopen`/`fread`/
      `fwrite`/`fgets`, the `file_*` and `is_*` file predicates, `scandir`/`opendir`, `mkdir`,
      `copy`, `chmod`/`chown`, `clearstatcache`. Every one is answered by § 14's landed `Core\IO`
      and `Core\Path`, so this is transcription against members already on disk. A new section
      after *Paths*: `docs/spec/02-php-migration.md:518`,
      `docs/spec/02-php-migration.md:551`.
- [ ] **Migration rows: hashing, crypto and randomness.** `hash*`, `crc32`, `crypt`, `md5`/`sha1`,
      `random_*`, `uniqid` — against `Core\Hash`, `Core\Crypto`, `Core\Random` and `Core\Uuid`,
      all landed. Same file, same shape. `docs/spec/02-php-migration.md:551`.
- [ ] **Migration rows: output buffering, processes and reflection.** The `ob_*` family against
      `Core\Out`, `proc_*`/`exec`/`shell_exec` against ADR 0044's `Core\Process`, and the
      `get_class*`/`class_exists`/`method_exists`/`debug_backtrace` family against `Core\Reflect`.
      `docs/spec/02-php-migration.md:551`, `docs/spec/02-php-migration.md:599`.

## Backlog

- `differential` at `min_passing = 250` is stage 10's other open gate, at 210 — `docs/agent/loop-goal.toml:2624`.
- § 15's thirty request-class members and § 18's eleven `Core\Db` ones are goals 6 and 5, not this one — the ratchet's own comments say so.
- ADR 0091 § 3's defaults are applied only by the runtime flip and not at boot — `crates/nvs-config/src/mode.rs:25` states the gap.
- `docs/agent/doc-cleanup.md`'s rationale-bloat pass is user-fired and has not run this goal.
