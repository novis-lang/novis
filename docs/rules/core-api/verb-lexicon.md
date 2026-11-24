A member's name says whether it reads, changes or copies, and its first word predicts its return type. The
rule holds for `Core` and for every first-party package (`extensions/*/nvs/`, `packages/rest/`), in four
parts:

1. **A member with an effect starts with a verb.** An effect is anything beyond returning a value: it
   changes its own object, sends or writes output, changes the request, the response, the session, a
   file, the network or the clock, or prompts the user. `add…` adds to a collection, `set…` replaces one
   value, and `send…`, `write…`, `start…`, `open…`, `run…`, `show…`, `ask…` and `create…` say what they
   do. `on…` registers a handler.
2. **A noun or an adjective names what a member returns, and such a member has no effect.** It reads a
   property (`Request::path()`), reads by key (`$row->int('id')`), computes a value (`Str::length($s)`,
   `$a->gcd($b)`), or builds a new value (`Duration::seconds(5)`, `Zone::of(…)`).
3. **An instance member that returns a changed copy says it is a copy.** It is named `with…`,
   `without…`, `plus`, `minus`, `keeping…`, or a past participle (`negated`, `multipliedBy`, `resized`,
   `signed`). The plain imperative (`add`, `resize`, `sign`) is kept for a change in place. So
   `$set->add($x)` changes the set, and `$image->blurred(2)` returns a new image, and a call whose result
   is thrown away reads wrong at a glance. A static member is not held to this: it takes its subject as
   an argument, and `rule:core-api/nothing-mutates` already says it changes nothing, so `Str::trim($s)`
   keeps its name.
4. **A `bool` read starts with a predicate:** `is…`, `has…`, `can…`, `must…`, `needs…`, `allows…`,
   `uses…`, `contains`, `startsWith`, `endsWith`, `equals`, `matches`, `exists`, `any` or `all`. A member
   with an effect that also returns `bool` keeps its effect verb (`Config::set`, `Queue::cancel`), and so
   does a check that throws when it fails (`verify…`, `assert…`).

The rest of the lexicon fixes what a verb returns: `find…` returns `?T`; `indexOf` and `keyOf` return
`?uint` or `?K`; `count…` returns `uint`; `to…` and `from…` convert and construct. Two constructor forms
join them: the unit or component a value is built from (`Duration::seconds`, `TimeOfDay::at`), and `of`
for a canonical identifier (`Zone::of`, `Hash::of`).

**One word keeps one meaning across `Core`.** A word used as a verb with an effect is not also a noun read
in another class. `Db\Connection->query` runs a statement and `Http\Client::post` sends a request, so the
request's parameters are read with `Request::queryParameter` and `Request::formField`, not `query` and
`post`.

`…OrNull`, `…Safe` and `…Ex` are banned, and so is `try…` on every verb but one. The exception is
`tryParse` (`rule:expressions/try-parse`), for a class whose `parse` takes exactly one `string` and can
fail: a malformed string is a *failure*, not the absence `?T` means, and `as ?T` never targets a class, so
without it `Core\Uri` and `Core\Uuid` would have no non-throwing form at all. The `…Safe` half of the ban
reads the suffix as a claim about failure; a suffix that is an ordinary adjective of the subject is
untouched, which is how `Core\Db\Schema::applySafe` is admitted — it is named for the grade of the steps it
will run and is the more refusing of the two, not the quiet one.

**Who checks it.** The ban list is `crates/nvs-stdlib/src/registry.rs`'s. `bun nv names --check`
(`tools/nv/cmd/names.ts`) judges the four parts from each member's return type, with its word lists in
`tools/data/api-names.json`:

- a member returning `void` starts with an effect verb (part 1);
- a member returning `bool` starts with a predicate or an effect verb (part 4);
- an instance member returning its own class is in a class listed as `value`, and is named as a copy, or
  in a class listed as `changing`, and starts with an effect verb (part 3). A class on neither list
  fails, so the author of every new builder classifies it once;
- a member that takes an argument besides its subject and starts with no verb is on the `reads` list
  (part 2). This closes the one shape the types cannot show: a noun with an effect that returns another
  type, such as a `Workbook->sheet($name)` that adds a sheet. The list is every such member that
  exists, and a new one fails until its author adds it.

A past participle is a first word ending in `ed` (outside `notParticiples`), or one of
`irregularParticiples`; a participle ending in `en` is only ever listed, since `flatten` and `open` end so
too. `bun nv ci --check` runs the command after the build, so CI and `bun nv close` run it.

What it costs: chains get a few characters longer, and every call says whether it reads, changes or
copies. Nothing on the request path, since a name is resolved at compile time.

**Declined:**
- **Holding parameter names to it** (`$a`, `$s`, `$fn`). A reader meets them at the declaration and in
  the reference, not at the call site this rule is about. `spawn`'s `arguments:` is the exception, since
  it is the same value as `Script::arguments()`.
- **A per-member effect flag in the registry.** Most of the rule follows from the types, and the `reads`
  list fails on every new member of the one shape the types cannot judge.
- **Copy names for statics** (`Str::trimmed`). A static changes nothing in place, so there is nothing to
  confuse it with.
- **A naming check on a user's own program.** The rule is for `Core` and the first-party packages. A
  program names its own methods.
