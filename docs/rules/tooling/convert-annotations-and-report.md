One greppable spelling each, stable across releases. A runnable-mode D site carries a `TODO`
naming the rule and the difference:

```
// TODO(convert:S0140): Core\Str::length counts grapheme clusters; strlen counted bytes.
var $n = Core\Str::length($blob);
```

A commented-out site carries the rule id, what the original meant, and the idiomatic shape:

```
// convert:C0004 — PHP compares an int against a string here, and 8.0 changed what that means.
// Idiomatic Novis: convert once at the boundary, then compare — `$id == ($raw as int)`.
// if ($id == "1") { … }
```

`nvs convert --check` writes no files and emits a report — TOML, for the reason
`rule:config/the-file-is-nvs-toml-and-it-is-toml` gives — ordered by path then rule id so it diffs
cleanly. It carries per-tier counts, per-rule counts, the share of input constructs emitted as code
in each mode, and the rules that fired most often without an E branch, which is the work queue for
the table itself.

That report is the number `rule:programs/no-compatibility-promise` obliges the project to publish
instead of a compatibility claim. `--explain <id>` prints one rule: its branches, their tiers, their
conditions and their proofs.
