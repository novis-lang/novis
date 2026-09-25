# M4S — The `Core` API contract and its pure half (~5 weeks)

**Carried by goal `core-depth` — the first milestone of the parity program.** §§ 1–12 are registered
whole: the ratchet `crates/nvs-stdlib/tests/spec-members-outstanding.txt` holds no keys, which is the
condition that file's own header states for it. What the parity program keeps adding is depth — cases per
member — and `bun nv gaps` is the live worklist for that.

The library the language has been compiling calls *against* since M2 without any of it existing. Its shape
is `rule:core-api/shape-rules` and its member list is
[docs/spec/01-core-library.md](../spec/01-core-library.md), which is authoritative for every signature; this
milestone implements **§§ 1–12** of that file — `Core\Str`, `Arr`, `Math`, `Time`, `Json`, `Regex`,
`Encoding`, `Bytes`, `Path`, the three collection types, the exception types, `Random`, `Uuid`, `Hash`,
`Uri`, `Validate`, `Csv`, `Out`. Every one is pure: no capability, no reactor, no driver, no open handle, so
none of it is blocked on M5–M7. § 13's compiler-facing surfaces are pure too but each waits on something
outside `Core`; that file's own *Milestones* section says which, and is the one home for it. It is placed here rather than at M8 so that everything after it — the LSP's
completion data, M5's concurrency tests, M9's extension conformance fixtures, M11's converter mapping
table — is written against a real standard library instead of against fixtures that will need rewriting.
Part II of the spec file (anything capability-bearing) stays at M8 and merely conforms to the same
contract. `crates/nvs-stdlib` starts here — the Tier 0 crate
`rule:packaging/three-tiers` already names, and the workspace manifest already
declares; `Core\Regex` binds the engine `rule:core-classes/regex-two-tiers`
picks, and `Core\Time`'s `format`/`parse` (CLDR patterns), `Core\Time\Duration::parse` and
`Core\Str::format` land as `rule:expressions/intrinsic-literals` intrinsics with the
compile-time half wired into `nvs-types` — and, per
`rule:security/every-grammar-is-a-sink`, all three grammars are
`tainted` **sinks** alongside `Core\Regex`'s pattern. **`nvs-stdlib`'s member registry gains a per-parameter
qualifier classification here**, with an unclassified `string`/`bytes` parameter refusing `tainted` and that
crate's own test suite failing on any member that ships without one (§ 2 of the same ADR); the pass that
marks the existing rows is part of building §§ 1–12 rather than a separate slice. `Duration::parse` shares its grammar and its implementation with
M1's duration literal (`rule:types/duration-literal`), so build the literal first and this is
the same parser reached from a second entry point. `Core\Str`'s
unit is `rule:types/string-is-utf8`'s grapheme cluster, decided and seamed in
`nvs_stdlib::granularity`; the **lazily cached count** that ADR's *Consequences* names is still owed and
belongs here, in `nvs_runtime::NvsStr`'s header, alongside the O(1) boundary correction a concatenation
needs at the seam. `Core\Json` also brings the first **compiler-recognized**
attribute: `rule:core-classes/derive-attribute`'s `#[Json\Derive]`, a `nvs-types`→`nvs-ir` pass that emits
a `Json\Codec` implementation per annotated class, plus the nominal-matching rule that gates it. `#[Db\Derive]`
is the same pass over a second format and lands with M8. The **second** compiler-recognized attribute lands
here as well: `rule:routing/routes-are-compiled-not-registered`'s `#[Route]`, whose route table is built by
filtering `rule:programs/implementing`'s program enumeration and
whose three compile errors — a duplicate route, a `{param}` with no matching method parameter, an unknown
literal `url()` name — are the whole point of doing it here.
`rule:routing/the-servers-match-dispatches-nothing` adds
four more to the same pass: a `{name?}` outside the last position or bound to a parameter with no default
(§ 4), a capture or `#[Query]` parameter whose type is outside § 3's list, and a `url()` key that is neither
a capture nor a declared `#[Query]` parameter (§ 6). Its `#[Query]` and `#[Access]` are two further
compiler-recognized attributes on the same nominal-matching rule. `Core\Router::match` itself waits for M7.
The **third** rides the same pass: `rule:tooling/commands-are-compiled`'s
`#[Command]`/`#[Option]`/`#[Argument]` command table, with its own three compile errors — a duplicate
command name, two options sharing a spelling, an `#[Option]` on a parameter with no conversion from
`string`. `Core\Command::run` and the rest of `Core\Cli` wait for M8, since neither argv nor a terminal is
reachable before capabilities exist at M6.

**Verify:** every member in the spec file has a conformance test, and a mechanical check over that file
enforces the rules that can be checked mechanically — `rule:core-api/shape-rules`'s
*Verification* section is the one home for that list. PHP 8.5 is the differential oracle wherever a member
claims PHP-compatible observable behaviour (`Core\Str`, `Core\Arr`, `Core\Math`, `Core\Regex`), and each
deliberate divergence is a named fixture rather than a failing comparison. A `tainted` value cannot reach a
sink and cannot be laundered except by the members the spec marks **launder**. `Core\Arr` mutates in place
when its argument's refcount is 1 — measured, since it is the whole cost argument for
`rule:core-api/shape-rules` R3 — and allocates a copy when it is not.
`rule:core-classes/derive-attribute`'s own *Verification* section lists the derive's cases, including the
one that matters most: a decode with four bad fields throws exactly one error listing all four. The M4 CLI program
is rewritten against `Core` and gets shorter. `bun nv migration` reports full coverage of
every PHP name this milestone's classes replace, which is the point at which
[docs/spec/02-php-migration.md](../spec/02-php-migration.md)'s string, array, number and date rows stop being
a plan and become a tested claim.

**Also here: the OpenAPI emitter** (`rule:routing/api-document-is-generated-from-the-route-table`),
alongside the `#[Route]` and `#[Json\Derive]` passes it reads. `Core\Api` joins
`rule:core-classes/derive-attribute`'s closed attribute list, the four contradiction cases become
compile errors, and `nvs build --openapi` writes a deterministic 3.1 document. `nvs api diff` is the same
slice — the classification is mechanical over two emitted documents, so it costs a comparison rather than a
design.
