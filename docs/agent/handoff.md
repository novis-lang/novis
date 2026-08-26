# Handoff

## State

**Stage 4's two counts are the frontier — conformance 449 of 600, differential 90 of 150** — and the gap
is behavioural depth per member, not coverage: every registered member already has a case, and both of
Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean) and
`mwl test tests/conformance` is **449 passed, 0 failed** — run it as well as `verify.py`, which executes
no `.mwlt` case at all (playbook, twice).

**`Core\Heap` and `Core\Uuid` are each done to depth at 3 cases.** Heap is pinned at its *edges*: `peek`
on an empty heap throws with its own member name in the message (`pop`'s spelling was already pinned), a
drained heap is empty *and* reusable, one value pushed three times is held three times, the same object
pushed twice is answered twice under ADR 0090 § 4's identity `==`, three equal `compareTo` keys all come
out with the order among them deliberately unasserted, and `count`/`peek` track through interleaved
push/pop. Uuid is pinned by *invariance over a sweep*: 48 draws, each canonical, carrying its own version
nibble and RFC 9562's variant and round-tripping through `parse`, with no value seen twice; `tryParse`
swept against `parse` over one table of eight spellings, so their agreement is the assertion rather than
two members' separate rows; and ADR 0088's bounded quote asserted on both sides of its 48-character limit,
built with `Core\Str::repeat`.

**Three spellings this pass established, for the next case to reuse.** A `try`/`catch` at file scope works
and repeats — four in one case compile, each binding `$e`. A `for` sweep with `int` counters and a
`? 1 : 0` accumulator is how a case counts invariants without asserting a drawn value. An array literal is
`array<mixed>`, and a `var` in a loop body belongs to the function, so a second loop needs its own name —
both are the playbook bullet under *Writing a test case*.

## Next group

Three sections that have not had a depth pass, each its own domain module plus its
`tests/conformance/core/<name>-*.mwlt`. The file set they share is
`crates/mwl-stdlib/src/{path,json,random}.rs` and `tests/conformance/core/`.

- [ ] **`Core\Path` depth** — `crates/mwl-stdlib/src/path.rs:589` `join`, `:664` `normalize`, `:709`
      `relativeTo`; spec § 8. Two cases today (`path-decomposes-a-path-without-touching-the-disk.mwlt`,
      `path-join-normalize-and-relative-to.mwlt`), and it is the thinnest section left. The standing
      decision about the two legs applies: `Path::SEPARATOR` differs, so a case asserting a *built* path
      normalizes it (`Core\Str::replace($p, Core\Path::SEPARATOR, "/")`) or asserts something
      separator-free. Edges: `..` past the root, a trailing separator, an empty component, an absolute
      path joined onto a relative one, and `relativeTo` where neither is a prefix of the other.
- [ ] **`Core\Json` depth** — `crates/mwl-stdlib/src/json.rs:500` `encode`, `:673` `decode`; spec § 6.
      Three cases today, two of them about ADR 0071's derive. What is thin is `encode`/`decode` as a pair:
      the round trip as an identity, the depth and size limits, a refused value (NaN, an infinity), and
      what a malformed document's throw says.
- [ ] **`Core\Random` depth** — `crates/mwl-stdlib/src/random.rs:263` `int`, `:315` `bytes`, `:403` `pick`;
      spec § 11's first table. Three cases today. Same sweep shape as Uuid's: draws cannot be asserted, so
      count invariants over a run — every `int` inside its stated bounds and both bounds reached, an
      inverted or empty range refused, `bytes` of length 0 and of a large length, and `pick` over a
      one-element array.

## Backlog

- `Core\Json::decodeAs<T>`'s decoder — `mwl_stdlib::json` gap 2.
- ADR 0071 decodes only a scalar-fielded class: no enum/`decimal`/`Instant`/array/nested field, no
  optional key from a parameter default — `mwl_stdlib::json`'s module doc.
- `mwl-stdlib`'s member rows carry no ADR 0088 qualifier classification — `mwl_stdlib::hash`'s module doc.
- ADR 0086 § 1's substitution table, M8 — `crates/mwl-stdlib/src/cli.rs` gap 1.
- `do`/`while` is the one M4 control-flow statement that does not lower — the plan's *Open now*.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
