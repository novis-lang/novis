---
milestone: M7
---
# Loop goal 21 — a class a string names, at one contract

Three places in this compiler ask the same question — *may this class be built from text?* — and all
three answer it by comparing a name against the string `Core\Uuid`:

- `crates/nvs-types/src/commands.rs:694`, the arm `Ty::Class(name, _) => *name == QName::parse(r"Core\Uuid")`
  inside `converts_from_string` — which its own doc comment calls "the one home for *what an argument's
  text may become*", read by `rule:security/route-capture-is-laundered-by-its-type`'s command arguments and § 6's
  options, and by `rule:routing/a-capture-narrows-to-a-closed-set`
  's path captures and § 3's `#[Query]` parameters. One predicate, four surfaces, one blessed name.
- `crates/nvs-cli/src/openapi.rs:372`, `Some(r"Core\Uuid") => json!({"type": "string", "format": "uuid"})`.
- Three diagnostics that spell the roster out in prose and end in the same name —
  `commands.rs:660`, `routes.rs:1713`, `routes.rs:1799`.

A user's `Slug`, `Isbn` or `TenantId` is refused at every one of them, and there is nothing true of it
that is not also true of `Core\Uuid`: text arrives, a checked parse either produces the value or does not,
and a failure is a `404` or a usage error rather than something the handler was trusted to remember. The
roster is a roster because the language had no way to *say* what `Core\Uuid` is.

So this goal says it. **`Parses` is a global interface — one required member, one default body — and
`converts_from_string`'s class arm becomes "the class implements `Parses`".** `Core\Uuid` stops being a
name in a match arm and becomes a class that carries the members, the way a `Core` class already satisfies
`rule:classes/comparable`'s `Comparable` (`crates/nvs-types/src/core_lib.rs:76`).
Every other class in the program gets the same door on the same terms.

Its floor is goal `input-shapes`'s whole list, which is the parity program, the six post-parity goals, M4B, and the
three request goals before this one.

## Why here

goal `parses`'s record: `Parses` — the global interface that replaces `converts_from_string`'s `Core\Uuid`
name test, so a route capture, a `#[Query]`, a command argument and an option admit any class that
declares it can be built from text. Last because it is the only entry that edits a predicate all
four of those surfaces read, and every goal before it writes routes and commands against the roster
as it stands; running it earlier would mean rewriting their fixtures when the roster opened. Its
stage 4 closes the two known gaps goal `server` and the CLI work left behind by name, so both have to exist
first.

## The contract, in the two registers the cards carry

**In simple words**: *a class that can be built from a piece of text says so, by implementing `Parses`.
Then the compiler lets it stand where a `Core\Uuid` stands — a route's `{capture}`, a `#[Query]`, a
command's argument — and a value that does not parse is refused at the door instead of reaching your
code.*

**In detailed words**:

```nvs
interface Parses {
    public static function parse(tainted string $s): static;

    public static function tryParse(tainted string $s): ?static {
        try { return static::parse($s); } catch (RuntimeError $e) { return null; }
    }
}
```

**One required member and one default body, not two required members.** This is the whole reason the
contract is expressible without re-opening a decided question: `rule:expressions/try-parse` condition 2 says `tryParse` **is** `parse` plus a caught throw and never a second implementation,
and its stated reason is CVE-2024-5458 — two implementations of one predicate that agree until they do
not. A contract requiring both would hand that failure back to every implementor. A default body
(`crates/nvs-hir/src/interfaces.rs`, and the interface default-method rule the language already has) makes
the drift unwritable rather than merely discouraged.

The binding sites call `parse` where a failure is a refusal they own — a route capture that does not
parse is no match, a command argument that does not parse is a usage error — and `tryParse` where the
`null` is the answer, which is `#[Query]` on a parameter with a default. Both are **static calls to a
statically known class**: the parameter's declared type is what named the class, so nothing here needs
dynamic dispatch and stage 4 stays four match arms wide.

## Stage 1 — the floor

Goal `input-shapes`'s whole acceptance list, never traded.

## Stage 2 — the keystone: `Parses` on the reserved roster

Nothing after this is writable. It is small because both precedents are in the files being edited.

1. **`Parses` joins `nvs_hir::interfaces::RESERVED`** — `crates/nvs-hir/src/interfaces.rs:46`, the
   `("Comparable", &[]), ("Stringable", &[])` list, plus the name constant beside `COMPARABLE` (`:55`)
   and `STRINGABLE` (`:59`). No type parameters, exactly as those two take none.
2. **Conformance owes the member** — `crates/nvs-types/src/conformance.rs:41` is where a class claiming a
   roster interface is checked for what it owes, and its own test at `:570` (`class Money implements
   Comparable {}` names `compareTo`) is the shape of this one's. A class claiming `Parses` and declaring
   no `parse` is a compile error naming the member and the interface.
3. **The default body is inherited, and overridable.** `tryParse` is a `public static` method with a body
   on the interface — the language already admits both (novis.md § A.5's interface members), so this is a
   declaration, not a mechanism. An implementor *may* override it; one that does is writing the second
   implementation `rule:expressions/try-parse` warns about and gets no diagnostic for it, which is the same latitude
   `Comparable::compareTo` has.
4. **`static` as the return type** is what makes an implementor's `parse` answer its own class rather than
   the interface — the language's existing late-static-binding return (novis.md § A.5, `: static`), not a
   new rule.

## Stage 3 — the predicate learns the contract, and `Core\Uuid` stops being a name

This is the streamlining, and it is where the goal pays for itself.

1. **`converts_from_string`'s class arm** — `crates/nvs-types/src/commands.rs:694`. The name comparison
   becomes "this class implements `Parses`", asked of the same signature table `expr/operators.rs:604`
   asks `Comparable` of. **Every one of the four surfaces changes at once** because they all read this
   one predicate, which is what its doc comment promised and this goal is the first thing to collect on.
2. **`Core\Uuid` satisfies it by carrying the members** — `implements_parses` beside
   `implements_comparable` (`crates/nvs-stdlib/src/registry.rs:2311`), asking the same structural
   question of the `CoreClass` rows: a static `parse(string): Instance(self)` and a static
   `tryParse(string): Nullable(Instance(self))`. `Core\Uuid`'s rows already are exactly that
   (`crates/nvs-stdlib/src/uuid.rs:139` and `:148`), so **not one line of `Core\Uuid` changes** — it
   simply stops being named elsewhere. Seeded in `crates/nvs-types/src/core_lib.rs:83`, the two lines
   below the `implements_comparable` seed.
3. **The three diagnostics stop reciting the roster.** `commands.rs:660`, `routes.rs:1713` and
   `routes.rs:1799` name the closed set and then `Core\Uuid`; they name the closed set and then "a class
   implementing `Parses`". A diagnostic that lists a name the language can now describe is a list that
   goes stale the first time someone writes the interface.
4. **`routes.rs:492`'s doc comment** and `closed_set`'s (`routes.rs:1726`) both use `Core\Uuid` as the
   worked example of "converts, but names no set anything could be checked against". That sentence stays
   true and is re-worded to the general case: a `Parses` class converts and narrows nothing, so it is
   `converts_from_string`'s business and never `closed_set`'s.

## Stage 4 — the runtime arm, which closes two known gaps at once

Both gaps are already written down as gaps, in the two modules that own them, with the same diagnosis:
*the conversion exists as a `Core` member and what is missing is the arm.*

1. **`CaptureConv` gains no parsing arm, and that is the decision rather than an omission** —
   `crates/nvs-runtime/src/routes.rs:88`, and gap 3 at `:76`. The router narrows on the conversions it
   reads natively, `CaptureConv::Uuid` (`:418`, `crate::uuid::read`) among them, and a capture typed as
   any other `Parses` class stays `CaptureConv::Unconverted`: it matches on shape, and the class's own
   `parse` runs at the binding site where the program dispatches. `crates/nvs-cli/src/main.rs:1413`'s
   `Some(r"Core\Uuid") => CaptureConv::Uuid` therefore stays a name arm, because it answers which types
   the *router* reads and not which may stand in a capture — and only the second question is the roster
   this goal deletes. *Standing decisions* below is the settlement and what it costs.
2. **`ArgConv` gains the same arm** — `crates/nvs-runtime/src/commands.rs:57`, gap 1 at `:41`: four of
   § 6's conversions are `Unconverted`, of which `Core\Uuid` is one. A command argument `parse` refuses is
   a usage error at the moment it is run, which is what every other `ArgConv` arm already does.
3. **`decimal` rides along, and it is not scope creep** — it is named in both gaps beside `Core\Uuid`,
   it is in `converts_from_string`'s roster, and it is one arm in each of the two matches this stage is
   already editing. Leaving it would mean a session opening these two files again for one line each.
   The enum and literal-union arms of commands gap 1 are **not** in scope: they are `closed_set`'s
   business, they have no `Parses` story, and they are the half `crate::commands` gap 1 shares with
   `rule:routing/a-capture-narrows-to-a-closed-set`'s enum-case subset, which `routes.rs:1726` records as out of scope for a reason of its own.

## Stage 5 — the proofs

`examples/parses.nvs` as the runnable fixture: a user class implementing `Parses`, standing in a route
capture, a `#[Query]` with a default, and a command argument, with one segment that parses and one that
is refused at the door. Plus:

1. **The OpenAPI schema** — `crates/nvs-cli/src/openapi.rs:372`. A `Parses` class is
   `{"type": "string"}`, since text is what it is built from and the class itself says nothing narrower.
2. **The reference pages** — `Parses` is a global interface and belongs beside `Comparable` and
   `Stringable` in novis.md § A.5, and in the routes/commands rosters at novis.md:5725 and :5927, which
   are the two places the `Core\Uuid` list is written out in prose.
3. **The diagnostic corpus**: a class claiming `Parses` with no `parse`; a class standing in a capture
   without the interface, naming the interface as the fix; and a `Parses` class in a `#[Query]` with no
   default, which is the site that needs `tryParse` and gets `parse`'s refusal instead.

## Standing decisions

- **This goal does not touch `as`, and that is the decision, not an omission.** The user's opening
  proposal was `$s as Slug` gated on this same interface, and it was rejected on three grounds worth
  restating so no session re-derives them. **One**: `mixed as Foo` is already a checked downcast, so a
  parsing meaning would make the operator pick its behaviour from the operand's runtime tag — a `mixed`
  holding request text would take the parse path where a downcast was written, which is priority 1 in
  AGENTS.md's ordering and not tradeable. **Two**: `as` is a closed laundering set today (novis.md:1237),
  and a user-extensible `as` makes every implementor a taint launderer to audit — the cost
  `rule:expressions/nullable-conversion` refused to pay for `Core\Duration` alone.
  **Three**: `X as ?Foo` would become legal or illegal on two inputs at once, the operand's type and the
  target's implements-list, so a conversion could not be read without resolving a class. `rule:expressions/nullable-conversion-availability`'s
  *the class row is absolute* survives this goal intact; what changes is only that the sites which
  already convert implicitly stop naming one class by hand.
- **`parse` takes `tainted string`, and this is not a laundering hole.** The text at every binding site
  arrived from outside the process, so the contract says so rather than letting each implementor
  discover it. The object it answers carries no taint because taint is a property of `string`/`bytes` and
  not of a class (`rule:security/tainted-qualifier`) — and an
  implementor storing the text in a plain `string` field is refused by the *existing* assignability rule,
  with no new rule written for it. **Safe fallback if a plain `string` argument turns out not to assign
  to a `tainted string` parameter**: declare the parameter `Qual::Contagious` under
  `rule:security/unclassified-parameter-refuses-tainted`, which is the admission every `Core`
  member in this position already uses. Recorded in `rule:security/tainted-qualifier`'s body either way.
- **The converted value at a binding site is not `tainted`** — `rule:routing/a-query-parameter-is-declared-like-a-capture`'s existing sentence,
  unchanged and not re-argued. It held for `Core\Uuid` because `parse` checks; it holds for a `Parses`
  class for the same reason and no other.
- **A program's `parse` runs after the match; the router's own conversions decide the match.**
  `converts_from_string` becomes a predicate for one question — may this type stand in a capture — and
  never for the other, which is what the *router* narrows on. It narrows on what it reads natively:
  `int`, `uint`, `decimal`, `Core\Uuid` and a closed set, where a segment it refuses is no match and
  then a `404`, all unchanged. A capture typed as any other `Parses` class matches on shape and
  converts at the binding site, so a segment that class refuses is a `400`.
  **Why the split sits there:** matching runs at the door with no program installed —
  `crates/nvs-server/src/route.rs:85` holds no `Ctx`, and `crates/nvs-cli/src/runner.rs:504` matches
  before `unit.install_in(ctx)` — so arming a class table ahead of it would put an implementor's
  `parse`, a body that can loop, allocate and throw, on every request URL including the ones that match
  nothing. That is the priority-1 objection `rule:routing/a-capture-narrows-to-a-closed-set` already
  makes to a regex, and a `parse` body is strictly more than a regex. `Core\RateLimit` and the access
  decision are both the program's own (0075 § 4, 0102 § 8), so binding at dispatch is what puts a
  program's parsing behind them. This is *A `Parses` class narrows nothing* below, read at run time.
- **What that costs, said rather than implied:** a `Core\Uuid` capture that will not convert falls
  through to the next route and a user class's capture does not, so two captures spelled the same way
  fail two ways — a `404` and a `400`. The line between them is *who runs at the door*, it is one
  sentence to document, and it is the line the rulebook already draws when it sends a `[a-z0-9-]+` slug
  to a `Core\Validate` check inside the handler answering `400`.
- **The rule edits and ADR 0160 land in the commit that implements the binding site, not before it.**
  `rule:security/route-capture-is-laundered-by-its-type`'s *a failed conversion is not a match* and
  `rule:routing/a-bad-query-value-is-a-400`'s *a path capture that fails is a `404`* are both true of
  the tree as it stands, and both gain the class-typed exception the day the binding site exists —
  `conventions.md` § *A decision record*: a rule and the record behind it are one commit.
- **`Core\Uuid` keeps its `format: uuid`, and it is one of the two name tests that survive.** The other
  is `main.rs:1413` above, and both survive for the same reason: they answer what the *engine* does
  with a type it ships, not which types the language admits. A `Parses` class
  answers `{"type": "string"}`; a *named format* is a documentation hint over that, not a conversion rule,
  and a short list of them in `openapi.rs` is the right home for a short list of them. Adding a way for a
  class to declare its own OpenAPI format is a real feature and it is not this goal's.
- **No `Parses` implementor is discovered, enumerated or auto-registered.** The class is named by a
  parameter's declared type at every site, so nothing here needs
  `rule:programs/no-runtime-autoload`'s program query and nothing
  gains a registry.
- **A `Parses` class narrows nothing.** It is `converts_from_string`'s business and never `closed_set`'s
  (`routes.rs:1737`), so it changes no route's rank, no precedence and no `Core\Router::url`. Two routes
  distinguished only by a `Parses` capture's *content* are the same route, exactly as two `{id}` captures
  are today.
- **This goal may open one new record and no other number.** Everything else is a change to a rule that already stands, through a record whose `changes:` block
  names it: `rule:classes/comparable` (the roster gains a third global interface, on the precedent it set),
  `rule:expressions/try-parse` (the `tryParse` shape is now a contract and its three conditions are what the interface
  encodes), `rule:security/route-capture-is-laundered-by-its-type` and `rule:routing/a-query-parameter-is-declared-like-a-capture` and `rule:routing/a-capture-narrows-to-a-closed-set` (the type roster's last entry becomes a predicate), and
  novis.md's two prose rosters.
- **What this spends**, per `rule:programs/memory-priority`'s ledger: on the command side, nothing at
  run time that the roster did not already spend — a conversion that was a match arm stays one, and the
  arm now carries a class pointer the compiled table already holds. On the route side a `Parses`
  capture costs one `call_static` into the program per such capture, on the matched route and nowhere
  else: the walk itself is unchanged, a request that matches nothing pays nothing, and a segment the
  class refuses never builds the `String` a bound capture holds. At compile time it is one structural
  query per class per binding site, memoized in the signature table `Comparable` is already asked of.
