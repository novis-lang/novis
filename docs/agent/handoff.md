# Handoff

## State

**Spec § 15's half that needs no request is complete.** `Core\Env` was already whole;
`Core\Cap::has` landed this session, so what is left of § 15 is the three request classes.
`crates/nvs-stdlib/src/cap.rs`'s module doc owns the member: it reports and never widens, its
`registry::CAPABILITIES` row is `None` for `Core\Cache::local`'s reason, and a written name outside
ADR 0112 § 8's roster is `E0616` from `crates/nvs-types/src/capability.rs`.

**The compile-time half of ADR 0112 is still absent**, and `cap.rs`'s doc says what that costs:
§ 1's per-namespace `[grants]` table has no representation in `nvs_config`, so `has` answers the
request's whole grant table — the outer bound of what any namespace inside it can hold — and § 4's
`E0604` is not on disk either. ADR 0112's Diagnostics table now says out loud that its first four
codes were claims the band has since issued elsewhere.

**`crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt` is 41 keys and none of them is
this goal's**: 30 are § 15's request classes (goal 6's) and 11 are § 18's `Core\Db` (goal 5's).

**So `check-migration --min 74` is the goal's only open gate, and it is long.** It reads 37% — 428
of 1,152 inventory names classified, 724 open — and 74% needs roughly 425 more *rows* in
`docs/spec/02-php-migration.md`. That is many sessions of doc work, not one group; the group below
is the first three families of it, sized one family per slice.

## Next group

**Every slice is `docs/spec/02-php-migration.md` alone, and they share all three anchors**: new
family sections go in before `## Not yet classified`, whose closing paragraph names the domains
still to do and shrinks as they land. `python tools/check-migration.py --report` prints the open
names; `--min` is what the driver's gate reads.

- [ ] **Migration rows: the filesystem, directories and streams family.** `fopen`/`fread`/`fwrite`
      and the handle family against § 14's `Core\IO\File`, the `dir`/`scandir`/`glob` half against
      `Core\IO`, and every `stream_*` against ADR 0052 § 2's closed door — a wrapper is *dropped*
      with no replacement, which is a verdict the section has not had to write before.
      `docs/spec/02-php-migration.md:599`, `docs/spec/02-php-migration.md:13`,
      `docs/spec/02-php-migration.md:551`.
- [ ] **Migration rows: hashing, crypto and randomness.** `hash*`, `crc32`, `crypt`, `md5`/`sha1`
      and the `random_*`/`mt_rand` family against `Core\Hash`, `Core\Digest`, `Core\Password` and
      `Core\Random`, all four of which are landed and cased, so every row here names a member that
      exists. `docs/spec/02-php-migration.md:599`, `docs/spec/02-php-migration.md:13`,
      `docs/spec/02-php-migration.md:551`.
- [ ] **Migration rows: output buffering, processes and reflection.** The `ob_*` family against
      `Core\Out`, `proc_*`/`exec`/`shell_exec` against ADR 0044's argv-only `Core\Process` — the
      shell spellings are dropped, not mapped — and the `Reflection*` classes against ADR 0019's
      `Core\Reflect`. `docs/spec/02-php-migration.md:599`, `docs/spec/02-php-migration.md:13`,
      `docs/spec/02-php-migration.md:551`.

## Backlog

- ADR 0112 § 1's per-namespace `[grants]` table and § 4's `E0604`: unowned by any goal — `cap.rs`.
- `[context]` printed no `docs/spec/` section, and the three slices above are all spec prose:
  `docs/spec/02-php-migration.md:"## How to read a row"` is the shape they need — `loop-goal.toml`.
- `[context] adrs` still lacks ADR `0112` §§ 6 and 8; this session read them only because the item's
  own anchors inlined them — `loop-goal.toml`.
- § 15's three request classes and § 18's `Core\Db`: goals 6 and 5, not this one — the ratchet file.
- `Core\Cap::has` answers `false` for a computed name that is not a capability; if a case ever wants
  to assert that, it needs a string the folder cannot fold — `cap.rs`'s helper doc.
