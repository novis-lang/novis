# Handoff

## State

**M4's Stage 8, and `Core\Uri` is the class the work is in**: two of its three remaining
edges landed, so the tree is at **753 conformance plus 189 differential**. Both slices
found the member disagreeing with its own doc comment rather than only lacking a case,
so `crates/nvs-stdlib/src/uri.rs` moved in two places:

- **`tryParse` folded only one of `parse`'s two throwing steps.** `read`'s grammar
  refusal became `null` and `built`'s port refusal propagated, so
  `Core\Uri::tryParse("//h:99999/")` threw where the member's own doc says it answers
  "`parse` exactly, with `null` where it throws". The whole body is folded now
  (`uri.rs:1443`), which is what
  `uri-parse-and-try-parse-are-one-reader-asked-two-ways.nvst` counts: 21 subjects, 14
  read and 7 refused, asserting the two **agree** rather than what either answered.
- **`with`'s port bound was drawn at one end only.** The option is declared `int`, so it
  can be spelled below the range as well as above it, and a negative one recomposed to a
  `-` inside the authority and came back as "a byte the URI grammar does not admit" — a
  true sentence about the wrong thing. `port_out_of_range` (`uri.rs:795`) is the one rule
  both ends now draw, called from `port_of` and from `with` (`uri.rs:1585`).
  `uri-port-is-a-tcp-port-at-both-ends-of-its-bound.nvst` names `0`/`65535` beside `-1`
  and `65536`, with `00080`, the empty port and the absent one on the reader's side.

`python tools/loop.py --list` still reports no named `.nvst` case owed by any stage, and
`gaps.py --differential` still ranks 0.

## Next group

**`Core\Uri`'s last edge, then `Core\Random`** — the file set is
`crates/nvs-stdlib/src/uri.rs` and `tests/conformance/core/` for the first, which is what
this session had open, and `crates/nvs-stdlib/src/random.rs` for the second (`gaps.py`
ranked it 0.86, second-thinnest, behind the `Core\Uri` this session moved).

- [ ] **`scheme` and `host` are the two components `with` has no *grammar* boundary case
      for** (`uri.rs:964` `unmoved`, `uri.rs:926` `recompose`, the two option rows at
      `uri.rs:~460`) — spec § 12. The standing cases pin case-preservation
      (`uri-compares-by-normalized-components.nvst`) and the component that *moves*
      (`uri-with-refuses-a-component-that-moves.nvst`); what none asks is where each
      component's own grammar stops — RFC 3986 § 3.1's `ALPHA *( ALPHA / DIGIT / "+" /
      "-" / "." )` for a scheme, so `with({scheme: "1x"})` and `with({scheme: ""})`, and
      an empty `host` against an absent one, which are two different authorities.
- [ ] **`Core\Random`'s bounds** (`crates/nvs-stdlib/src/random.rs`) — the *bound at both
      ends* shape over the range members, and the invariance a sweep asserts by counting.
      Read the module's own doc comment for which members carry a refusal before writing
      rows.
- [ ] **`Core\Time\DateTime`** (0.88, third) — only if the two above leave room; it is a
      different file set.

## Backlog
- `Core\Uri::with({port:})` now narrows to `u16` before recomposing, so `port_of` inside
  `built` is a second, unreachable check on that path — harmless, and deliberately kept
  as the one place text-derived ports are judged (`docs/spec/01-core-library.md` § 12).
- `gaps.py` ranks `Core\Time\DateTime` (0.88) after `Core\Random`; neither shares a file
  with `uri.rs`.
- ADR 0092 § 6's `Throwable` record producer still waits on the crate edge above
  (`nvs_stdlib::debug`'s own known gaps).
- A `require` whose path is not a string literal runs nothing, silently, in both forms
  (`nvs_hir::requires`' own known gap).
