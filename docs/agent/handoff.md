# Handoff

## State

**Stage 4's two counts are the frontier — conformance 453 of 600, differential 90 of 150** — and the gap
is behavioural depth per member, not coverage: every registered member already has a case, and both of
Stage 4's named guards pass. Verify is green (**1596** cargo tests, 74 suites, clippy and fmt clean) and
`mwl test tests/conformance` is **453 passed, 0 failed** — run it as well as `verify.py`, which executes
no `.mwlt` case at all (playbook, twice).

**`Core\Path` and `Core\Json` are each done to depth at 4 and 5 cases**, joining `hash`, `csv`, `validate`,
`out`, `heap` and `uuid`. Path is pinned by the half of its grammar the POSIX leg has no filesystem for —
a drive is `C:` *and a separator*, so `C:`, `C:log` and `a:b` are components and not roots; `split` leads
with the whole root, drive included; a drive root has no name and walking past it keeps the drive;
`relativeTo` compares byte for byte except a drive letter, and answers `null` across two drives and across
a drive and none — and by three properties over a sweep: normalize's answer is a normal form, `join($p)`
and `dirname($p, {levels: 0})` are the same re-render, an empty segment contributes nothing, no segment
can move the base's root, and `join(dirname, basename)` recomposes. Json is pinned by decode-then-encode
being the identity over twelve canonical documents (with the four documents that do not decode swept
beside them so `isValid`'s agreement with `decode` is the assertion), by the four places a re-encoding
deliberately differs — `{}` and `[]` are one value, a number is re-spelled to what it is, a repeated key
is the last, an escape is decoded — and by `maxDepth` on both sides of every bound it has.

**Three case shapes are now established** and named in the plan's *Open now*: a section's edges,
invariance over a sweep, and a bound asserted on both sides. Reuse them rather than inventing a fourth.

## Next group

Three sections that have not had a depth pass, each its own domain module plus its
`tests/conformance/core/<name>-*.mwlt`. The file set they share is
`crates/mwl-stdlib/src/{random,bytes,objmap,objset}.rs` and `tests/conformance/core/`.

- [ ] **`Core\Random` depth** — `crates/mwl-stdlib/src/random.rs:263` `int`, `:315` `bytes`, `:403` `pick`,
      `:460` `shuffle`; spec § 5. Three cases today and the thinnest section left. A draw cannot be
      asserted, so this is `Core\Uuid`'s sweep shape: count invariants over many draws — every `int($lo,
      $hi)` inside its stated closed bounds, both endpoints reached over enough draws, `$lo == $hi`
      answering that value, `$lo > $hi` refused, `bytes($n)` always `$n` long and not the same buffer
      twice, `pick` only ever answering an element of its argument and refusing an empty array, `shuffle`
      a permutation (same count, same multiset) rather than a new draw.
- [ ] **`Core\Bytes` depth** — `crates/mwl-stdlib/src/bytes.rs:484` `slice`, `:1049` `pack`, `:1275`
      `unpack`; spec § 2. Five cases. The boundaries: a slice at and past the buffer's end, a negative
      offset, `pack`/`unpack` at each width's own limits and the one refusal each, and ADR 0009 § 3's
      checked `bytes as string` on a buffer that is not UTF-8 (`Core\Encoding::fromHex` is how a case
      writes one — playbook).
- [ ] **§ 9's two collections depth** — `crates/mwl-stdlib/src/objmap.rs:212` `set`, `:256` `get`,
      `:293` `remove`, `crates/mwl-stdlib/src/objset.rs:237` `add`, `:262` `has`; spec § 9. Five `object-`
      cases. The edges are identity ones under ADR 0090 § 4: two equal-content objects are two keys, the
      same object re-`set` replaces rather than appends, `remove` of an absent key, and iteration order
      being insertion order through a remove.

## Backlog

- `Core\Json::decodeAs<T>`'s decoder and ADR 0071's non-scalar fields — `mwl_stdlib::json` gap 2.
- ADR 0088's qualifier classification on every `mwl-stdlib` member row — `mwl_stdlib::hash` module doc.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir` gap 1's remainder.
- The differential corpus is 90 of 150 — `docs/plan/M4S.md`'s Stage 4 paragraph.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- ADR 0090 § 3's string/array/object equality helpers are still owed — that ADR's own body.
