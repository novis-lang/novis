# Handoff

## State

Goal `lang:enums` is met. All nine features of `docs/reference/lang/55-enums.md` carry their feature
proofs: `python tools/dossier.py --verify --group lang:enums` reports nothing owed, 27 examples ok
and 9 attacks ok. `python tools/verify.py` is 14 of 14 green (conformance 2192), `--doc` resolves
every link, and `owners.py --closes lang-enums` and `playbook.py --closes lang-enums` own nothing.
Nothing is blocked.

One proof found a bug and the fix landed with it. `18446744073709551615 as Mask` did not compile for
a `uint`-backed enum, because the literal was typed `int` before the conversion was looked at.
`rule:types/numeric-literal-placement` makes `expr as T` a placing position, so
`crates/nvs-types/src/expr/literals.rs`'s new `wants_uint_placement` answers for an enum's backing
type and descends the `?T` union to the one member beside `null`.

## Next group

**Stage 2, the dossier — one file set: the placement rule and the enums chapter.** Both slices write
down what the fix above made true; `dossier.py` owes neither, so a goal switch may take them instead.

- [ ] **The placement rule names its enum row** — `rule:types/numeric-literal-placement` states the
      `as T` placing position with a `decimal` example only, and the binary now also places at a
      `uint`-backed enum's backing type and through the `?T` around it.
      `docs/rules/types/numeric-literal-placement.md:13`
- [ ] **The enums chapter says the widest case is reachable under `as E`** — § *Declaring an enum*
      names what may back an enum without saying that a literal above `int`'s half is writable under
      a conversion to a `uint`-backed one. `docs/reference/lang/55-enums.md:34`

## Backlog

- The `about.md` shape is not in the pack: `[context] shapes` prints the example, attack and bench
  skeletons and then points at `docs/examples/README.md` § *The description*, so every dossier
  session pays one peek for it. Adding that section to the shapes manifest closes it.
- `rule:types/numeric-literal-placement` was not in `[context] rules` for this goal, and a proof that
  writes a literal under `as` needs it.
