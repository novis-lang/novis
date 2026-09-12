# Handoff

## State

**Goal `webcrypto`: stages 1–7 are complete, and stage 8 — the rulebook — is all that is left.** Its
three checks (`docs/agent/loop-goal.toml:9185-9203`) ask that `core-classes/crypto-interop-tier`,
`security/jwe-compact-subset` and `security/jws-issued-subset` each read `shipped`; all three still
read `designed`, and nothing else about them is open.

**Stage 2's three checks were red on the tool, not on the tree.** The rules were written and correct;
`tools/rules.py --show` died printing a rule body on a cp1252 console, so the check exited 1 after
matching its `want`. Fixed at `tools/rules.py:445-450`, and the playbook's *Tooling* bullet is the
general shape of that failure.

**`Core\Jwe::decrypt` takes the token a request handed in.** Its `$token` is `Qual::Neutral`
(`crates/nvs-stdlib/src/jwe.rs:194`) for `Core\Jwt::verify`'s reason: the payload is `tainted` on the
way out whatever the token was, so contagion has nothing left to carry and
`admits_tainted_argument` (`crates/nvs-types/src/expr/quals.rs:307`) would only have refused the
ordinary call. `encrypt`'s payload stays `Qual::Contagious` — its return is plain `Str`, so the
qualifier still has somewhere to go, and the new case proves it does.

`nvs-host`'s two CPU-charging watchdog tests stay the known flake
(`crates/nvs-host/src/watchdog.rs:1051` and `:1103`): one fails per run under load, a different one
each time, and each passes alone.

## Next group

**Stage 8: the rulebook** — one file set: `docs/rules/core-classes.json`, `docs/rules/security.json`
and whatever `--render` writes from them. `rule:core-classes/crypto-interop-tier`,
`rule:security/jwe-compact-subset`, `rule:security/jws-issued-subset`.

- [ ] **Flip the interop tier to `shipped`** — the `status` field of the rule object at
      `docs/rules/core-classes.json:692`, `rule:core-classes/crypto-interop-tier`. The goal's whole
      surface is on disk and its conformance cases are green, which is what the status asserts.
- [ ] **Flip both JOSE subsets to `shipped`** in one edit — `docs/rules/security.json:1437`
      (`rule:security/jwe-compact-subset`) and `docs/rules/security.json:1454`
      (`rule:security/jws-issued-subset`) — then re-render with `python tools/rules.py --render`,
      which rewrites `docs/rules/*.md` and `docs/ground-rules.md`.
- [ ] **Regenerate `docs/novis.md` rather than hand-editing it.** It is filtered to rules whose
      status is `shipped` (`tools/rules.py:35`), so it grows by these three; `tools/reference.py`
      owns it, and which flag renders it is not checked. `python tools/verify.py --doc` before the
      `DONE` status, as the session prompt's step 6 requires.

## Backlog

- The Rust-side half of the tainted-token claim has no home: nothing under `crates/nvs-types/tests/`
  names a `Core` member, and `core_lib.rs`'s per-slot audit (`crates/nvs-types/src/core_lib.rs:938`)
  already pins the row's mark.
- JWE-encrypted ID tokens, key-set fetching and caching, and structured claims under a shared key are
  out of this goal by its § *Standing decisions*.
- `crates/nvs-host/src/watchdog.rs:1051` and `:1103` flake under load; nothing owns the repair.
