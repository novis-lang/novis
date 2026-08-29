# Handoff

## State

**The driver's failing acceptance check is closed.** `valgrind examples/routes.nvs` leaked 292
bytes in two blocks — one array header per `Core\Router::url` call — because
`nvs_ir::lower::lower_route_link` builds its two arguments by hand instead of through
`lower_call_args` and never called `account_for_arg` on either. Both are staged now, and
`a_resolved_route_link_releases_its_params_array` in `crates/nvs-ir/src/lower/tests.rs` pins it
over the call's own argument `ValueId`. The playbook bullet under *Writing Novis itself* owns the
trap. This was not a new-work regression: the sweep is only *reached* now that stage 7 passes.

**Item 12's roster: 34 members across seven classes remain.** `Core\Bytes`, `Core\Path` and
`Core\Time` are off `UNCLASSIFIED` at `crates/nvs-stdlib/src/registry.rs:1526`. `Core\Path`'s nine
were the two ordinary bullets of the rule at `registry.rs:74` — `isAbsolute` answers a `bool` and
is `Neutral`, the other eight are `Contagious` — with `normalize`'s **non**-laundering recorded in
`path.rs`'s module doc, because it is the member most likely to be mistaken for one.

**`Core\Time`'s seven rows are the roster's first class with no `Contagious` parameter at all**,
and that judgement is the one to read before classifying `Core\Uri`: every pattern is `Sink`
(R11's third grammar, at the three `format` members and `Core\Time::parse`'s *second* argument),
and every parsed text is `Neutral` because an instant, a magnitude and an IANA roster entry are
**closed spaces** no byte of the argument survives into — ADR 0024 § 2's checked conversion
reached at a member. `time.rs`'s module doc § *What these members do with a qualifier* owns it,
including the line it draws against `Core\Bytes::at` and the warning that `Core\Uri::parse` is on
the *other* side: a `Uri` keeps the host and the path as text.

**Nothing consumes `Qual` yet** — it is declarative in `nvs-stdlib` and no `nvs-types` code reads
`classification()`, so a classification pass cannot change what a program does today.

**Untouched:** `crates/nvs-stdlib/src/router.rs:39`'s *Known gaps* 1 and 2 are stale, a `bytes`
array key ICEs in `nvs-ir`, and `catch (Core\Error $e)` panics — the last two have playbook
bullets under *Writing a test case*.

**Orientation gaps.** Still owed from before, none of them added yet: `docs/spec/02-php-migration.md`
and `tools/check-migration.py` in `[context] docs`, the stage-7 comment header's per-goal floor
table, a selector that prints the *failing* check's own `cases` block, `[context] modules` naming
an `nvs-stdlib` class module, a `[context] anchors` entry for `registry.rs`'s `UNCLASSIFIED`, and
ADR 0088 § 1, 0063 R11, `0085 §§ 1-4`, `0071 §§ 2, 7`. New this session: `[context] modules` needs
`nvs-ir/src/lower/*` — the failing check routed a whole session into lowering and the pack named
none of it.

## Next group

**Shared file set:** `crates/nvs-stdlib/src/registry.rs` — every slice deletes its class's lines
from `UNCLASSIFIED` at `registry.rs:1526` and reads the rule at `registry.rs:74` — plus one class
module each. Item 12 continues in roster order. `Sink` is only ever R11's four grammars, and
`time.rs`'s new § *What these members do with a qualifier* is the precedent for a parsed value.

- [ ] **`Core\Uri`'s nine rows** (ADR 0088 § 2) — `crates/nvs-stdlib/src/uri.rs:343`. The
      component encoders are the class's whole point: `encodeComponent`/`encodeFormValue` are
      **`Launder`** and their doc comments name the sink (the URL path, the form body), while
      `decodeComponent`/`decodeFormValue` are `Contagious` — decoding gives content back. `parse`
      and `tryParse` are `Contagious`, not `Neutral`: unlike `Core\Time`'s parses a `Uri` keeps
      the host and the path as text, which `time.rs`'s module doc says out loud.
- [ ] **`Core\Test`'s eight rows** (ADR 0088 § 2) — `crates/nvs-stdlib/src/test.rs:365` is
      `assertCount`'s body; the rows are in that module's `CLASS`. Every assertion answers `void`
      or throws, so no argument's bytes reach an answer and all eight are `Neutral` — the
      roster's most mechanical class.
- [ ] **`Core\Validate`'s six rows** (ADR 0088 § 2) — `crates/nvs-stdlib/src/validate.rs`. Each
      answers a `bool` about its subject, so `Neutral` throughout by the rule's first bullet; the
      one to look at twice is any member answering the *value* rather than a verdict.

## Backlog

- `Core\Hash` 4, `Core\Uuid` 2, `Core\Router` 2, `Core\Csv` 2 — the rest of item 12's roster.
- `crates/nvs-stdlib/src/router.rs:39`'s *Known gaps* 1 and 2 are stale now that the table lands.
- A `bytes` array key ICEs in `nvs-ir` (playbook, *Writing a test case*).
- `catch (Core\Error $e)` panics rather than diagnosing (playbook, *Writing a test case*).
- `docs/agent/loop-goal.toml`'s `[context]` gaps, listed under *Orientation gaps* above.
- `python tools/check-migration.py` stands at 34%; goals 2-6 own the remaining families.
