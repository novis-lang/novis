# Handoff

## State

**M4's frontier is the two file-scope statement shapes `holes.py --cases` still named.** The
first is landed as one corpus case over already-landed work, taking that tool from 7 named cases
to 6; the second is a real multi-crate feature and is **not started** — this session stopped at
the point where it had located every blocker, rather than half-landing it. Nothing in `crates/`
changed.

- **`tests/conformance/lang/inline-html-at-file-scope-is-echoed-in-place.nvst`** pins a run's
  *placement* (in source order, per iteration, per taken arm, in a method body) and its agreement
  with an `echo` of the same literal, counted through `Core\Out::capture`. The plan's `Open now`
  carries what it asserts; the sibling `inline-html-is-written-verbatim.nvst` keeps *what* a run
  writes.
- **`require` still runs nothing of the required file's own top-level statements**, and the shape
  that closes it is now scoped rather than guessed: `nvs_hir::requires::resolve_program` is the
  only place that knows which file a written path resolved to, and neither `nvs_types::ProgramFile`
  nor the `ExprTypeTable` carries that edge, so `nvs-ir` at the site has the literal path text and
  nothing else. Re-deriving the resolution inside `nvs-ir` is the wrong direction — it would be a
  second copy of the canonicalization rule.
- **One decision that group has to take and record** (pre-authorized, no ADR): ADR 0021's "sharing
  the calling frame completely" is already false for *variables* in the tree, because
  `nvs_types::locals` checks each file's top-level body on its own, so a `require`d file's `$x` is
  not the caller's. The safe reading is that declarations cross a `require` and variables do not;
  fold one sentence saying so into ADR 0021 § *Decision* rather than leaving the body to disagree.
- **`orient.py`'s pack was complete for this item.** The two standing manifest gaps are unchanged —
  `[context] modules` has no `nvs-runtime` and no `nvs-diagnostics` entry.

## Next group

**`require` runs the required file's own top-level statements — `nvs-ir` gap 22, in three slices
that share one file set.** The files: `crates/nvs-hir/src/requires.rs`,
`crates/nvs-types/src/lib.rs`, `crates/nvs-cli/src/main.rs`, `crates/nvs-ir/src/lower/mod.rs` and
`crates/nvs-ir/src/lower/stmt.rs`.

- [ ] **Carry the resolved edge out of the graph walk.** `resolve_program` at
      `crates/nvs-hir/src/requires.rs:163` already turns each `(literal, span)` its `walk_expr`
      harvests (`crates/nvs-hir/src/requires.rs:874`) into a canonical `PathBuf`; record that as a
      `span -> SourceId` map on its `Loaded` output, and give `nvs_types::ProgramFile`
      (`crates/nvs-types/src/lib.rs:255`) the file's own id. `crates/nvs-cli/src/main.rs:245` and
      `:292` are the two builders of that list.
- [ ] **One script frame per file, called from the site.** `lower_program`
      (`crates/nvs-ir/src/lower/mod.rs:443`, doc comment at `:400` states today's
      `files[0]`-only rule and has to be corrected with it) lowers a `lower_script` for every file
      under a per-file label, and `ExprKind::Require` at `crates/nvs-ir/src/lower/stmt.rs:347`
      — today an empty arm — emits the `Call` to the label the site's span resolves to. ADR 0021
      § 2: it runs *every time* control reaches it, so this is a call and not a once-guard.
- [ ] **`tests/conformance/lang/a-required-file-runs-its-own-top-level-statements.nvst`**, the
      named case, plus the ADR 0021 sentence above. A multi-file case is `--FILE <path>--`
      repeated (`crates/nvs-test`'s module doc); pin a `require` inside a loop, since running once
      per reach is the half a once-guard would still pass.

## Backlog

- ADR 0021 § 3's value form (`$c = require 'config.nvs';`) is `E0704` and stays a gap until the
  frame above exists to return from — `crates/nvs-ir/src/lib.rs` gap 22.
- Four corpus cases remain named by `python tools/holes.py --cases`: the property observer, the
  delegated interface, the attribute retrieved by its own type, and the dump that redacts a secret.
- `docs/agent/loop-goal.toml`'s `[context] modules` names no `nvs-runtime` and no
  `nvs-diagnostics` — unchanged from the previous handoff.
