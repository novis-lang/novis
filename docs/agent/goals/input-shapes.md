---
milestone: M7
---
# Loop goal 20 — untrusted input becomes a declared shape, at one converter

Reading a request today ends in `mixed`. `Core\Request::post('user[address][city]')` answers `mixed`,
`query` answers `mixed`, and goal `request-json`'s `json()` answers `mixed` too — so every step a handler takes after
the boundary is unchecked, and the first thing that notices a peer sent `city` as a list is whatever the
program did with it three frames later. The language already has the answer to this and has never pointed
it at input: [ADR 0036 § 3](../../decisions/0036.md)'s inline shape type is a
structural, width-subtyped constraint the checker verifies, and
[ADR 0007 § 5](../../decisions/0007.md) already lets a call site write the type argument
for a compiler-owned member — the door `Core\Json::decodeAs<T>` and `Core\Db::queryAs<T>` go through.

So this goal builds the third door and makes it the general one: **one converter from `array<mixed>` to a
declared shape**, plus the two request wrappers where the boundary actually is. Untrusted data is checked
once, where it arrives and where `400` is still the right answer, and downstream code holds a value whose
fields are proven rather than an array whose keys are hoped for.

Two things in the type surface have to move first, and stage 2 is why this goal is not just four `Core`
members. A shape's fields are all **required** today, so a form with an unchecked checkbox — a key that is
simply absent — cannot be described at all. And `tainted` attaches to `string`/`bytes` and nothing else
([ADR 0024 § 1](../../decisions/0024.md)), so a shape over a request body has
to write the qualifier once per field, which is verbose in proportion to the form and is the reason people
ask for the check to be turned off.

Its floor is goal `test-request`'s whole list, which is the parity program, the six post-parity goals, M4B, and the
two request goals before this one.

## Why here

Last because its stage 2 is type-surface work (`rule:types/object-top`'s optional field,
`rule:security/tainted-qualifier`'s qualifier over a shape) and every goal before it writes shapes
whose spelling this one widens; running it earlier would mean writing them twice. Its stages 4-5 are
proven over goal `request-json`'s `.nvst` request sections and goal `test-request`'s builder, so both have to exist first.

## What the members answer, in the two registers the cards carry

**In simple words**: *`shapeAs<T>()` — an array of loose values, read as a type you declared. Every field
the type names is checked; a field it does not name is left behind. What does not fit is one error naming
every field that did not fit, not the first one. `postAs<T>()`, `queryAs<T>()` — the same thing over the
form or the query string, which is where the loose values usually come from.*

**In detailed words**: `Core\Arr::shapeAs<T>(array<mixed> $a): T` hydrates an array into an inline shape
or a class carrying a `Codec`, field by field, using the language's own `as` operator and its table
([ADR 0007 § 2](../../decisions/0007.md)) as the whole conversion rule — `"42"` into a
`uint` field is the `string → uint` row, so `"42abc"` and `""` throw exactly where `as` throws, and a `?T`
field gets `as ?T` (`rule:expressions/nullable-conversion`) and answers `null` where
`as T` would have thrown. Nothing new enters the conversion table. A key the shape does not name is
ignored (`rule:types/shape-type`'s width subtyping, unchanged); a required key that is absent, and a value that
does not convert, are failures — and **every failure of one call is collected into one `ParseError`**,
each named by its dotted path, which is the shape `rule:core-classes/derive-attribute`'s derived
hydration already throws. `Core\Request::postAs<T>({name?: string}): T` and `queryAs<T>` are that member
over the form and the query string, whole or at one bracket-named subtree.

## Stage 1 — the floor

Goal `test-request`'s whole acceptance list, never traded.

## Stage 2 — the keystone: the two things a shape cannot say yet

Nothing after this is writable, and both edits are smaller than they look because the precedent for each
is already in the file being edited.

1. **An optional field, `{name?: T}`.** `Ty::Shape(Vec<(String, TypeId)>)`
   (`crates/nvs-types/src/ty.rs:250`) gains a required bit per field. **The bit already exists one struct
   away**: `CoreShapeField::required` (`:359`) is exactly this for a Core options bag under
   `rule:core-api/shape-parameter`, so this is the
   user-facing half of something the Core half has had since that ADR landed — one representation, not a
   second one. The parser is `crates/nvs-syntax/src/parser/ty.rs`. **`{a?: T}` and `{a: ?T}` are different
   types and both are legal**: the first says the key may be absent, the second that it must be present
   and may hold `null`. Collapsing them is the ambiguity goal `request-json` refused for an empty body, one storage
   kind along.
2. **A qualifier over a shape, `tainted {…}`.** `rule:security/tainted-qualifier`'s grammar admits `'tainted'? scalar_type` and
   nothing else; it is widened to admit a shape type, distributing to every text-carrying field
   transitively — through nested shapes and through an `array<string>` element. **This needs no new `Ty`
   variant and no qualifier axis**: taint is spelled as variants (`Ty::TaintedString`, `TaintedBytes`,
   `SecretTaintedString`, `ty.rs:48-62`), so `tainted {…}` is a desugaring at parse time that rewrites
   each text-carrying field to its tainted variant and is gone before the checker sees it. A shape with no
   text-carrying field anywhere is a diagnostic, not a no-op — a qualifier that promises nothing still
   reads as a promise.
3. **Assignability learns the bit** — `is_assignable` (`crates/nvs-types/src/expr/assign.rs`): a source
   missing an *optional* field satisfies the shape, missing a *required* one does not, and a source with
   extra fields still satisfies. That last clause is `rule:types/shape-type` unchanged and is restated here only
   because this goal is where someone will be tempted to change it.

## Stage 3 — the converter

`Core\Arr::shapeAs<T>` in `crates/nvs-stdlib/src/arr.rs`, the five edits, plus:

1. **The type-argument door gains its third member.** `rule:types/arrays`'s list — a call site may write the type
   argument for a compiler-owned member that declares one it cannot infer — is `Json::decodeAs<T>` and
   `Db::queryAs<T>` today; the ADR's own sentence is amended to name three. `crates/nvs-types/src/expr/args.rs`
   is where a written argument is bound.
2. **One hydration walk, one home.** `Json::decodeAs<T>` over an inline shape is `decode` — which produces
   only arrays and scalars — followed by exactly this walk, so it calls it rather than keeping its own.
   Two field-error conventions in one runtime is how a program learns to catch two things.
3. **The failure is collected, not the first one** — one `ParseError` carrying every field that failed,
   each with its dotted path (`user.address.city`), built the way `rule:core-classes/derive-attribute`'s derived hydration builds its.
   A handler maps it to `400`; a form UI reads the list.
4. **What is a failure**: an absent required key, and a value `as` refuses. What is not: an absent
   optional key, an extra key, and a `?T` field whose value `as ?T` answers `null` for.

## Stage 4 — the two members at the boundary

`Core\Request::postAs<T>` and `queryAs<T>` in `crates/nvs-stdlib/src/request.rs`, beside `post` (the rows
at `:255`, the cards after them, the bodies, the `address()` arm at `:982`):

1. **One member each, not two.** `postAs<T>({name?: string}): T` reads the whole form; with `name` it
   reads one bracket-named subtree under `post`'s existing convention. `rule:core-api/shape-rules`
   R15 — one name, one signature, optional arguments the only variance — is why this is one row.
2. **The whole-form read walks nothing new.** `crate::uri::parse_query` already answers the entire array
   and `post(name)` indexes into it (`request.rs:1229`), so the whole-form spelling hands over the array
   that already exists. **This does not open a public `post(): array<mixed>`** — `$_POST` stays closed,
   and the array is reachable only through a declared shape.
3. **`postAs` is a buffering reader** under goal `request-json`'s body-read rule — unchanged and not re-litigated: it
   may follow another buffering reader and it refuses after a streaming one.
4. **Three `.nvst` cases each**, over goal `request-json`'s stage 2 request sections and goal `test-request`'s builder.

## Stage 5 — the proofs

`examples/input-shapes.nvs` as the runnable fixture, reference pages for the three members, and the
diagnostic corpus stage 2 owes: an unqualified shape at a tainted call site names the shape, a missing
required key names the key, and a shape whose `tainted` promises nothing is refused.

## Standing decisions

- **There is no shaped `array<T>`, and this goal does not add one.** Making every array key access
  shape-checked was the user's opening question and it is rejected on the merits: `array<T>` is
  homogeneous, invariant, and carries one interned type descriptor per header
  (`rule:types/declaration`), so a per-key-typed array is a second array type
  family with its own descriptor, variance and `Core\Arr` signature story — and `rule:types/object-top` already explored
  a general structural record type and rejected it as more machinery than the need justifies. Converting
  at the boundary is strictly the safer half of that trade anyway: it checks the data where it arrives and
  where a `400` is still the right answer, and after it nothing downstream is holding a `mixed` to check.
- **Both spellings land — the general converter and the two wrappers.** Settled, and not re-opened on
  `rule:core-api/tier-placement` test 6 grounds: the wrappers are one call each over
  one mechanism, and the request boundary is the only place the taint diagnosis actually pays.
- **Extras are ignored, and there is no `exact` option.** A client adding a field must not break a server.
  A flag choosing between two behaviours is what `rule:core-api/shape-rules` R4 refuses.
- **The taint diagnosis stays on, and `tainted {…}` is the answer to its verbosity** — not an opt-out, not
  an auto-taint that leaves the declaration understating the value, and not a `taintedPostAs` spelling: no
  `Core` member names a qualifier today (`body`, `post`, `query`, `header`, `cookie` and `Part::filename`
  are all tainted and silent about it), and the first one that did would make every other member's name
  read as a claim it is not making.
- **A conversion is `as` and only `as`.** No coercion table is written for this member. If a row is
  missing, the fix is `rule:types/conversion`'s table, where every other conversion in the language already reads.
- **This goal may open one new record and no other number.** Everything else is a change to a rule that already stands, through a record whose `changes:` block
  names it: `rule:types/shape-type` (the optional marker, and the two spellings it distinguishes), `rule:security/tainted-qualifier`
  (the qualifier grammar), `rule:types/arrays` (the third member at the type-argument door), `rule:core-api/shape-rules` R15's
  worked list, and spec §§ 6 and 15's rosters.
- **Goal `request-json`'s `json(): tainted mixed` is settled here, not there.** `rule:security/tainted-qualifier`'s grammar admits that
  spelling no more than it admits `tainted {…}`, so stage 2 either widens to cover `mixed` as well or
  goal `request-json`'s signature is corrected to what the grammar allows. Decided-and-recorded in `rule:security/tainted-qualifier`'s own
  body, never `BLOCKED`.
- **What this spends**, per `rule:programs/memory-priority`'s ledger: the hydrated
  object for a request that asked for one, freed with the request, O(in-flight) and never O(requests
  served). The optional bit is one bool per shape field in an interned descriptor, O(distinct types in the
  program). `tainted {…}` costs nothing at all — it is erased before codegen with the rest of the
  qualifier, exactly as `rule:security/tainted-qualifier` already promises.
