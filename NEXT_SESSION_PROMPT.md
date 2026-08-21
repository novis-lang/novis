# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is in progress — see git history for
detail on how earlier items landed; it's not repeated here per CLAUDE.md's "state a fact once"
rule.

**Last session finished the ADR 0029/0030 identifier-casing check**, the smaller of the two
threads the prior session's prompt left open. It lives in `crates/mwl-syntax` (new module
[`crates/mwl-syntax/src/casing.rs`](crates/mwl-syntax/src/casing.rs)), not `mwl-hir` — the ADR
itself calls out that this check needs no name resolution, so it runs directly off the AST
`parse_file` already produces. Concretely: `check_casing(stmts, src, diags)` walks every
declaration and reports one diagnostic per violation, each carrying a mechanically-derived
rename suggestion (split on the identifier's own case/underscore boundaries, re-joined in the
target convention) attached as a `with_fix` machine-applicable edit:

- Class/interface/trait/enum/enum-case/namespace-segment names: `PascalCase` (`E0110`).
- Method names: `camelCase` (`E0111`), except a method literally named `__construct`, which gets
  its own targeted `E0114` naming `constructor` as the fix rather than the generic diagnostic.
- Property/parameter/local-variable names, and a closure's optional self-name
  (`FnExpr::name`): `camelCase` with **no** leading-underscore allowance at all (`E0112`) — this
  is ADR 0030's tightening of ADR 0029's original one-underscore carve-out, so there was never a
  separate "allowed" case to implement, only the already-merged zero-exception rule.
- Class constant names: `SCREAMING_SNAKE_CASE` (`E0113`).

**One correction already landed within this same session, worth knowing about so it isn't
re-litigated:** the first cut of `is_pascal_case`/`is_camel_case` added an extra check beyond ADR
0029's own regex table — rejecting any run of two-or-more consecutive uppercase letters, to
enforce § 1's "acronyms are one word, never kept all-caps" rule (which the bare table doesn't
express on its own: `HTTPClient` matches `^[A-Z][A-Za-z0-9]*$` exactly as well as `HttpClient`
does). The user asked for that relaxed back out — `HTTPClient`/`parseXMLPayload`-style spellings
should compile unchanged — so [ADR 0032](docs/adr/0032-acronym-casing-rule-revoked.md) now revokes
ADR 0029 § 1 outright, and `is_pascal_case`/`is_camel_case` check only the leading character's case
plus an alphanumeric rest, nothing more. Don't reintroduce the consecutive-uppercase check.

Only a *declaration* site is checked, never a reference (`Class::method`, `$obj->prop`, a
`use Trait;` name, an `extends`/`implements` target) — each of those names something declared
elsewhere that was, or will be, checked once at that other site. Two deliberate gaps, both
documented in `casing.rs`'s module docs: a `type` alias's own name is not checked at all (ADR
0029's scope table doesn't list that category, so no rule is enforced rather than guessing one),
and anything that already gets its own "this construct is rejected" diagnostic elsewhere
(`TopLevelFunction`/`TopLevelConst`, a function-scope `static` local, a non-`case` member inside
an `enum` body) is left uninspected here too, same as those constructs' own AST doc comments
already argue. 36 unit tests in `casing.rs` cover one correctly-cased and one mis-cased fixture
per category, that an all-caps acronym is accepted (both in a `PascalCase` and a `camelCase` name,
per ADR 0032), the leading-underscore rejection, the `__construct`/`constructor` pair, and that
nested declarations (a class inside a function body, an anonymous class's members, a property
hook's parameter) are still walked. Full workspace `cargo test`,
`cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check` are all green — read
`CLAUDE.md` first, then run `sh .claude/brief.sh` for the live status slice.

**This closes out the "smaller, self-contained" thread entirely.** There is now exactly one
open thread left before M2 can be called done: **`mwl-types`, the type checker** — the crate
doesn't exist yet. Everything the previous prompt said about scoping it still applies verbatim
(not repeated here per "state a fact once"): it builds on `mwl-hir`'s now-feature-complete tables
(`SymbolTable`, `ClassGraph`, `MemberTable`, `AliasTable`), and needs, from earlier *Accepted* ADRs
not yet implemented anywhere:

- ADR 0007's full type table: declared-type recording/enforcement, definite-assignment checking,
  flow-sensitive union narrowing, array element-type checks at every write and nesting depth, the
  arithmetic result-type table (including refusing `int + uint`), interned type descriptors.
  **No inference engine, no `Unknown` type** — every binding's type is declared, so this is a
  checker, not a solver.
- ADR 0022's definite-property-initialization rule, extended into constructor bodies.
- ADR 0013's `Comparable` requirement for `<`/`>`/`<=`/`>=`/`<=>` on two objects.
- ADR 0024 §§ 2-3's `tainted` qualifier propagation and laundering, plus the sink refusal.
- ADR 0027's `callable`-only rule (no string/array callable spellings, no `__invoke`).
- ADR 0028's two checker-side rules: `Stringable` at implicit string-conversion sites; `unset()`
  on a declared object property refused outright.
- ADR 0031's `callable`-not-`Closure` rename (a pure rename, no behavior change).
- `AliasTable`'s first real consumer: substituting a `type` alias wherever a declared type
  (property, parameter, return type) is looked at — and, per this session's own casing-check gap
  above, `mwl-types` (or a follow-up to this session's work) is also the natural place to decide
  whether a `type` alias's own name should ever get a casing rule, if that question comes up again.

This is a large milestone slice — worth scoping into its own sub-steps (start with the type table
and definite-assignment, since everything else in the list depends on having a type to check
against) rather than attempting all of it in one session.

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone
needs before it can be called done. ADR 0029/0030/0032's corpus entries are now satisfied by
`casing.rs`'s own unit tests (this session's work) — that's the one corpus item that didn't need
to wait on `mwl-types`. Everything else in that Verify line (ADR 0007, 0013, 0022, 0024, 0027,
0028 corpus files, plus IR snapshot tests) still depends on `mwl-types`/`mwl-ir`, neither of which
has started.
