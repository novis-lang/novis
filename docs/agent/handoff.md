# Handoff

## State

**Goal `gap-owners`, stage 3 — the attribution pass — has finished `crates/nvs-stdlib`'s `db/`,
`time.rs`, `json.rs`, `cldr.rs`, `uuid.rs`, `queue.rs`, `process.rs`, `html.rs`, `lib.rs`,
`registry.rs`.** 38 items still name nobody, every one of them in `nvs-stdlib`;
`python tools/owners.py --check --reasons` is green over the tagged ones.

**`queue.rs` kept all five items and its numbering** — `queue.rs:873` and `:226` cite gaps 5 and 3 by
number. Gaps 1–2 are goal `unowned-sweep`'s own stage 0 list, gap 5 is goal `gap-zero`'s stage 5
items 4–5, and gaps 3–4 are `unowned` under one reason. Gap 1's claim was stale: the registry spells
a shape (`CoreTy::Shape`, `crates/nvs-stdlib/src/db/registry.rs:170`), and what it cannot spell is a
shape *inside* the trailing bag. Item 5's two-dialect leg moved out of the block into the module
doc, as the settled fact it is.

**`html.rs` had no gap where the register said it did.** The computed `$reason` *is* refused —
`crates/nvs-types/src/reasons.rs:116` emits `E0805`, pinned by `crates/nvs-types/tests/tainted.rs:304`
— so that half became its own section, goal `gap-zero`'s register row is struck with that evidence,
and its stage 6 group 2 is `cldr.rs` alone. What the module still waits on is the automatic `Markup`
lift, which [m7.md](../plan/m7.md) owns: `— owner: M7`.

**`lib.rs` went four items to two.** Gap 3 was a changelog of what stopped being a gap, so it is now
§ *Every shape a §§ 1–12 signature writes can be stated*, and gap 2 (a type variable is inferred)
closes it as the decision it always was. `registry.rs`'s `# Known gap` deferred to that item and had
nothing left to defer to; `str.rs:2298` cited it for a shape that is declarable now.

**`python tools/verify.py`: green.**

## Next group

**Stage 3: the attribution pass, module by module** — one file set: `crates/nvs-stdlib`'s module
docs, which hold all 38 items that still name nobody. Owner kinds are the goal's
§ *Standing decisions*; `--untagged` is the worklist and `--check --untagged-is-an-error --reasons`
is the gate. Two rules of thumb this session paid for: a `carried-gaps.md` row or a live goal's own
stage list is *evidence* and outranks a guess, and a retired goal (no `.toml` in
`docs/agent/goals/`) is not an owner — `formats`, `xml-tree` and `test-request` are retired, so what
they were carrying is `unowned` or its milestone's.

- [ ] **Tag `crates/nvs-stdlib/src/compress.rs:38`'s two items and
      `crates/nvs-stdlib/src/zip.rs:80`'s two** — at `crates/nvs-stdlib/src/compress.rs:38`, `:44`,
      `crates/nvs-stdlib/src/zip.rs:80` and `:85`. `rule:core-classes/decompression-bound` is the
      rule both implement; `carried-gaps.md` § *Owned* files spec § 17 under goal `formats`, which
      is **retired**, so decide the kind rather than copying the row.
- [ ] **Tag `crates/nvs-stdlib/src/mime.rs:40`'s three items** — at
      `crates/nvs-stdlib/src/mime.rs:40`, `:46` and `:50`. No rule owns `Core\Mime`
      (`brief.py --where mime` finds none), so spec § 17's row and the module's own doc are the
      specification, and all three are about a signature a format does or does not have.
- [ ] **Tag `crates/nvs-stdlib/src/xml.rs:122`'s two items** — at `crates/nvs-stdlib/src/xml.rs:122`
      and `:127`. `rule:core-classes/xml-tree-and-stream` is the rule;
      `carried-gaps.md` § *Owned* files `Core\Xml`'s tree under goal `xml-tree`, **retired**, and
      the two items here are about what a *parsed* name and inter-element whitespace are, which the
      rule may already answer.

## Backlog

- `ast.rs` (3), `reflect.rs` (3), `debug.rs` (3), `regex.rs` (3) — the reflection-shaped modules, one
  file set, `docs/agent/loop-goal.md` § *Standing decisions* for the kinds.
- `cli.rs`, `command.rs`, `out.rs`, `test.rs`, `response.rs`, `storage.rs`, `csv.rs`, `path.rs`,
  `decimal.rs`, `math.rs`, `random.rs` — the remainder of the 38.
- A `carried-gaps.md` § *Owned* row whose owner is retired is a finding nobody has unpicked:
  `test-request`, `formats`, `xml-tree`, `net-os-signal` all have rows (`docs/agent/playbook.md`'s
  bullet on retired slugs).
- `lib.rs`'s remaining items are numbered 1 and 4, and `carried-gaps.md:64` plus goal
  `unowned-sweep`'s stage 0 both cite gap 4 by number — renumbering costs those two.
