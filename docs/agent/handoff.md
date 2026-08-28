# Handoff

## State

**M4's Stage 8, and `Core\Test`'s failure *rendering* now has its bounds.** The tree is at
**768 conformance plus 189 differential**. Three cases landed, all over
`crates/nvs-stdlib/src/test.rs`, and the whole named group is done.

- **A quoted `string` stops at 64 characters, counted in characters.** `SHOWN_CHARS = 64` at
  `test.rs:912`; a 64-character subject is quoted whole and a 65-character one takes a trailing
  `…` over the same 64. 65 two-byte characters cut at the same place, so a byte-wise bound is
  ruled out. The bound belongs to `shown`, not to a member: `assertEqualsDeep`'s named leaf
  goes through it too.
- **A container is named by its size and an object by its class, never by its contents.**
  `an array of 3`, `70 bytes`, ``a `Cell` `` — an empty array is `an array of 0` rather than a
  word of its own, and two `Cell`s holding different properties render identically. That is what
  makes the quoting bound worth anything: nothing else in a message can spill a subject.
- **A `secret` property is compared but not quoted.** `test.rs:830` replaces both sides with
  `«redacted»` at the property's own path, composed through containers as usual
  (`$actual->creds["0"]->token`). The descent still happens: an agreeing pair passes, a differing
  pair fails, and a non-secret sibling on the same class is quoted in full.

## Next group

**`Core\Test`'s remaining rendering arms and its two non-assertion members** — the file set is
`crates/nvs-stdlib/src/test.rs` plus `tests/conformance/core/`. `shown`'s container and `string`
arms are now pinned; what is left is the scalar half and the two members that are not an
assertion.

- [ ] **`shown`'s scalar arms each render as themselves** (`test.rs:872` `shown`, arms at `:874`
      `null`, `:876` `bool`, `:877` `int`, `:878` `uint`, `:879` `float`, `:880` `decimal`) — the
      *invariance over a sweep* shape: `false` renders as `false` and not as the empty string
      `bool as string` gives it, an `int` and a `uint` of the same digits render the same, and a
      `float` renders through Rust's own `to_string`. Reach these through `assertSame`, whose two
      arguments bind to one type variable, so each row needs a differing partner of its own type.
- [ ] **`assertEquals` over an object with no `compareTo` refuses, naming the two members that
      would work** (`test.rs:653`) — the *edges* shape. The message quotes the receiver through
      `shown`, so it names the class; assert it against a class that does implement `Comparable`
      on the line beside it, or the refusal reads as "objects are not supported".
- [ ] **`expectFailure` discharges an expectation and nothing else does** (`test.rs:531` the doc,
      `:579` the failure, `:584` `held`) — ADR 0079 § 5. A body whose assertion fails is the
      passing row; a body that passes is the refusal. The `callable` goes in as a `fn` literal at
      the call site, since a first-class `Class::method(...)` still panics `nvs-ir`.

## Backlog

- A `secret string` read out of its property and handed straight to `assertSame` is quoted in
  full — the redaction at `test.rs:830` is scoped to the *slot*, and the tag carries no
  qualifier. Whether that is intended belongs to ADR 0033 / ADR 0092 § 5, not to this module.
- `shown`'s `Tag::Resource` and `Tag::Unset` arms are unreachable from source; owed a comment
  saying so rather than a case (`crates/nvs-stdlib/src/test.rs`).
- 54 of the 156 guard tests `loop-goal.toml` names match nothing `cargo test` runs —
  `docs/agent/guard-name-debt.md`.
- `array<T> as array<U>` does not lower (`crates/nvs-ir/src/lower/expr.rs:877`), which is what
  keeps `Core\Csv::format`'s non-`string` cell refusal unreachable — `docs/agent/playbook.md`.
