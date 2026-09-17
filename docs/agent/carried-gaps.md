# Carried gaps — what a shipped feature still owes, and who owns it now

[carried-refusals.md](carried-refusals.md) does this for one kind of hole: a `nvs-ir` refusal site,
which `python tools/holes.py` can find on its own because the site is in the source. **This file is
for the other kind** — a gap that is real, written down in the module doc that owns it, and invisible
to every tool, because nothing in the tree is shaped wrong. A `Core\Log` record with two keys where
`rule:observability/a-log-record-carries-trace-ids-when-a-trace-is-active` names six compiles, tests green and ships.

**It exists because the handoff cannot hold one.** `docs/agent/handoff.md` is *state*: `tools/loop.py`
overwrites it with the next goal's seed at every switch, and `tools/goal-switch.py` carries the
outgoing goal's `[[check]]` blocks forward and nothing else. So a `## Backlog` bullet lives exactly
until the goal that wrote it goes green — which is how `rule:packaging/an-artifact-is-one-immutable-content-addressed-file`'s cache redesign came to be "in the
handoff's backlog" according to a module doc, and in no file at all according to the repository, and
how `§18 stream` came to be filed under "goal `database`'s" six goals after goal `database` closed. Same failure,
same fix, one file up.

## The contract

- **An entry names an owner.** A goal slug naming a live goal in
  [the goals directory](goals/), a **milestone tag** whose plan already covers the gap, or the
  word **`unowned`** with the reason it is nobody's yet. `unowned` is a legitimate state — it is a
  scheduling question for the user — but it is never the *absence* of an answer, and it is never what a
  *future* milestone's scheduled work is called. Goal `gap-owners` turns these three kinds into a gate.
- **The ratchet files carry the same column.** `crates/nvs-stdlib/tests/`'s four `*-outstanding.txt`
  lists write `# <owner>` after every key, and `every_outstanding_key_names_an_owner` in
  `spec_registry_coverage.rs` fails on one no goal answers for. Two kinds rather than three
  there: a key is struck by a session and only a chain entry runs sessions, so a milestone nobody has
  cut into goals reads as `unowned` on a key.
- **An entry leaves exactly one way: the gap is closed.** Not when it is rewritten, not when it stops
  being convenient. An entry whose owner went green without closing it is the failure this file
  exists to make visible; strike the owner, not the entry. The *Owner* column is the row's expiry
  declaration: `python tools/playbook.py --check` flags a row whose owner is retired in
  [the goals directory](goals/) and not yet struck. A § *Unowned* bullet has no owner to
  watch, so it ends with the `[until: ...]` trailer [tools/playbook.py](../../tools/playbook.py)'s
  module doc defines, naming the state of the tree that closes it.
- **One line of *what*, and a pointer to the module doc that owns the detail.** Every fact in this
  repository has one home, and for a gap that home is the module. This file is an index of who, not a
  second copy of what.
- **A session that finds a gap off its path writes it here**, not in the handoff, and moves on —
  [loop-authoring.md](loop-authoring.md) § 8's rule with a durable destination.

## Owned

Each of these is claimed by an entry on the chain and will be struck when that entry goes green.

| Gap | Owner | Where the detail lives |
|---|---|---|
| `Core\Db::stream` on four drivers, `streamAs` whole, § 18's `serverVersion` | `gap-zero` | `crates/nvs-stdlib/src/db/mod.rs` gap 3 |
| `rule:core-classes/html-to-source`'s computed `$reason` is not refused — was blocked on a full diagnostic band | `M7` | `crates/nvs-stdlib/src/html.rs` § *Known gaps* |
| `queryAs<T>`'s three refusals are at run time; the band they waited on is open | `gap-zero` | `crates/nvs-stdlib/src/db/mod.rs` gap 4 |
| `Core\Queue`'s `limits` and `grants` stay undeclared until an isolate enforces them | `gap-zero` | `crates/nvs-stdlib/src/queue.rs` gap 1 |
| Nothing arms `DebugFlags::TRACE` and nothing reads the `debug.trace` grant, so a query span renders on sampling alone | `M10` | `crates/nvs-db/src/span.rs` gap 1 |
| A MySQL unique or primary key over a bounded column wider than InnoDB's key budget is emitted whole and refused by the server, and no grade says so | `gap-zero` | `crates/nvs-db/src/ddl.rs` § *An engine's own limit is the dialect's rule* |

## Unowned

Nobody's, and each is a scheduling question rather than a session's. **One bullet each, and `python
tools/owners.py --unowned` is the roster derived from the modules themselves.** They arrive
three ways: an owner that went green without closing its gap and was struck rather than renamed, a
rule answered in full by code that no configuration key reaches, and a decision nobody has taken,
where taking it is the work and the code that follows it is not.

- **Spec § 13 gives `Core\Test` a double half that no class declares.** `double<T>`, `partial<T>`,
  `assertCalled` and `assertNeverCalled` are specified in full by `rule:testing/doubles` and
  `rule:testing/interaction-after-the-fact` — a double is a shape of closures checked *structurally*
  against an interface, so it is compiler work rather than a `Core` body — and `assertCompletes` has
  the spec row and nothing else. No goal on the chain builds any of them and no milestone's plan
  carries them. What has to be decided is whether the double half becomes a goal of its own or waits
  for a milestone that states the scope, which is the same question either way: the structural check
  is what prices it. `crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt`.
  [until: gone crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt:Test::double]
- **`Core\BigInt` is a spec § 13 row with no class behind it.** Arbitrary-precision integers sit in
  the same row as `Core\Decimal`, which is registered whole, and `docs/plan/m8.md:52` scoped the pair
  together — M8 closed with the decimal half alone, so the tag that carried it is behind the program
  and a deferral would name a milestone that has already gone green. What has to be decided is
  whether the type is wanted before a program asks for it, since nothing in the corpus does and the
  `decimal` scalar covers the money case it would otherwise be reached for.
  `crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt`.
  [until: gone crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt:Core\BigInt]
- **An integer where a grant expects a bool, a path or a list validates clean and grants nothing.**
  `[capabilities.fs] read = 1` passes `nvs config check` at `0 warnings` and is denied at run time,
  because `crates/nvs-config/src/tree.rs:50`'s `Setting` is `#[serde(untagged)]` and every directive
  shares that one enum. Narrowing it is not the fix — `:39-43` records that `memory = false` and
  `exporter = false` are load-bearing second spellings — so the arms a directive accepts are
  per-directive and nothing declares them. What has to be decided is where that table lives, beside
  each field in the tree or beside the capability roster the checker already walks; `docs/plan/m6.md:52`
  promises the verb and not the refusal, so no milestone's plan carries it either.
  `crates/nvs-config/src/tree.rs:50`. [until: reviewed 2026-09-13]
- **The route table is walked by comparison rather than by a trie**, which answers identically and
  costs one comparison per row of the right verb where a trie costs one step per path segment.
  `rule:routing/path-grammar` names `matchit`'s left-to-right precedence as the model and no
  structure, and no milestone claims routing-table performance — M12 is the JIT tier, and M7 scopes
  `Core\Router::match`'s surface rather than what walks under it. What has to be decided is whether
  the curve is ever the request path's problem at all: a table of tens of rows may never measure, so
  the decision is a measurement on `benches/serve-proxied.json`'s arms and the code that follows it
  is one function. `crates/nvs-runtime/src/routes.rs` gap 1. [until: reviewed 2026-09-10]
- **A division whose intermediate exceeds 128 bits throws where the quotient would have fit**, in the
  one corner where a wide mantissa and a wide scale difference meet: the fold of the two operands'
  scales is a `checked_mul` over `u128` taken before the divide. ADR 0054 § *Consequences* already
  predicts the wider intermediate that closes it. What has to be decided is whether every division
  pays a 192-bit fold to remove a corner refusal, or that refusal stands as a stated bound of the
  type. `crates/nvs-runtime/src/decimal.rs` gap 1. [until: reviewed 2026-09-10]
- **An array header carries no interned element-type descriptor**, which `rule:types/arrays` gives it
  so a value arriving through `mixed`, `json_decode` or an isolate boundary can be checked. Nothing
  builds an array by those routes yet, and both readers that would want the word are specified to
  walk without it — `as array<T>` does, and `rule:types/type-test` makes `is array<int>` the same
  O(n) walk. What has to be decided is whether the word is carried before a reader needs it, given
  that adding it later is a widening of `ArrayHeader` rather than a redesign.
  `crates/nvs-runtime/src/array.rs`. [until: reviewed 2026-09-10]
- **A command argument typed as a *subset* of an enum's cases is refused when the command is run,
  not when it is written.** `Log\Level` converts; the `Log\Level::Warn|Log\Level::Error` that § 3 also
  admits is `ArgConv::Unconverted`. Both facts a conversion needs are already answered on
  `ArgConv::Enum`, so the code is a filter rather than a design; what has to be decided is which end
  closes it — reading the union's members where § 6 owns the spelling, or refusing the declaration
  outright so the error arrives where it was written. `crates/nvs-runtime/src/commands.rs` gap 1.
  [until: reviewed 2026-09-10]
- **A local declaration cannot be typed with a bare inline shape type**, although every other
  declaration slot can. Statement-initial `{` commits to a block, and telling a type-prefix apart from
  one needs lookahead past a matched, possibly-nested `{...}` to the `$name` behind it. What has to be
  decided is whether the grammar buys that lookahead or whether `rule:types/shape-type`'s own example
  — `type Point = {x: int}; Point $point;` — is the answer for a local: a surface question, not a
  parser one, and the parser cost is the reason it is worth asking.
  `crates/nvs-syntax/src/lib.rs` § *Known gaps*. [until: reviewed 2026-09-10]
- **The compiled-unit cache loads on no `aarch64` host**, one of which
  [design.md](../plan/design.md)'s platform row (line 94) names as supported for macOS. Making freshly
  written bytes executable there needs instruction-cache maintenance `mprotect` does not imply, and
  what has to be decided is where that lives — this loader, or whatever maps the page in
  `nvs-codegen` — rather than whether it is wanted; Mach-O's leading underscore rides along with it,
  since neither is worth a format branch on its own. Until then every run on such a host compiles:
  slow, never wrong. `crates/nvs-cli/src/cache.rs` gaps 2–3. [until: reviewed 2026-09-10]
- **`rule:routing/a-shared-name-is-one-endpoint-everywhere` is shipped and unguarded.** Its three
  observations — `Core\Router::url` answers the one path, `Match::name` answers for every verb, the
  API document suffixes the operation with the verb — have no fixture: no `.nvst` under
  `tests/conformance/` declares two `#[Route]`s sharing a name, and `crates/nvs-cli/tests/openapi.rs`
  has no suffix case. The rows themselves are guarded by `crates/nvs-types/tests/routes.rs`. Found by
  the docs migration's sweep (unit C8), which could record it and not write it.
  [until: exists tests/conformance/core/a-shared-route-name-is-one-endpoint-everywhere.nvst]
- **The queue's two tables are the runtime's own, and two questions about what they carry are open.**
  `nvs_jobs` gets its dedupe guarantee from a plain unique key over `dedupe_pending`, which
  `rule:core-classes/queue-storage-is-a-table` states and the runtime's own schema carries — but that
  key exists only where an operator has run `nvs queue migrate`, and `push` proves nothing about
  whether they did, so a deployment behind on the converge is racy at `read committed` while reading
  as healthy. What has to be decided is whether a member proves the constraint is there — an
  introspection on first use, or a refusal to serve a queue whose schema is behind — or whether that
  window stays an operational bound the migration command's own text names. Beside it, `stats`
  answers the four counters the rule names and a dead-lettered job's attempts are in none of them,
  because `nvs_dead_jobs` deliberately carries nothing beyond `id` and `queue`; what has to be decided
  there is whether that table keeps the columns a fifth counter would read, through the same
  `nvs queue migrate` converge goal `queue-purge` takes its `tag` column through.
  `crates/nvs-stdlib/src/queue.rs` gaps 3 and 4.
  [until: gone crates/nvs-stdlib/src/queue.rs:deployment that never ran]
- **`Db\DbError` is outside spec § 10's error tree, so a database refusal carries no `issues`.** The
  class declares all five of § 18's values and `RuntimeError` is its parent, but `issues` is declared
  by `ParseError` alone, so `queryAs`'s per-column refusals are thrown as a `ParseError` naming the
  columns rather than as the class § 8 gives the database. What has to be decided is whether `DbError`
  joins that tree — § 10 giving it the property, which is a spec change and a registry one — or the
  split stands and a row that does not fit its class is a parse failure by design. Neither goal that
  would have taken it is live: `database` and `schema` are retired with M8's database half carried.
  `crates/nvs-stdlib/src/db/mod.rs` gap 2.
  [until: gone crates/nvs-stdlib/src/db/mod.rs:spec § 10's error tree]
- **`Core\Xml` reads a qualified name as its spelling, so no `xmlns` declaration is resolved.**
  `<x:a/>` answers `x:a`, which is right for a document a program controls and not enough for one it
  does not: two documents meaning the same thing under different prefixes compare unequal. What has
  to be decided is not whether resolving one is possible but where the resolved answer would live —
  both shapes answer one node family, the one `rule:core-classes/html-parsing` makes both parsers
  produce (`rule:core-classes/xml-tree-and-stream`), so a namespace URI on a node is a change to a
  family the HTML parser fills too and HTML has no namespaces to put there. Goal `xml-tree` is
  retired and no milestone's plan names namespaces. `crates/nvs-stdlib/src/xml.rs` gap 1.
  [until: gone crates/nvs-stdlib/src/xml.rs:prefix and all]
- **A `Core\Ast` node carries no position and no source text, so a walk classifies but cannot
  quote.** A `#[Test]` that walks the tree can fail today and cannot name the `file:line` it failed
  about, and a structural rule that cannot point is a check rather than a report
  (`docs/adr/tooling-parity.md`, the Deptrac row). What has to be decided is a qualifier question
  rather than a slot: that module's § *Decision: the tree is inert because there is nothing in it to
  run* says a node answering its own source text is the point at which `parse`'s `$source` stops
  being `Qual::Neutral`, so the text comes back out `tainted` and every consumer of it becomes a
  sink question. `crates/nvs-stdlib/src/ast.rs` gap 1.
  [until: gone crates/nvs-stdlib/src/ast.rs:carries no position and no text]
- **A `secret` value held in an `array<T>` element or in a shape literal's field is dumped in full**,
  because redaction reads a *declared* property's bit off `nvs_runtime::ClassDesc::field_is_secret`
  and neither container has a declaration to read: `rule:security/secret-qualifier` does not compose
  the qualifier onto an element type, and a `rule:types/object-top` shape field's type is inferred
  from its initializer rather than written. Both ends that exist are closed — the argument is refused
  where the call is written and a `secret` property is a `nvs_render::Node::Redacted`. What has to be
  decided is whether the qualifier composes onto a container at all, which answers once for the type
  lattice and is not a dump walk's call to make. `crates/nvs-runtime/src/record.rs` gap 1.
  [until: reviewed 2026-09-10]
- **An enum case dumps as its backing integer**, because an enum has no tag of its own at run time
  and a case arriving through `mixed` is indistinguishable from an `int`. The rendering half is not
  the hole — `nvs_render::Node::EnumCase` exists and is what a producer with a static type would
  build — so what has to be decided is whether an enum earns a representation of its own, a tag or a
  bit the roster can test. That answers for every `mixed` consumer at once rather than for this walk,
  and `rule:types/conversion` is where it lands. `crates/nvs-runtime/src/record.rs` gap 2.
  [until: reviewed 2026-09-10]

- **`Core\Cli::arguments` is empty inside a served request**, because it answers whatever the launcher
  wrote with `nvs_runtime::Ctx::set_command_line` and only `nvs-cli` writes one. Empty is not wrong —
  a request has no command line — but the member is rostered for every program rather than for a CLI
  one, and nothing this module can reach decides which. What has to be decided is what the whole of
  `Core\Cli` means off the terminal: an empty roster, a refusal, or the host's own argv, and the same
  answer settles `colorDepth` and the output sink. `crates/nvs-stdlib/src/cli.rs` gap 1.
  [until: reviewed 2026-09-10]
- **A generated help page names neither a parameter's declared type nor its default**, because § 6's
  table deliberately carries no declared type and `nvs_types::commands`' own gap 1 refuses inventing a
  second copy of the signature the handler already holds — so rendering the default alone would read
  as though the type were absent from the declaration. What has to be decided is whether a help row
  reaches that signature at render time rather than carrying a copy of it, which is a table-shape
  question in `nvs-types` and not a renderer's. `crates/nvs-stdlib/src/command.rs` gap 1.
  [until: reviewed 2026-09-10]
- **A UNC path re-renders without the doubled separator that makes it one**, because
  `\\server\share\f` parses as an ordinary absolute path whose components are `server`, `share` and
  `f`. Spec § 8's roster is whole, so this is a modelling hole and not a missing member, and the fix
  is a third root shape beside `Parts::drive` rather than a change of interface. What has to be
  decided is whether Novis models UNC at all: nothing on the path to `examples/collect.nvs` writes
  one, and a root shape no supported deployment reaches is surface bought for nothing.
  `crates/nvs-stdlib/src/path.rs` gap 1. [until: reviewed 2026-09-10]
- **A drive-relative path is one component named `C:log`**, because `split_drive` declines a drive
  with no separator after the colon, so Windows' "the current directory *on* drive C" round-trips as
  a relative path rather than as a root. The obvious fix is worse — treating `C:` as a root makes
  `Path::split('a:b')` answer `['a:', 'b']` for an ordinary relative path — so what has to be decided
  is whether a drive-relative root is distinguishable from a colon inside a segment at all, which is
  a grammar question spec § 8 does not answer. `crates/nvs-stdlib/src/path.rs` gap 2.
  [until: reviewed 2026-09-10]
- **`Core\Random\Seeded` is not registered, and the tree states two designs for it.** Spec § 11 makes
  it a separate object with the same members whose guarantee is reproducibility rather than
  unpredictability, while `docs/novis.md`'s `Core\Random` chapter says there is no seeded generator
  under any name. ADR 0079 § 12 takes the second route without retiring the first: `#[Test(seed:)]`
  seeds `Core\Random` itself for the isolate a test runs in, which is the reproducibility a test needs
  and nothing a program outside one can reach. What has to be decided is which of the two the language
  means, since a class registered under § 11 would be the thing § 12's attribute exists to make
  unnecessary. `crates/nvs-stdlib/src/random.rs` gap 1. [until: reviewed 2026-09-10]


- **`rule:classes/definite-property-initialization` is checked over the part of a constructor body the
  walk can see, and three corners sit outside it.** A property backed by a `set` hook is exempted
  rather than checked against whether the hook commits a value; `scan_expr` descends into a handful of
  composite expression forms, so a `$this->prop = ...` inside a closure body or a `match` arm is
  diagnosed spuriously rather than missed silently; and a class with no explicit `constructor` is
  never checked against the `parent::constructor(...)` obligation, since it has no body of its own for
  such a call to sit in. These are M4 checker holes and M4 is carried: that milestone's loop goal
  passes, and no live entry on the chain writes this crate's checking passes again. What has to be
  decided is
  whether this pass models a hook's body at all — `rule:classes/lateinit`'s *Revisiting* section
  defers the hooked-property question to `docs/spec/`, so it is a language decision before it is a
  checker one — and whether PHP's inherited constructor is modeled here or left to the runtime throw.
  `crates/nvs-types/src/ctor_init.rs` gaps 1–3. [until: reviewed 2026-09-10]
- **`rule:classes/lateinit`'s compile-time half stops at the declaring class.** An inherited
  `lateinit` property read through `$this` in a subclass's own method is not checked by this pass,
  which tracks a class's *own* properties only and leaves that read to the rule's runtime throw;
  `scan_expr` shares `crate::ctor_init`'s partial descent, so a read inside a closure body or a
  `match` arm is silently unchecked; and a `set`-hooked `lateinit` property is not modeled specially.
  What has to be decided is whether the pass walks the `extends` chain — `own_required_properties`
  took the opposite decision for the same reason, so taking this one alone leaves the two passes
  inconsistent with each other — and the hooked case is the same `docs/spec/` question that rule's
  *Revisiting* section defers. `crates/nvs-types/src/lateinit.rs` gaps 1–3.
  [until: reviewed 2026-09-10]
- **A named constant is a legal property default and an illegal parameter default.** An enum case
  (`Mode $m = Mode::Fast`) or another class's `const` becomes a `nvs_runtime::FieldDefault`
  materialized straight into a slot, where the case *is* the integer it folded to; at a parameter
  default `nvs_ir::lower::emit_const_arg` would have to hand a `ConstArg::Int` to a
  `nvs_ir::ty::Ty::Enum` position, so `literal_default` refuses it as
  `E_PARAM_DEFAULT_NOT_LITERAL`. What has to be decided is whether that emitter carries the
  position's own IR type rather than the constant's — which closes the parameter half by widening
  `literal_default`, and changes what every omitting call site emits — or the asymmetry stands as a
  stated bound of a parameter default. `crates/nvs-types/src/defaults.rs` gap 1.
  [until: reviewed 2026-09-10]
- **A mount's entry script may echo a page and call a response body member, and nothing refuses it.**
  `rule:http-server/a-request-resolves-in-five-steps`'s steps 4 and 5 run a `.nvs` file's top-level
  frame as the request body, which is the one row of the shared-output table this pass does not
  refuse; the same file is one compiled unit whether `nvs serve` ran it or `nvs run` did, so a static
  refusal there would refuse the CLI use of every such file. What has to be decided is whether the
  language gains a declaration that a file is an entry — the fact that is missing, and a surface
  question rather than a checker one — or the row stays answered at run time by the default this
  module's own doc names, where `echo` means `text/html` and a body member wins the `Content-Type` it
  declared last. `crates/nvs-types/src/response.rs` gap 1. [until: reviewed 2026-09-10]
- **A `Core` target on `extends`/`implements` is trusted to exist.** `QName::is_core` is a spelling
  test, and this crate cannot hold a name to `nvs_stdlib::registry`'s roster the way
  `nvs_types::expr::calls` holds a `new` target to it: `crates/nvs-hir/Cargo.toml` depends on
  `nvs-diagnostics` and `nvs-syntax` and nothing else, which is the graph position that lets a class
  link resolve before the stdlib exists at all. What has to be decided is which end closes it — a
  roster this crate is handed at construction, or the `Core` half of a link checked in `nvs-types`,
  which already reaches the registry — because the second answer makes one link error arrive from a
  different pass than every other. `crates/nvs-hir/src/hierarchy.rs` gap 1.
  [until: reviewed 2026-09-10]
- **A `const` in a `require` path is not folded, with two corners around it.** A class constant is
  the only constant Novis has, so reading one means resolving a class out of the very table this walk
  is building; the double-quoted cooker recognises a practical escape subset and leaves
  octal/hex/unicode un-cooked; the name harvest is an over-approximation whose miss costs a class
  that fails to autoload. What has to be decided for each is where it lives rather than what the code
  is: a constant folder that runs before the graph is walked, a string-literal cooker something
  besides this module needs, and a harvest derived from the AST rather than hand-written so a new
  node cannot be missed. `crates/nvs-hir/src/requires.rs`'s own `# Known gaps` is the live list.
  [until: reviewed 2026-09-10]
- **A `type` alias's own name is held to no casing rule.** `rule:core-api/identifier-casing`'s scope
  table lists the categories the checker enforces and "type alias" is not one of them, so the
  declaration walk passes over the one name it could check and does not guess a rule. What has to be
  decided is whether that table gains the row — a rule change rather than a check, and the reason
  this sits here rather than in the pass: the pass is four lines once the rule says which casing an
  alias takes. `crates/nvs-syntax/src/casing.rs` gap 1. [until: reviewed 2026-09-10]
- **`compiler_version_hash` is the release version, so two builds of the same version share an
  artifact key.** A developer who rebuilds the compiler without bumping `CARGO_PKG_VERSION` keeps
  every artifact keyed against the old one; `debug_assertions` separates a debug build from a release
  build and nothing separates two debug builds. What has to be decided is whether the workspace takes
  a `build.rs` for a stamp that moves with the source — this crate has none, and a stamp is a
  workspace-wide build dependency rather than a line in this key — or a developer's stale artifact
  stays their own problem. `crates/nvs-config/src/cache.rs` gap 1. [until: reviewed 2026-09-10]
- **A bundled program resolves a `require` and not an autoload root.** `rule:programs/no-runtime-autoload`'s
  probing lists real directories and is not routed through the embedded payload, so a name reached
  only through an autoload root does not resolve inside a bundle, while
  `rule:packaging/a-bundled-require-resolves-at-build-time` makes `require` closed-world and does.
  What has to be decided is whether a bundle's build resolves its autoload roots into the payload —
  which makes the root set part of what `nvs build --compile` freezes — or an autoload root is
  declared unsupported in a bundle and refused where it is written.
  `crates/nvs-diagnostics/src/embedded.rs` gap 1. [until: reviewed 2026-09-10]
- **The door decides which requests a CSRF check covers and refuses none of them, and the route label
  it derives has no caller.** Verification is `rule:security/protocol-roster`'s constant-time
  comparison against a key bound to the issuing session, and this crate has neither half in reach:
  `Core\Csrf::verify` is `nvs-stdlib`'s, deliberately not a dependency, and no `[http]` directive
  names a key. The label is what `rule:observability/route-label-is-the-declared-name` reaches, and
  no core owns a metrics registry because nothing exports one yet. What has to be decided is which
  side verifies a token — the door with a configured key, or a handler calling `Core\Csrf` — and
  which crate owns the registry a label lands in. `crates/nvs-server/src/route.rs` gaps 1 and 2.
  [until: reviewed 2026-09-10]
- **A write to a field of `Core\Task::all`'s result is checked against the tags of the literal that
  started it.** The result is built with the argument's own shape class, because
  `rule:types/object-top`'s shape class is named for its field names alone and those are identical on
  both sides, but that descriptor's per-slot tags come from the literal, where every field is a
  closure — so `$page->count = 5` is refused with a message naming `object`, while reading is
  unaffected. What has to be decided is whether `nvs-ir` records the *result* shape's representations
  at the call site, which degrades the shared class's tags to unchecked exactly as two disagreeing
  literals of the same shape already do, or the two shapes stop sharing a class at all.
  `crates/nvs-stdlib/src/task.rs` gap 1. [until: reviewed 2026-09-10]
- **The queue's schema has no spelling a program can reach, so a conformance case over a converged
  queue writes the DDL out itself.** `nvs_stdlib::queue::schema()` is the one home and `nvs queue
  migrate` is the only thing that applies it, and a `.nvst` case reaches no operator subcommand — so
  the three `tests/conformance/core/queue-*-on-sqlite*.nvst` cases each carry the text that command
  prints under `--dry-run`, and a schema change that a statement reads breaks them rather than
  drifting past them. What has to be decided is whether the value gets a `Core` spelling — which a
  deploy script would use as much as a case would, and which is a surface addition nothing has asked
  the user for yet. `crates/nvs-stdlib/src/queue.rs`'s `schema`.
  [until: gone tests/conformance/core/queue-cancel-releases-a-dedupe-key-on-sqlite.nvst:create table nvs_jobs]
- **Emmet and HTML validation do not reach a template region**, where completion, hover, the colour
  picker and tag renaming do. Forwarding runs a *provider* over a virtual document, and neither of
  those two is one: Emmet expands from the language of the document the cursor is in, and validation
  is published by the HTML service for the documents it owns.
  `rule:ide/a-template-region-gets-the-editors-services-and-formatter` names both, so what has to be
  decided is whether `emmet.includeLanguages` mapping `nvs` to `html` — which turns abbreviation
  expansion on in the Novis half of the file too — is the trade, or whether the client grows a
  second, real document the service can own. `editors/vscode/src/regions.ts` § *What forwarding does
  not reach*. [until: reviewed 2026-09-11]
## What is *not* on either list

Two kinds of thing look like an unowned gap and are not, and saying so here is cheaper than each
session deciding again:

- **A gap a future milestone's plan already covers is scheduled work, not a hole.**
  `crates/nvs-cli/src/bundle.rs`'s `.nvsx` embedding is M9's; the inlining items in
  `crates/nvs-runtime/src/decimal.rs` and the string fast path in `crates/nvs-runtime/src/lib.rs` are
  M12's. Only a gap in a milestone that has *already been carried*, and that no chain entry claims, is
  unowned. Goal `gap-owners` makes this distinction machine-readable.
- **A decision is not a gap.** `crates/nvs-stdlib/src/time.rs`'s "there is no `Core\Month`, and there
  is not going to be one" and `crates/nvs-syntax/src/casing.rs`'s "left out deliberately" are settled
  positions that happen to sit under a `# Known gaps` heading. Goal `gap-owners` moves them out of the block, so
  the roster counts what is owed.

**The list this file indexes is not the whole inventory.** Every crate's own `# Known gaps` block holds
enumerated items — `python tools/owners.py` counts them and prints who owns each — and this file names
the ones whose ownership needed an argument, while
[carried-refusals.md](carried-refusals.md) 901 covers `nvs-ir`'s. The rest were unindexed until goal `gap-owners`,
which puts the owner in the module doc beside the gap and derives the roster rather than copying it —
this file's own contract, applied one level down.
