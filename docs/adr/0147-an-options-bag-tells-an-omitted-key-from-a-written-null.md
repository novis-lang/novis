# ADR 0147 — An options bag tells an omitted key from a written `null`, and `null` is the one spelling that removes

- **Status:** Accepted
- **Date:** 2026-09-06
- **Scope:** how a `Core` member's trailing options bag ([0063](0063-core-api-conventions.md) R2) and a
  fixed-key shape parameter ([0135](0135-a-core-shape-parameter-is-one-coretty-carrying-its-arms.md))
  let a call site say *"leave this alone"* and *"clear this"* as two different things; what an omitted
  **nullable** field materializes at the ABI; what a member is then allowed to read that distinction
  as; and the three members on `Core\Uri` that spend it first, § 5. Not in scope: which *other*
  members adopt it, which is each member's own row in
  [docs/spec/01-core-library.md](../spec/01-core-library.md) — that file stays the home of every
  signature, and a row lands there with its implementation, never ahead of one; a **user-declared**
  function's optional parameter, which is unchanged and § 3 says why; and the derived-codec table in
  [0071](0071-derived-codecs.md) § 4, which already separates the two questions for a decoded payload
  and needs nothing from this.
- **Depends on:** [0063](0063-core-api-conventions.md), whose R2 bag is the surface this changes;
  [0135](0135-a-core-shape-parameter-is-one-coretty-carrying-its-arms.md), whose § 3 fill list is the
  mechanism; and [0022](0022-definite-property-initialization.md), whose § 3 storage state this
  reuses rather than inventing a second one.
- **Amends:** [0063](0063-core-api-conventions.md) R2 — an optional bag field may be nullable, and a
  member may read omitted and written-`null` as two different requests.
  [0135](0135-a-core-shape-parameter-is-one-coretty-carrying-its-arms.md) § 3 — a shape field is no
  longer *never* nullable; a nullable one is admitted and its omission fills a different constant.
  [0022](0022-definite-property-initialization.md) § 3 — the never-written marker's scope widens from
  a property slot to an omitted nullable option's call-site fill, keeping all three of that section's
  containment properties.
- **Validated by:** [crates/nvs-stdlib/src/registry.rs](../../crates/nvs-stdlib/src/registry.rs) —
  the pair that today holds the invariant § 1 replaces, `a_shape_field_is_never_nullable` and
  `a_union_option_excludes_null`. § 1 renames them to `a_nullable_shape_field_omits_as_the_never_written_marker`
  and `a_nullable_option_omits_as_the_never_written_marker` and they hold the pairing rule instead,
  over the same rows; they land with the implementation, at loop goal 28 stage 2.

> **In short:** an optional field of an options bag may now be **nullable**, and the call site's two
> ways of not giving it a value stop being one. `{}` leaves a component alone; `{fragment: null}`
> removes it. The mechanism is one constant, not a sentinel: an omitted **nullable** field fills
> `rule:classes/an-unwritten-property-read-throws`'s already-existing never-written marker
> instead of `null`, so the helper reads three states from one ABI argument and
> [ADR 0135](0135-a-core-shape-parameter-is-one-coretty-carrying-its-arms.md) § 3's flattening —
> one argument per field, no runtime shape, no new calling convention — survives untouched. A
> **non**-nullable field is completely unaffected and every existing member lowers byte for byte as
> it does today. The semantic half is as fixed as the mechanical one: **wherever a `Core` member
> admits a written `null` — a bag field or an ordinary argument — it means *remove***, never a second
> operation a member invented, so a reader who has seen one has seen all of them. `Core\Uri` spends it
> first and at two levels: `with` clears a component, and the pair `queryParameter` /
> `withQueryParameter` edits one query parameter by name, which `with` cannot do because a bag's keys
> are declared and a parameter's name is chosen at run time.

## Context

- **The bag has no way to say "clear this", and the reason is representational rather than considered.**
  [ADR 0135](0135-a-core-shape-parameter-is-one-coretty-carrying-its-arms.md) § 3 flattens a bag into
  one ABI argument per field and fills every field the caller left out with `Const::Null`. That makes
  `null` the *marker for omission*, so a field that could itself hold `null` would arrive at the
  helper as the same argument whether it was written or not. The invariant that follows — a bag or
  shape field is never nullable — is held by two tests, `a_union_option_excludes_null` and
  `a_shape_field_is_never_nullable`, and restated at four more places in the tree.
- **`Core\Uri::with` is where the cost shows.** Its own `written` helper states the consequence
  exactly: *"with no second null to spend, a clearing spelling would have to overload a legitimate
  value"*. So `with` replaces and never removes, and a program that wants a URL without its fragment
  has to recompose the text and re-parse it — through the same `parse` whose whole job was to save it
  from doing that. `""` cannot stand in: an empty query is what `?` with nothing after it produces and
  `query()` already reports it as distinct from `null`, and `file:///tmp` has an empty host.
- **It is about to bind a second member.** [ADR 0146](0146-a-signature-is-over-a-payload-and-a-url-is-a-payload-core-uri.md)'s
  `$uri->sign({keys, until})` makes `until` a *required* key holding a nullable value precisely
  because the bag cannot offer an optional one — `{until: null}` is the forever spelling and a lifetime
  is written rather than omitted. That reading is the right one for a signature and it stays, but it
  was reached with one hand tied.
- **The distinction is not a new idea in this repository; it is one the bag alone lost.**
  [ADR 0071](0071-derived-codecs.md) § 4 already answers the same question for a decoded payload with
  a four-row table, on the principle that *nullability is a property of the type and optionality is a
  property of the default; neither borrows the other's meaning*. A written `?int $rank = null` at an
  ordinary parameter has always worked. The bag is the one position where the two collapsed, and only
  because the marker for "not written" was itself a value of the field's own type.
- **A sentinel is not available.** Any in-band marker — `""`, a magic string, a reserved constant — is
  a value the field's own type admits, so user data can arrive as one by accident. That is the class
  of bug this decision exists to remove, not to relocate.

## Decision

### 1. An optional field may be nullable, and its omission fills a constant of its own

A field of a `CoreTy::Options` bag or a `CoreTy::Shape` arm carries a default, and that default is what
an omitting call site materializes. Three states, and the registry declares which of them a field has:

| Declaration | Key absent | Key present as `null` |
|---|---|---|
| no default | a compile error: *required key missing* | a compile error: `null` is not of the field's type |
| a default, non-nullable type | the default is materialized | a compile error: `null` is not of the field's type |
| a default, **nullable** type | the never-written marker is materialized | `null` is materialized |

Only the third row is new. The two tests that hold the old invariant become
`a_nullable_shape_field_omits_as_the_never_written_marker` and
`a_nullable_option_omits_as_the_never_written_marker` — the name has to move with the rule, since
"never nullable" is the thing that stopped being true — and what they assert becomes the pairing:
**a field admitting `null` — a `CoreTy::Nullable`, a `CoreTy::Union` with a null arm, or
`CoreTy::Mixed`, which admits one without spelling it — is admitted exactly when its default is the
never-written marker**, and a field not admitting `null` is admitted exactly when its default is
`Const::Null` or a literal. Neither test becomes weaker; both change what they assert to the rule
that is now true, and a row that gets the pairing wrong still fails the build.

### 2. The ABI does not change, and neither does any existing member's lowering

[ADR 0135](0135-a-core-shape-parameter-is-one-coretty-carrying-its-arms.md) § 3's flattening is the
part worth protecting: a bag is **one ABI argument per field**, no runtime representation of a shape
exists, and no helper learns a new calling convention. All of that is untouched. What changes is which
constant fills one slot, for one kind of field.

The marker is `rule:classes/an-unwritten-property-read-throws`'s `Tag::Unset` — already
defined as *"distinct from every legal value including `null`"*, already costing **zero additional
bytes** because it is one more discriminant on a representation that carries one, and already
non-refcounted, so it raises no ownership question at a call boundary. Reaching it from a call site
costs one `ConstArg` variant, one `InstKind` constant beside `ConstNull`, and the codegen arm that
writes the tag byte. `rule:programs/memory-priority` requires the spend be stated: **nothing
per request, and nothing per call** — the omitting call site emits one constant either way.

A **non-nullable** field is unaffected in every respect, which is what makes this additive: every
member registered today keeps `Const::Null` as its omission fill and lowers to the same instructions,
so nothing needs migrating and no helper that does not want the distinction has to learn it.

### 3. The marker still never reaches a program

`rule:classes/an-unwritten-property-read-throws` admits its state on three conditions, and all
three survive here:

- **It is not in the type system.** The field's declared type is `?T`; the marker is not a member of
  it. Nothing in `nvs_types` produces one from a source expression, and no expression evaluates to
  one — the call site that "writes" it writes *nothing at all*, and the compiler materializes the fill.
- **It is never handed to user code.** Its only reader is the native helper behind the member, which
  turns it into the member's own behaviour before anything returns. A helper that fails to handle it
  is a `Fault::fatal` — a contract violation in this repository's own code, the way an unexpected tag
  in a slot already is — never a wrong answer handed back to a program.
- **It is transient.** It exists for the duration of the call that materialized it and is consumed by
  the helper; it is never stored, never returned, and never reachable from a value a program holds.

**A user-declared function's optional parameter is unchanged**, deliberately. `?int $x = null` in
program source still cannot tell an omitted argument from a written `null`, because closing that would
need a spelling for asking the question — an `isset`-on-a-parameter, or a third state a program can
observe — and that is a language-surface decision this one does not open. A `Core` helper is native
code that can be held to reading three states by a test; a user's function body cannot be, and giving
it a state it has no way to name would be the observable marker § 3 above refuses.

### 4. Where a `Core` member admits a written `null`, it means remove — and never a second thing

The mechanical half without the semantic half would buy one bug for another: a bag whose `null` means
*clear* on one member and *reset to the default* on the next, or *use the server's value* on a third,
is a table a caller has to memorize per member. So the meaning is fixed with the mechanism.

The rule is stated over a `Core` member's written `null` rather than over a bag field, because **the
position the `null` arrives in is incidental to what it means**. §§ 1-3 are what make a *bag field*
able to carry one at all; an ordinary argument could always carry one, and where it does the meaning
is the same. `Core\Uri::buildQuery` is the case that already behaved this way before this ADR existed
— a pair whose value is `null` is dropped rather than written, which is `http_build_query`'s
behaviour and the only one that round-trips, since a query string cannot spell an absent value. That
is this rule arrived at independently, which is the best evidence it is the right one; it is stated
here so the next member does not have to rediscover it.

- **A written `null` means the component, option, parameter or setting is not present in the result.**
  *Clear*, *remove*, *none*. It never means "restore a default", never selects an alternative
  behaviour, and is never a flag under another name.
- **An omitted key means the receiver's existing value is carried through unchanged**, which is what it
  already means everywhere.
- **An input is made nullable only where removal is a thing the member can actually do.** A member with
  no removal to offer keeps its non-nullable field, and writing `null` into it stays the compile error
  it is today. This is the rule's teeth: the nullability in the signature *is* the announcement that
  the thing can be cleared, so a caller reads it off the type rather than off prose.
- **`""` is never a removal spelling**, on any member. It is a legal value of most of these fields and
  is already distinguishable — an empty query is not an absent one — so overloading it would reinstate
  exactly the in-band sentinel this decision removes.

Folded into [ADR 0063](0063-core-api-conventions.md) R2 as the bag's own rule, since R2 is where a
reader looking for what a bag may contain arrives.

### 5. `Core\Uri` is the first class to spend it, and it does so at two levels

**Three of its six fields become nullable — `port`, `query` and `fragment`** — and a written `null`
removes the component:

```nvs
var $u = Core\Uri::parse("https://example.com:8443/a?x=1#top");
echo $u->with({fragment: null})->toString(), "\n";           // https://example.com:8443/a?x=1
echo $u->with({port: null, query: null})->toString(), "\n";  // https://example.com/a#top
echo $u->with({query: ""})->toString(), "\n";                // https://example.com:8443/a?#top
```

The other three stay non-nullable, and § 4's last rule is why — in each case what looks like a removal
is a different operation wearing its clothes:

- **`path` has no absence to spell.** `path()` returns `string`, not `?string`: RFC 3986 gives every
  reference a path, and an empty one is `""`.
- **`host` cannot be removed on its own.** A port and a user-info exist only *inside* an authority, so
  clearing the host would have to silently clear two components the caller did not mention. That is a
  cascade, not a removal, and a bag field whose `null` clears a neighbour is precisely the "second
  thing" § 4 forbids.
- **`scheme` is a conversion, not a clearing.** Dropping it turns an absolute URI into a relative
  reference — the inverse of `resolve`, and a member's worth of behaviour rather than a field's.

Removing is otherwise not a shortcut around anything. The result still goes through the same
`recompose` → `read` → `unmoved` path every other `with` takes, so a removal that would let a
remaining component move into another's position is the same throw a bad replacement already is,
naming the component that moved.

`userInfo` is unaffected by all of this and stays off the bag entirely, as it is today: `with` may
neither add nor drop a credential.

**A query parameter is the second level, and it gets a pair of its own.** A URL's components are fixed
and few; its query parameters are dynamic and many, and `with({query: …})` can only replace the whole
string — so editing one parameter is a `parseQuery`, an array write and a `buildQuery` at every call
site that does it, which is where this area's two real bugs come from: concatenating `&k=v` onto a
query that already carries `k`, and forgetting to encode. Two members close it, and
`Core\Uri::with` cannot: a bag's keys are declared in the registry and a parameter name is chosen at
run time.

```
$uri->queryParameter(string $name): mixed
$uri->withQueryParameter(string $name, mixed $value): Uri
```

They add **no mechanism**. Both compose `parseQuery`, `buildQuery` and `with` — all three already
specified, implemented and tested — so a query string gains no second canonicalization for one of them
to drift from, which is the whole of what
[ADR 0146](0146-a-signature-is-over-a-payload-and-a-url-is-a-payload-core-uri.md) found every
framework in this space getting wrong. Four things they settle:

- **A `null` value removes the parameter**, which is § 4's rule and, for this member, is inherited
  rather than added: `buildQuery` already drops a null pair, for the reason § 4 quotes.
- **They are singular and by name, matching `Core\Request::query(string $name)`** — the same operation
  on the same bracket convention, from the other side of the request. `Request` carries `query`,
  `cookie` and `post` that way, so this is the one side of it `Core\Uri` was missing. `query()` is
  already the raw-string reader here, so the `query` prefix is what disambiguates, and R7 spells the
  word out rather than writing `Param`.
- **The bracket convention comes free**, because the value may itself be an `array<mixed>`:
  `withQueryParameter("tag", ["a", "b"])` writes `tag[]=a&tag[]=b`, and `queryParameter("tag")` reads
  that back as the same list.
- **Removing the last parameter leaves no query at all**, not a bare `?`. Each of the three states
  keeps exactly one spelling: a query with pairs is these members, an empty query is
  `with({query: ""})`, and no query is `with({query: null})`.

There is no plural `withQueryParameters` and no `withoutQueryParameter`. Setting several is this
member twice or the `parseQuery`/`buildQuery` pair that already exists, and removing is `null` — a
third and fourth spelling would each be R15's *two behaviours need two names* read backwards.

## Consequences

- **A URL edit stops round-tripping through text.** The recompose-and-re-parse that removal required
  was the one operation `Core\Uri` existed to save a program from writing by hand, and it was also the
  one operation most likely to be written wrong.
- **The bag becomes able to express a partial update**, which is the shape every `with`-style member
  and every PATCH-shaped API needs. Deciding it once here is worth more than the `Core\Uri` case
  alone, and is why this is an ADR rather than a line in a goal file.
- **A new way to be wrong, and it is real:** `$u->with({fragment: $maybe})` where `$maybe` is a
  `?string` computed elsewhere now *removes* when it happens to be null, where today it is a compile
  error. This is the standing trade of merge-patch semantics and it is accepted rather than mitigated
  away. Two things blunt it: the nullability is written in the signature, so the possibility is visible
  at the call site, and § 4's last rule keeps a field non-nullable wherever an accidental removal would
  be dangerous rather than merely surprising.
- **[ADR 0146](0146-a-signature-is-over-a-payload-and-a-url-is-a-payload-core-uri.md)'s `sign` is not
  reopened by this.** `{until: null}` stays the forever spelling and `until` stays a *required* key:
  that reading was chosen because a permanent bearer credential should be something a person typed,
  which is an argument about the member and not about what the bag could express. This ADR removes the
  constraint that also pointed that way; the decision stands on the reason that remains.
- **What it spends**, per `rule:programs/memory-priority`: nothing per request and nothing
  per call. One tag discriminant that already exists, one IR constant, and one more arm in codegen.

## Alternatives rejected

- **`""` means remove.** An in-band sentinel over a value the field legitimately holds — an empty
  query, an empty host on `file:///tmp` — so a program forwarding user input would clear a component by
  accident. This is the bug being removed, spelled differently.
- **A reserved constant, `Core\Uri::REMOVE` or similar.** Still in-band, still a value of the field's
  type, and it adds a name to learn per class. It also cannot be checked: nothing stops it being
  written into a field where removal is meaningless.
- **A second parameter — `{clear: ["fragment", "port"]}`.** Stringly-typed against a fixed key set that
  the shape already spells and the checker already knows, so a typo becomes a runtime throw where the
  bag gives a compile error. It also puts one component's fate in two places when a call both sets and
  clears.
- **A presence bitmask as an extra ABI argument.** Correct, and it would work for a user-declared
  function too, but it changes the calling convention
  [ADR 0135](0135-a-core-shape-parameter-is-one-coretty-carrying-its-arms.md) § 3 deliberately left
  alone, and charges a word to every call with a bag for a distinction a handful of fields want.
  Reusing an existing tag costs nothing and reaches every case this decision is scoped to.
- **Six members — `withoutFragment()`, `withoutPort()`.** Multiplies the surface by the number of
  clearable components on every class that has any, and
  [ADR 0063](0063-core-api-conventions.md) R6 would then want `with`/`without` treated as a symmetric
  pair, which they are not: one takes a structural literal and the other would take names.
- **Make `with` mutate the receiver instead.** Refused by [ADR 0063](0063-core-api-conventions.md) R3
  and R20, and it is not an answer to this question anyway: removal is a *spelling* problem, so an
  in-place `set` with the same bag would be unable to express it for exactly the same reason.

## Verification

- **The two registry tests change what they assert, not how much.**
  `a_nullable_shape_field_omits_as_the_never_written_marker` and
  `a_nullable_option_omits_as_the_never_written_marker` hold § 1's pairing — a field admitting `null`
  defaults to the never-written marker, one that does not defaults to `Const::Null` or a literal —
  over every row in `CLASSES`, so a member declared the wrong way round fails the build rather than
  answering a caller wrongly.
- **A conformance case per state, over `Core\Uri::with`:** `{}` carries every component through,
  `{fragment: null}` removes one, `{query: ""}` sets an empty query and does *not* remove it, and a
  removal that would let a component move is the same throw a bad replacement is.
- **A reject case** holds § 4's teeth: `null` written into a non-nullable option is still the compile
  error it is today, so the nullability in a signature keeps meaning what § 4 says it means.
- **This lands at loop goal 28, stage 2**, whose file names the four places in the tree that restate
  the old invariant and must read as this ADR does when the stage closes.
