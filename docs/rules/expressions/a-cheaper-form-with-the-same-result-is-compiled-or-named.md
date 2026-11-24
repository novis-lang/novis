Where two ways to write the same thing give the same result and one costs much less, the compiler
runs the cheaper one wherever it proves the result is the same, and warns with the fix where only a
possible failure stands in the way. Nobody writes a hint for it: the author writes the clear form,
and the compiler decides.

**One table decides every form.** `crates/nvs-ir/src/forms.rs` holds every row: the shape as written,
the fast form, and what must be proven. The lowering asks it at the four places a shape is still a
tree: a call, a `foreach` subject, a `try` and an array literal. Nothing else decides a form, so there
is one place to read, test and switch off. The list is closed: a goal adds a row, with its
measurement, and a row is built only when its fast form measures 2x or better.

**Three outcomes, and the proof decides which.**

- **Proven: the fast form is compiled, silently.** The result, the order of everything the program
  can observe and every catchable failure are the same. A request that would have ended at the memory
  ceiling in the slow form and does not in the fast one is not a difference that counts: a `FATAL`
  ends the request, and no code observes it.
- **Only a possible failure stands in the way: the form stays as written and warns.** The callback
  writes nothing but has an error edge, and the fast form calls it for fewer entries, so a failure
  for a later entry would not happen. That changes what the program does, so the compiler does not
  choose it. The warning names the fast form and carries it as an `alternative`
  (`rule:ide/a-quick-fix-is-a-diagnostics-own-suggestion`): a person picks it, and no batch applies it.
- **The callback has an effect, or is unknown: nothing.** A callback that writes a property, prints,
  or reaches a callable the compiler cannot name may be called for every entry on purpose. Nobody
  could act on a warning in one edit, so there is none.

**The rows that need no proof** are matched on resolved calls with static types, in the lowering,
which emits the fast form directly:

| As written | Compiled as | Holds when |
|---|---|---|
| `Core\Regex::match(s, p)` used only in `== null`, `!= null` or `is null` | `Core\Regex::matches(s, p)` | no `from` option |
| `Core\Arr::count` of `keys(a)` or `values(a)` | `Core\Arr::count(a)` | always |
| `Core\Arr::first` of `keys(a)`, `first` of `values(a)` | `firstKey(a)`, `first(a)` | always |
| `Core\Arr::filter(a, fn(x) => !Core\Arr::contains(b, x))`, and the same without `!` | `diff(a, b)`, `intersect(a, b)` | `b` is not assigned in the function, and the element type is not an array |
| `try { $v = X as T; } catch (C) { $v = null; }` | `$v = X as ?T;` | below |
| `try { $v = K::parse(X); } catch (C) { $v = null; }` | `$v = K::tryParse(X);` | below, and `K` declares `tryParse` |
| `foreach (Core\Arr::keys(m) as $k)`, `foreach (Core\Arr::values(m) as $v)` | the loop over `m` itself, binding the key or the value | always: a loop walks the array it began with |
| `foreach (Core\Arr::range(a, b, step) as $i)` | a counted loop whose direction is decided once, before the first step | the errors `range` throws for its arguments are thrown before the first step |
| a `foreach` key binding the body never reads | no key is made | always |
| an array literal whose keys are all constants, no two equal, with no spread | one `ArrayNew` with its entries | always |
| `static::m()` and `static::NAME` | the direct call and the constant | no class in the program overrides `m` or declares `NAME` again |
| `$x->m()` on a receiver of static class `C` | the direct call | `m` has exactly one body at or below `C` |
| `$x->m()` where the one body of `m` is `return $this->p;` | the read of `p` | the call is direct, and `p` has no hook |

A caught conversion holds when the `try` body is exactly that one assignment, `X` is a local, a
parameter, a literal or a property with no hook (so the conversion is the body's only failure), there
is one `catch` arm, its class is or contains the class the conversion throws, its variable is absent
or unread, its body is exactly `$v = null;`, and there is no `finally`. Anything wider stays as
written.

The two method rows count only the classes the program declares. `Core\Test::double` and
`Core\Test::partial` make a class while the program runs, so a program that calls either keeps every
instance call virtual and every getter a call (`ExprTypeTable::makes_classes_at_run_time`). Only a
test makes a double, so only a test run loses them. Narrowing it to the doubled interface and its
ancestors was declined: a partial's class stands in for an object of a declared class, and one flag
for the whole program is the one answer that is right without a second walk. The getter's read takes
the reference the body's `return` would have taken, because its consumer treats a call as a fresh
value.

**A native class or interface is never `C`**: a `Core` or extension class, an error class, or a
reserved interface such as `Iterator`. The program's classes are not all of their instances: a
generator's cursor and a `Core` collection's cursor are `Iterator`s, and a helper throws error
objects. So a call through a native static class stays virtual even where the
program declares one body of its own. Without that, a program with one `Iterator` class called that
class's `advance` with a generator as `$this`.

**The rows that need the callback proven** are emitted as written, with a candidate recorded: the
site, the row and the callback's function. A pass after `crates/nvs-ir/src/effects.rs` reads the
callback's summary and takes one of the three outcomes:

| As written | Fast form |
|---|---|
| `Core\Arr::first(Core\Arr::filter(a, p))` | `Core\Arr::find(a, p)` |
| `Core\Arr::firstKey(Core\Arr::filter(a, p))` | `Core\Arr::findKey(a, p)` |
| `Core\Arr::count(Core\Arr::filter(a, p))` compared with `0`, and `Core\Arr::isEmpty` of it | `Core\Arr::any(a, p)`, negated where the test is for none |
| `sort`, `diff` or `intersect` with `comparator: fn(x, y) => E(x) <=> E(y)` | the same member with `by: fn(x) => E(x)` |
| the same comparator with `E(y) <=> E(x)` on `sort` with no `order` option | `by:` with `order: Core\Arr::Order::Desc` |

"Proven" is: the summary writes nothing, is not "anything", and the body has no error edge. A
`FATAL` is not an error edge. For the comparator rows, `E` is the same expression over each parameter
and reads no other parameter.

The pass, `crates/nvs-ir/src/forms.rs` `run`, runs before `crates/nvs-ir/src/ownership.rs`. It
follows every body the callback calls by name, and a body it cannot name is an effect. Printing, a
`Core` member outside `Str`, `Math`, `Arr` and `Regex`, a member that draws a random value, and a
helper call not on its quiet list are effects too, because a summary covers only properties. A
safepoint's error edge is not one: the entry poll's stack check throws on the first call or on none,
since the member calls the callback at one depth, and a time limit is not an outcome a program can
count on. A `count` compared with `0` is lowered as `isEmpty` of the same filter, negated where it
tests for some entry, so the three `any` shapes are one candidate. A proven candidate turns the
filter's instruction into the fast member, where the callback is still alive, and the outer member's
into a relabelling of it or its negation. A candidate that can fail is kept in `Program::forms`, with
its fast form as the program would write it.

A `Core` call in a callback is no error edge when its member is on that quiet list and its reference
card lists no errors (`rule:core-api/reference-card`): every member has a card, and the card names
every error the member throws. Its own edge is the memory ceiling and a stop, which no program
observes.

A comparator row takes a narrow `E`, read from the source text: `E($y)` is `E($x)` with the variable
renamed, both parameters have the same written type, and there is no `by` option. `E` is a static or
`Core` call whose arguments are variables or literals, or a conversion of a variable, and it gives
`int`, `uint` or `string`. For those, `<=>` orders as the member's own comparison does and gives `0`
exactly where the set members find two values the same. A `float` stays as written, because `NaN` and
`-0.0` compare differently in the two. The lowering lowers `fn($x) => E($x)` as a function of its own
class, with the comparator's captures as its fields. A proven candidate makes the comparator's object
an object of that class and passes it as `by`, and the descending row turns the default `order` into
`Desc`. So the call makes the same one object in both forms. **What it spends:** while compiling, one
more function for each comparator row. At run time, nothing.

**Two warnings, both raised by the compiler once.** A form the compiler could not choose is `W1034`,
raised by `nvs_ir::forms::warnings` from the table's pass's own decision, never from a second
analysis, so the editor and `nvs check` cannot disagree. Its span is the written form, its message
names the member to use, and its one fix is the fast form's text as an `alternative`. The same
value read twice is `W1035`, found on the written tree by `nvs_ir::rereads::find` and raised by its
`warnings`, and a `Core` call between the two that takes a callable stops it as a Novis call does. The same value read twice warns where two calls of one reading member in one
function do the same work again: `get` on a cache store of one key,
two or more `queryParameter` calls on one `Core\Uri`, and two or more of `Core\IO::isFile`, `isDirectory`,
`size` and `modifiedAt` on one path. It carries no fix, because the fix needs a variable name. The
shape is exact: the same receiver variable and the same literal key or path variable, neither assigned
between the calls, the second reached only through the first, and between them no call to a Novis
function, no callable call and no member of the same class that writes.

**What is not a warning.**

- **A cost that depends on how often a line runs** is shown from a recorded request, never guessed
  while compiling: a query, an HTTP request, a cache read, a session read or a file open inside a
  loop. The fast form of each behaves differently on a failure or when another process changes the
  file, so no fix is exact. `Core\IO::append` in a loop opens the file on every call, and one opened
  handle writes to the old file if the file is replaced. The recorded request names the line of each
  such call, as `rule:observability/a-query-span-names-its-call-site` does for a query.
- **A type that could be narrower** is an editor action with no diagnostic behind it
  (`rule:ide/a-code-action-writes-only-what-is-already-determined`): every deliberate `mixed` would
  otherwise need suppressing.
- **A closure that captured an array changed later** copies once, and moving the change above the
  closure changes what the closure sees. It is a row of the copy table
  (`rule:types/a-replaced-value-is-changed-in-place`) and nothing more.

**A proven callback skips the check of each call.** Where the checker accepts a callback argument
against a `Core` member's declared callback signature, and the subject's static element type is what
the callback's parameter declares or narrower, the callable is passed marked proven and its call
skips the parameter checks. The proof travels with the argument, never with the context, so a member
call nested inside the callback cannot inherit it. A subject typed `mixed`, `array<mixed>` or a union
keeps the check. A debug build checks a proven callback too and aborts on a mismatch, so the tests and
the fuzzer find a wrong proof before a release build can.

**The off switch.** A hidden `--no-cheaper-forms` on `nvs run`, `nvs test` and `nvs serve` compiles
every form as written, and `bun nv suite … --no-cheaper-forms-too` runs each case both ways and fails
on any difference in output. It is a test tool, not a setting: no `nvs.toml` key, no help text. A
program lowered with it is neither written to nor read from the artifact cache. Every row owes cases
under `tests/conformance/forms/`, which `every_form_row_has_its_cases` counts: a row with no proof
one where it holds and one for each clause that fails, a callback row one each for proven, can fail
and has an effect.

**What it spends.** While compiling: one table lookup for each call, `foreach`, `try` and literal
lowered, and one candidate record for each callback form. At run time nothing: every row removes
work. A request or a task holds no more memory. With recording on, one site id for each recorded HTTP,
cache, session and file call; with it off, the flag test a query site already makes.

**Declined:**

- **`Core\Str::indexOf(…) != null` as `Core\Str::contains`, `Core\Str::length(…) == 0` as
  `Core\Str::isEmpty`, and `Core\Html::escape` inside a template hole as the plain hole.** Measured:
  1 ms against 1 ms for 500 calls on 42 KB, 1 ms against 1 ms for 20,000 strings, and 218 ms against
  151 ms for 1M holes (1.4x). None reaches 2x.
- **A `W1035` row for `Core\Session::get`.** Measured under `nvs serve` with a 16 KiB record and a
  Redis store: a request with two reads of one key took 946 µs against 945 µs with one (1.0x), and
  the same as with none. Decoding the record is lost beside the request and the store's round trip.
- **A warning for a caught conversion.** Every case it would name is one the compiler proves, so it
  is compiled silently.
- **A warning for a by-value loop that writes the entry it is on.** It is a silent change of
  `rule:types/a-replaced-value-is-changed-in-place`.
- **A warning for `Core\IO::append` in a loop.** One opened handle behaves differently when the file
  is replaced, so it is shown from a recorded request.
- **A hint in the editor alone.** `nvs check` and CI would never see it, and the language server would
  carry a second copy of a check the compiler makes once (`rule:expressions/misread-grouping-warns`).
- **Applying a warned form's fix on save.** It changes what the program does when the callback
  fails, so a person picks it.
