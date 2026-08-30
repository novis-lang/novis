# Handoff

## State

**ADR 0103 § 6 is landed: the configuration tree has a trust boundary.**
`crates/nvs-config/src/trust.rs` is the check on both platforms — Unix is owner-or-root plus the mode
bits, Windows is the owner SID plus an effective-write sweep over the five principals § 6 now names — and
it reaches the tree as `Files::trust` (`crates/nvs-config/src/resolve.rs:51`), which every reader owes and
which hands back the **canonical** path. Two consequences are already spent: the cycle test sees one file
through two spellings, so `MAX_INCLUDE_DEPTH` is a nesting cap and no longer the symlink backstop, and
`[[app]]` matching has its path comparison written once, at `trust.rs:83`. `E0607 E_UNTRUSTED_CONFIG` is
the refusal; a path that cannot be examined at all stays `E0605`, because an operator told "cannot read"
goes looking for a typo when the answer is a mode.

Three things the ADR now states rather than the code deciding quietly: which ACEs Windows accepts, that
the containing directory is checked for its **owner** as well as its mode, and that an absent `optional`
include whose directory is also absent puts the check on the nearest ancestor that exists.

25 tests in the crate. `verify.py` has no Linux leg, so the two `cfg(unix)` cases in `tests/trust.rs` were
run under WSL with `CARGO_TARGET_DIR=/tmp/nvs-unix-check cargo test -p nvs-config` — worth repeating for
any future change to `trust.rs`, since the Windows half is all that CI-on-this-machine can see.

**A machine fact, not a code one:** `<repo>` and its `nvs.toml` grant `Authenticated Users` modify, which
is exactly what § 6 refuses. Nothing depends on `nvs-config` yet, so nothing refuses today; Stage 3's
snapshot will. The playbook bullet under *Running things* has the commands and says to ask first.

The acceptance check still fails on `examples/config.nvs` — `Core\Config` has no `get`. That is Stage 3's
snapshot and Stage 4's members, unwritten, not a regression. Nothing is blocked on the user.

## Next group

**The two indirections that sit on the boundary, plus the reader they both need.** One file set:
`crates/nvs-config/src/tree.rs`, `crates/nvs-config/src/resolve.rs`, `crates/nvs-config/src/trust.rs`,
`docs/adr/0103`, `docs/adr/0104`.

- [ ] **`password_file`.** ADR 0103 § 7 — the named file's whole content is the value, with **one**
      trailing newline stripped and interior spaces kept; empty, whitespace-only, non-UTF-8 and oversized
      refuse, and so does setting `password` and `password_file` together. The block is
      `crates/nvs-config/src/tree.rs:410` (`Database`), with the two fields at `:431` and `:433`; the
      secret file goes through `Files::trust` like any other config input (`resolve.rs:51`,
      `trust.rs:83`), which is what makes § 7's "a group-writable secret file refuses" free. Next free
      code in the band is `E0608`.
- [ ] **`[[app]]` matches and layers.** ADR 0104 § 2 — the entry path canonicalized, then prefix-matched
      on `root` or matched exactly on `entry`, every matching block layering least-specific first, bounded
      by `[limits.hard]`. The block is `tree.rs:138` (`App`). **Do not write a second path comparison**:
      `trust.rs:83`'s `check` already canonicalizes and simplifies, so split that half out as its own
      function and call it here — an entry file is not a config input and must not be ownership-checked,
      but it must resolve identically. The goal's standing decisions make this one implementation.
- [ ] **A `.nvst` or crate case that a symlinked include cycle is refused by name.** `resolve.rs:193`'s
      chain test now compares canonical paths, and nothing asserts it: the in-memory `Fake` cannot make a
      symlink, so this belongs in `tests/trust.rs`'s style, `cfg(unix)`-gated, next to the two cases that
      already run only under WSL.

## Backlog

- Stage 3's snapshot and `Core\Config::get`/`set` — the acceptance check's only remaining failure
  (`docs/agent/loop-goal.toml`, stage 3).
- ADR 0042 § 5's cache directory and ADR 0078 § 3's socket directory want the same check `trust.rs` now
  holds; neither has one, and neither should grow its own.
- `nvs config check` / `nvs config dump --origin` (ADR 0103 § 9) — the resolver already carries the
  origins and overrides those verbs print.
- `orient.py`'s `[context] modules` names `crates/nvs-host/src/budget.rs`, which matches nothing; the pack
  prints a warning every session.
