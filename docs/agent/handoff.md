# Handoff

## State

**Spec § 14 is complete.** `Core\IO::walk` is registered over three conformance cases and answers a
`Core\IO\Walk`: every entry of the tree under a path, each a path relative to it, taken whole when
the member is called. `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt` is down from
47 keys to 43 and § 14 keeps none of them.

**`walk` is a tree where `list` is a directory, and that is the decision the slice turned on.** The
spec writes the two side by side, and ADR 0063 R6 admits only one reading of that pair — the same
entries in a second container would be one operation reachable two ways — so `list` reads one
directory as bare names and `walk` reads everything under it. That is also what makes the pair
cover the `DirectoryIterator` family the spec row names. `Core\IO\Walk`'s own doc comment owns that
decision and the three beside it: a symbolic link is an entry and is never descended into, so the
walk is finite whatever the links say; every directory the walk enters goes through
`capability::read_dir` rather than the root standing in for all of them; and the entries are held,
as `Core\IO\Lines` holds its lines and for that class's reasons. `held_strings` is now the one
reader both classes' `iterate()` goes through.

**`Core\Env`'s three constants are on disk** — `EOL`, `OS` and `VERSION`, with `OS` a closed
`PHP_OS_FAMILY` roster rather than `uname`'s free text. The gap note that made `EOL` a decision is
answered on that module's `CONSTANTS`: a `CoreConst` is folded when the program is *compiled*,
which is the machine that runs it, and `Core\Path::SEPARATOR` already rested on that reading.

**Stage 10's remaining open check is still the goal's own measure**: `check-migration --min 74`
reads 37%, and only registered members move it.

Nothing was missing from this session's pack.

## Next group

**§ 15's two remaining non-request members. Both are `crates/nvs-stdlib/src/registry.rs` plus
`tests/conformance/core/`; the first adds `env.rs` and a new `CoreEnum`, the second a module that
does not exist yet.**

- [ ] **Register `Core\Env::mode` and the `Env\Mode` enum it answers with.** ADR 0091's run mode,
      `Production` and `Development` and no third case. Spec § 15 is explicit that no environment
      variable is consulted for it, so this is the one member of that class which reads
      `Core\Config` instead of the environment — and `Core\Config` has no reader for the mode
      today, so where the value comes from is the first thing to settle. `crate::log::LEVEL` is the
      `CoreEnum` shape to copy. `crates/nvs-stdlib/src/env.rs:96`,
      `crates/nvs-stdlib/src/log.rs:90`, `crates/nvs-stdlib/src/registry.rs:1772`,
      `crates/nvs-config/src/mode.rs:1`.
- [ ] **Register `Core\Cap::has`, § 15's last member that needs no request.** ADR 0112 §§ 6, 8: it
      reports whether the *calling namespace* still holds a roster capability, so an unknown name
      is a compile error and the member itself grants nothing and declares a `None` row in
      `CAPABILITIES`. `nvs_config::Capabilities::allows` is the predicate it has to end at.
      `crates/nvs-stdlib/src/registry.rs:1079`, `crates/nvs-stdlib/src/registry.rs:1458`,
      `crates/nvs-config/src/capability.rs:272`.

## Backlog

- § 15's `Request`, `Response` and `Session` rosters need a request and are goal 6's —
  `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt`.
- § 18's `Core\Db` roster needs a driver and a server and is goal 5's — same file.
- §§ 16-17's classes are still a 9-key ratchet —
  `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt`.
- `check-migration --min 74` reads 37% and is the goal's own acceptance measure —
  `docs/agent/loop-goal.toml`.
