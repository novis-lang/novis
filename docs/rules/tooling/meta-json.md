`nvs meta --json` prints the `Core` registry as one JSON document: every class with its members and its
constants, and the enums beside them at top level — top level because the registry's roster is, an enum
having no owner class there. Each member's reference card (`rule:core-api/reference-card`) sits under a
`doc` key — `short`, `params` with each parameter's `name`, `desc` and shape keys, `return`, `errors` —
and each member also carries its signature half: `kind`, `signature` in the spec's own spelling, `params`
with types, qualifiers and defaults, `options`, `returns`; a class its `typeParams` and `constructor`, a
constant its `type` and `value`. Five rosters declared outside the registry sit beside `classes` and
`enums`: `exceptions`, `interfaces`, `attributes`, `directives`, and `capabilities`, one row per member
of the capability table naming its `class`, its `member` and, when it needs one, its `capability`.

The omission rule is the same at every level: a row with nothing written has no `doc` key, a written card
carries only its non-empty fields, and no array is ever emitted empty. That is
`rule:core-api/field-wise-precedence` made mechanical — an absent key is the one spelling of "not written"
a consumer can tell from "written, and empty" without learning a convention.

The command owns the contract, and `--json` is required so that a `meta` with nothing named cannot
succeed by printing nothing. A consumer ignores fields it does not know, so a field may be added but never
renamed or moved; a consumer on a toolchain without the subcommand treats it as "no registry docs yet",
never as an error. `docs/novis.md` and the website's core data are both built from this command alone.
