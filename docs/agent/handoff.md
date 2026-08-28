# Handoff

## State

**M4's Stage 6 is the frontier, and ADR 0023 now has the *Verification* section
M4's acceptance has named since it was written.** The `mixed` receiver's whole
deferral landed in the previous group and is not re-opened.

- **ADR 0023 § 1 is the only rule of that ADR verifiable before M5** — *2*'s live
  carrier is the `spawn` boundary and *3*'s is `Core\Serialize`, neither of which
  exists yet. The new `## Verification` section is the home of which case pins
  what, and of the M5 list the deleted trailing block used to carry.
- **`a-clone-copies-storage-without-running-anything.nvst` is the privileged-path
  case** — `readonly` survives the copy, no `PropertyObserver` fires, an object
  reached through a cloned `array<Leaf>` property stays shared.
- **`__clone` is unspellable, not merely uncalled**: `E0111` refuses the name
  where it is written. The ADR's own parenthetical said the opposite and is
  corrected; `nvs-ir`'s `clone_lowers_to_one_instruction_with_no_hook_call`
  snapshot is what verifies the no-hook rule instead.
- M4's acceptance names *Verification* sections for ADRs 0014, 0023, 0028, 0046
  and 0069. **Only 0028 is left without one** — 0069's is at
  `docs/adr/0069-array-combination-is-key-type-independent.md:201`.

## Next group

**ADR 0028's *Verification* section, the last one M4's acceptance still owes.**
One file set: `docs/adr/0028-closing-the-remaining-magic-methods.md`, plus
whatever `tests/conformance/` cases the section turns out to owe. It reaches no
crate, so take it as a fresh window.

- [ ] **Read what ADR 0028 decides and what the tree already pins** — the six
      rules are at `docs/adr/0028-closing-the-remaining-magic-methods.md:58`
      (`Stringable`), `:92` (no `__destruct`, and the paragraph the abandoned
      generator's `finally` folded into it), `:146` (`__isset`/`__unset`, and
      `unset()` on an object property refused), `:186` (`__debugInfo`), `:198`
      (`__set_state`) and `:209` (`__autoload`); *Consequences* is `:223` and the
      file ends at `:304` with no *Verification* section. `python tools/gaps.py`
      and `grep -rln "Stringable\|unset(" tests/conformance/` name what exists.
- [ ] **Write the section**, between *Consequences* (`:223`) and *Alternatives
      rejected* (`:259`), which is where ADRs 0014, 0046 and 0069 put theirs.
      State the things no case can assert — most of these rules are *absences*,
      and several magic names are unspellable under ADR 0029's casing rule for
      the reason `__clone` is, so say which rule a diagnostic verifies and which
      one only a lowering snapshot can.
- [ ] **Land whatever case the section names and the tree does not have**, one
      per rule, in `tests/conformance/class/` or `tests/conformance/reject/`
      depending on whether the rule runs or refuses.

## Backlog

- ADR 0069's *Verification* section exists (`0069-...:201`) but nothing has
  checked it against the tree since it was written — a read, not a rewrite.
- `nvs_hir::requires`' own known gap: a `require` whose path is not a string
  literal runs nothing at all, silently, in both forms.
- `nvs_stdlib::debug` known gap 1: an ADR 0036 shape literal's field and an
  `array<T>` element carry no `secret` bit (ADR 0033's unmodelled container axis).
- ADR 0024 § 5's `string as Core\Html\Markup` row waits on `Core\Html` (M7).
