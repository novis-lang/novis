The hole is `$_`. It wears a `$` because it is a binding, and in this language a binding wears a `$`.
The parser turns `$_` into its own node before identifier casing sees a variable, so the rule that
rejects an all-underscore identifier does not fire on it.

**`$_` appears exactly once on a right side** — not at least once. Three refusals keep that
enforceable, and `crates/nvs-diagnostics/src/lib.rs` is the registry that allocates their numbers:

| code | when | what it says |
|---|---|---|
| `E0129` | a right side of `\|>` contains no `$_` | names the shape (`Str::trim($_)`) and, when the right side is a method reference or a callable, adds that this `\|>` substitutes a hole rather than applying a callable |
| `E0130` | `$_` appears more than once on one right side | names binding the value to a local instead |
| `E0131` | `$_` appears anywhere outside the right side of a `\|>` | says the hole has no meaning there |

Exactly one hole is what makes substitution total: no temporary, no double evaluation, and an emitted
tree identical to the nested spelling's.
