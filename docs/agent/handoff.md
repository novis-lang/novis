# Handoff

## State

Goal `lang:types`: every feature in `docs/reference/lang/20-types.md` owes the five artefacts of
`rule:testing/four-proofs`, and the chapter is the whole file set — 18 features, one section each.

Six are complete: `array-t`, `callable-classes-object-shapes`, `every-binding-has-a-type`,
`numbers-bool-int-uint-float-decimal`, `text-string-and-bytes` and `mixed`. Twelve still owe;
`python tools/dossier.py --owed --group lang:types` is the list. Nothing is blocked.

`mixed`'s attack found a compiler bug and it is fixed in the same group, not recorded: any
`$a[$k]` whose key was a `mixed` panicked `nvs-ir`'s lowering, because
`nvs_types`' `check_array_key_type` deliberately lets a `mixed` key through for the tag to answer
and nothing answered it. `Helper::ValueToArrayKey` is that answer now — a `string` copied, an
`int`/`uint` normalized to its decimal, every other tag a catchable throw — and it serves the
subscript, the write, `unset` and an array literal's explicit key alike, because all four go
through `Lowering::lower_array_key`.

`target/release/nvs.exe` was rebuilt at the end of this session, because `dossier.py --run` prefers
it and the stale one still panicked on the `mixed` attack.

A language feature's two cases are usually `covers:` markers added to cases the corpus already
holds; only a claim nothing pins earns a new file. `text-string-and-bytes` took one of each, and
`mixed` took two markers plus the case pinning the fix.

`docs/agent/loop-goal.toml`'s `[context] rules` and its copy at
`docs/agent/goals/dossier/80-lang-types.toml` now carry the next group's chapter rules; keep
swapping them per group rather than naming the chapter's whole `types/` set, which is 47 fragments.

## Next group

**Stage 2: the dossier** — one file set: `docs/reference/lang/20-types.md` and the four proof trees
under `docs/examples/lang/types/`, `tests/hostile/lang/types/`, `benches/members/lang/types/` and
`tests/conformance/`. One slice is one feature with all five artefacts.

- [ ] **`lang:types/nullable-union-literal-and-enum-case-types`** — owes all five.
      `rule:expressions/nullable-conversion`, `rule:types/unions-and-mixed`,
      `rule:types/literal-types` and `rule:types/enum-case-type` specify it.
      `docs/reference/lang/20-types.md:265`
- [ ] **`lang:types/void-never-self-static`** — owes all five.
      `rule:statements/static-is-a-member-modifier` specifies the `static` half; the section itself
      is what the other three rest on. `docs/reference/lang/20-types.md:322`
- [ ] **`lang:types/type-aliases`** — owes all five. `rule:types/type-alias`,
      `rule:types/alias-is-never-a-bare-class` and `rule:types/class-scoped-alias` specify it.
      `docs/reference/lang/20-types.md:370`

## Backlog

- Nine more `lang:types` features after the group above — `python tools/dossier.py --owed --group
  lang:types` is the live list.
- `Core\Encoding::isValidText` takes a `Charset` second argument, so the language-level "is this
  text" question is `as ?string` — `docs/reference/lang/20-types.md` § *Text* says nothing about
  either, and could.
