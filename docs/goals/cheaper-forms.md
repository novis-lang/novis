---
looper: 1
title: "the compiler runs the cheaper of two forms that give the same result, and names the ones only an edit can change"
summary: "Reached when every row of one table of cheaper forms is compiled as its fast form wherever the compiler proves the result is the same, is a warning with its fix wherever only a possible failure stands in the way, has a case that prints the same with the table switched off, and a recorded request shows its HTTP, cache, session and file calls by line."
context: [docs/rules/types/a-replaced-value-is-changed-in-place.md, docs/rules/ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows.md, docs/rules/expressions/misread-grouping-warns.md, crates/nvs-ir/src/lib.rs]
stages:
  - id: 1
    title: the rule, the table, the off switch and a case for every row
    status: done
    context: [crates/nvs-cli/src/main.rs, tools/nv/cmd/suite.ts, benches/scaling/README.md, benches/scaling/arrays/sort.nvs]
  - id: 2
    title: a call form that needs no proof compiles as its fast form
    status: open
    context: [crates/nvs-ir/src/lower/expr.rs, crates/nvs-ir/src/lower/exception.rs, crates/nvs-ir/src/lower/tests.rs]
  - id: 3
    title: a loop, a literal and a static call take the direct path
    status: open
    context: [crates/nvs-ir/src/lower/control.rs, crates/nvs-ir/src/lower/expr.rs, crates/nvs-ir/src/lower/tests.rs]
  - id: 4
    title: a form with a callback compiles as its fast form where the callback is proven, and is recorded where it is not
    status: open
    context: [crates/nvs-ir/src/lower/expr.rs, crates/nvs-ir/src/lower/tests.rs, crates/nvs-stdlib/src/arr.rs]
  - id: 5
    title: a form only an edit can change is a warning with its fix, in nvs check and in the editor
    status: open
    context: [crates/nvs-diagnostics/src/code/warnings.rs, crates/nvs-lsp/src/actions.rs]
  - id: 6
    title: a callback the checker proved is called without the check of each call
    status: open
    context: [crates/nvs-runtime/src/callable.rs, crates/nvs-runtime/src/dispatch.rs, crates/nvs-ir/src/ir.rs]
  - id: 7
    title: a recorded request shows its HTTP, cache, session and file calls by line
    status: open
    context: [crates/nvs-lsp/src/recorded.rs, crates/nvs-cli/src/debug/index.rs, docs/rules/observability/a-query-span-names-its-call-site.md]
  - id: 8
    title: a parameter declared callable or mixed takes the type every caller passes, as an editor action
    status: open
    context: [crates/nvs-lsp/src/actions.rs, docs/rules/ide/narrow-an-annotation-to-its-literal.md]
  - id: 9
    title: a method call the compiler can name is direct, and a method that returns one property reads it
    status: open
    context: [crates/nvs-ir/src/lower/expr.rs, crates/nvs-types/src/signatures.rs, crates/nvs-ir/src/lower/tests.rs]
  - id: 10
    title: a split with a limit, a search that ignores case and a JSON validity test read only what their result needs
    status: open
    context: [crates/nvs-stdlib/src/str.rs, crates/nvs-stdlib/src/json.rs]
  - id: 11
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: table
    stage: 1
    name: "every row of the table has a conformance case for each outcome it can have"
    run: [cargo, test, --lib, --, every_form_row_has_its_cases]
    timeout: 1800
    tests:
      - every_form_row_has_its_cases
  - id: call-forms
    stage: 2
    name: "a null test of a match, a count or first of keys or values, a difference written as filter and contains, and a caught conversion that assigns null each compile as their fast form"
    run: [cargo, test, --lib, --, a_null_test_of_a_match_compiles_as_matches, a_count_or_first_of_keys_or_values_reads_the_array_itself, a_difference_written_as_filter_and_contains_compiles_as_diff, a_caught_conversion_that_assigns_null_compiles_as_the_nullable_conversion]
    timeout: 1800
    tests:
      - a_null_test_of_a_match_compiles_as_matches
      - a_count_or_first_of_keys_or_values_reads_the_array_itself
      - a_difference_written_as_filter_and_contains_compiles_as_diff
      - a_caught_conversion_that_assigns_null_compiles_as_the_nullable_conversion
  - id: direct-paths
    stage: 3
    name: "a loop over keys, values or a range walks without the array, a key nobody reads is not made, a literal with constant keys is built in one step, and a static call nothing overrides is direct"
    run: [cargo, test, --lib, --, a_loop_over_keys_or_values_walks_the_array_itself, a_loop_over_a_range_counts_without_an_array, a_key_nobody_reads_is_not_made, a_literal_with_constant_keys_is_built_in_one_step, a_static_call_nothing_overrides_is_direct]
    timeout: 1800
    tests:
      - a_loop_over_keys_or_values_walks_the_array_itself
      - a_loop_over_a_range_counts_without_an_array
      - a_key_nobody_reads_is_not_made
      - a_literal_with_constant_keys_is_built_in_one_step
      - a_static_call_nothing_overrides_is_direct
  - id: callback-forms
    stage: 4
    name: "a callback form compiles as its fast member where the callback is proven, stays as written and is recorded where the callback can fail, and stays as written and silent where it has an effect"
    run: [cargo, test, --lib, --, a_proven_callback_form_compiles_as_its_fast_member, a_callback_that_can_fail_keeps_the_form_and_is_recorded, a_callback_with_an_effect_keeps_the_form_and_is_silent, a_comparator_of_one_expression_sorts_by_it]
    timeout: 1800
    tests:
      - a_proven_callback_form_compiles_as_its_fast_member
      - a_callback_that_can_fail_keeps_the_form_and_is_recorded
      - a_callback_with_an_effect_keeps_the_form_and_is_silent
      - a_comparator_of_one_expression_sorts_by_it
  - id: differential
    stage: 4
    name: "every conformance case prints the same with the table switched off"
    run: [bun, nv, suite, tests/conformance/, --no-cheaper-forms-too]
    timeout: 3600
  - id: ladders
    stage: 4
    name: "every cheaper-form ladder grows as its file says"
    run: [bun, nv, scaling, --check, arrays/forms-]
    timeout: 3600
  - id: warnings
    stage: 5
    name: "a recorded form is a warning that carries its fast form as an alternative, and a second read of the same value warns only in straight code"
    run: [cargo, test, --lib, --, a_form_warning_carries_its_fast_form_as_an_alternative, a_second_read_of_the_same_value_warns_only_in_straight_code]
    timeout: 1800
    tests:
      - a_form_warning_carries_its_fast_form_as_an_alternative
      - a_second_read_of_the_same_value_warns_only_in_straight_code
  - id: warnings-editor
    stage: 5
    name: "the editor warns at a first of a filter and offers find, and warns at a second read of one session value"
    run: [bun, nv, suite, lsp-test, tests/lsp/, --case, tests/lsp/diagnostics/a-first-of-a-filter-warns-and-offers-find.lspt, --case, tests/lsp/diagnostics/a-second-read-of-one-session-value-warns.lspt]
    timeout: 1800
    tests:
      - tests/lsp/diagnostics/a-first-of-a-filter-warns-and-offers-find.lspt
      - tests/lsp/diagnostics/a-second-read-of-one-session-value-warns.lspt
  - id: proven-callbacks
    stage: 6
    name: "a callback the checker proved runs without the check of each call, a callback over a mixed subject keeps it, and a debug build still checks and traps a mismatch"
    run: [cargo, test, --lib, --, a_proven_callback_runs_without_the_check_of_each_call, a_callback_over_a_mixed_subject_keeps_the_check, a_debug_build_checks_a_proven_callback_and_traps_a_mismatch]
    timeout: 1800
    tests:
      - a_proven_callback_runs_without_the_check_of_each_call
      - a_callback_over_a_mixed_subject_keeps_the_check
      - a_debug_build_checks_a_proven_callback_and_traps_a_mismatch
  - id: recorded-sites
    stage: 7
    name: "an HTTP, cache, session and file call of a recorded request names its line, and the editor shows the count there"
    run: [bun, nv, suite, lsp-test, tests/lsp/, --case, tests/lsp/lens/a-recorded-http-call-shows-its-count-on-the-line.lspt, --case, tests/lsp/lens/a-recorded-file-open-in-a-loop-shows-its-count-on-the-line.lspt]
    timeout: 1800
    tests:
      - tests/lsp/lens/a-recorded-http-call-shows-its-count-on-the-line.lspt
      - tests/lsp/lens/a-recorded-file-open-in-a-loop-shows-its-count-on-the-line.lspt
  - id: typed-parameter-action
    stage: 8
    name: "a bare callable or mixed parameter is offered the type every caller passes, and is offered nothing where callers differ"
    run: [bun, nv, suite, lsp-test, tests/lsp/, --case, tests/lsp/actions/a-bare-callable-parameter-takes-the-type-every-caller-passes.lspt, --case, tests/lsp/actions/a-parameter-whose-callers-differ-is-offered-no-type.lspt]
    timeout: 1800
    tests:
      - tests/lsp/actions/a-bare-callable-parameter-takes-the-type-every-caller-passes.lspt
      - tests/lsp/actions/a-parameter-whose-callers-differ-is-offered-no-type.lspt
  - id: named-calls
    stage: 9
    name: "a method nothing at or below the receiver's class overrides is called directly, a method one class at or below it overrides stays virtual, and a method that returns one property is the property read"
    run: [cargo, test, --lib, --, a_method_nothing_below_the_receiver_overrides_is_direct, a_method_a_class_below_the_receiver_overrides_stays_virtual, a_method_that_returns_one_property_is_the_property_read]
    timeout: 1800
    tests:
      - a_method_nothing_below_the_receiver_overrides_is_direct
      - a_method_a_class_below_the_receiver_overrides_stays_virtual
      - a_method_that_returns_one_property_is_the_property_read
  - id: members
    stage: 10
    name: "a split with a limit visits no piece past its limit, a search that ignores case builds no table for plain text, and a validity test builds no document"
    run: [cargo, test, --lib, --, a_split_with_a_limit_visits_no_piece_past_its_limit, a_search_that_ignores_case_builds_no_table_for_plain_text, a_validity_test_builds_no_document]
    timeout: 1800
    tests:
      - a_split_with_a_limit_visits_no_piece_past_its_limit
      - a_search_that_ignores_case_builds_no_table_for_plain_text
      - a_validity_test_builds_no_document
  - id: member-ladder
    stage: 10
    name: "splitting a key off a long line does not grow with the line"
    run: [bun, nv, scaling, --check, strings/forms-]
    timeout: 3600
---

# the compiler runs the cheaper of two forms that give the same result, and names the ones only an edit can change

## Why

Two ways to write the same thing often cost very different amounts, and nothing tells the author.
The person's requirement, from the review of 2026-11-17: where the compiler can prove the result is
the same, it compiles the cheaper form and nobody has to know. Where only the author's edit can make
the change, the compiler says so at the line, with the fix.

Measured on 2026-11-17 with the release binary, single runs:

| As written | Cost | Same result | Cost |
|---|---|---|---|
| `Core\Arr::filter($a, fn($x) => !Core\Arr::contains($b, $x))`, 4k and 8k entries | 45 ms, 238 ms | `Core\Arr::diff($a, $b)` | 0 ms, 3 ms |
| `Core\Arr::first(Core\Arr::filter($a, $p))`, 200 calls on 20k entries | 157 ms | `Core\Arr::find($a, $p)` | 0 ms |
| `sort` with `comparator: fn($a, $b) => $a <=> $b`, 200k entries | 210 ms | `by: fn($x) => $x` | 16 ms |
| `try { $n = $s as int; } catch (Throwable $e) { $n = null; }`, 200k failures | 90 ms | `$n = $s as ?int` | 2 ms |
| `Core\Regex::match($s, $p) != null`, 200k calls | 206 ms | `Core\Regex::matches($s, $p)` | 15 ms |
| `foreach (Core\Arr::keys($m) as $k)`, 500k entries | 110 ms | `foreach ($m as $k => $v)` | 14 ms |
| `foreach (Core\Arr::range(0, $n) as $i)`, 2M steps | 36 ms | `for` | 2 ms |
| `foreach ($a as $k => $v)` where `$k` is never read, 2M entries | 47 ms | `foreach ($a as $v)` | 10 ms |
| `static::tag()`, 5M calls, nothing overrides `tag` | 88 ms | `self::tag()` | 23 ms |
| a literal with four constant keys, 1M times | 279 ms | a list literal of four | 130 ms |
| a call through a bare `callable` with a 1000-entry `array<int>` argument, 100k calls | 359 ms | `callable(array<int>): int` | 3 ms |
| `mixed $x` in `$x + 1`, 5M times | 146 ms | `int $x` | 11 ms |
| `Core\Arr::count(Core\Arr::keys($m))`, 200 calls on 10k entries | 38 ms | `Core\Arr::count($m)` | one step |
| `$dog->name()` where a sibling class overrides `name`, 5M calls | 129 ms | the same call where nothing overrides | 26 ms |
| `$dog->legs()` whose body is `return $this->legs;`, 5M calls | 34 ms | `$dog->legs` | 10 ms |
| `Core\Str::split($line, '=', { limit: 2 })`, 2000 calls on a 42 KB line, and on 84 KB | 42 ms, 80 ms | `Core\Str::before($line, '=')` | 0 ms |
| `Core\Str::indexOf($s, $n, { caseInsensitive: true })`, 500 calls on 42 KB | 124 ms | the same search without the option | 1 ms |
| `Core\Json::isValid($doc)`, 50 calls on 109 KB | 62 ms | `Core\Json::decode($doc)`, which also builds the value | 68 ms |
| three `queryParameter` calls on one `Core\Uri` with a 1.5 KB query, 20,000 times | 840 ms | one `Core\Uri::parseQuery` | 253 ms |
| two `get` calls for one cache key holding 8000 entries, 2000 times | 830 ms | one `get` | 291 ms |
| `Core\IO::isFile`, `size` and `modifiedAt` on one path, 20,000 times | 4.5 s | one `Core\IO::stat` | 1.6 s |

Three forms the review named were measured the same day and showed no gain worth a row:
`Core\Str::indexOf(…) != null` against `Core\Str::contains` (1 ms each for 500 calls on 42 KB),
`Core\Str::length(…) == 0` against `Core\Str::isEmpty` (1 ms each for 20,000 strings of 70 characters),
and `Core\Html::escape` written inside a template hole against the plain hole (218 ms against 151 ms
for 1M holes, which is 1.4x). They are not in the table.

`Core\Arr::map($rows, fn(array<int> $row): int => 1)` over 100 rows, 1000 times, took 536 ms with
1000-entry rows and 916 ms with 2000-entry rows. The callback never reads the row. The time is
`check_param_types` (`crates/nvs-runtime/src/dispatch.rs:286`) testing every element of the row on every
call, because every callback a `Core` member reaches is called as a bare `callable`
(`crates/nvs-ir/src/ir.rs:3468`).

The causes, each read in the code:

- **Nothing in the lowering looks at how two calls compose.** Every `Core` member call is one
  `CoreCall` (`crates/nvs-ir/src/lower/expr.rs:5160`). `filter` calls its function for every entry and
  builds an array (`crates/nvs-stdlib/src/arr.rs:2421`), and `find_slot` stops at the first match
  (`arr.rs:5233`). `contains` walks the array (`arr.rs:6448`), and `diff` without a comparator is one
  hash pass (`arr.rs:5959`). A comparator always takes the merge sort with one call per comparison
  (`arr.rs:4717`).
- **A `foreach` key is made whether or not the body reads it** (`crates/nvs-ir/src/lower/control.rs:1080`),
  and a list makes one decimal string for each.
- **A literal with any key is built with one write per element** (`expr.rs:6904`), where a list literal
  is one `ArrayNew` (`expr.rs:6884`).
- **`static::` always dispatches at run time**, where an instance call already asks whether anything
  overrides the method (`crates/nvs-types/src/signatures.rs:1125`).

Our own examples are written the slow way:
`docs/examples/tools/packages/the-registry-website/01-the-default-order.nvs:65` sorts with a comparator
of one expression, and several examples loop over `Core\Arr::range`.

## Standing decisions

**One table, in one file (my call).** `crates/nvs-ir/src/forms.rs` holds every row: the shape as
written, the fast form, and what must be proven. The lowering asks the table at the four places a
shape is still a tree: a call (`crates/nvs-ir/src/lower/expr.rs:5160`), a `foreach` subject
(`crates/nvs-ir/src/lower/control.rs:880`), a `try` (`crates/nvs-ir/src/lower/exception.rs:382`) and an
array literal (`expr.rs:6884`). Nothing else decides a form, so there is one place to read, test and
switch off. The list is closed: a row is added by a goal, with its measurement.

**Three outcomes, and the proof decides which (the person's call for the first two, mine for the
third).**

- **Proven: the fast form is compiled, silently.** The result, the order of what the program can
  observe and every catchable failure are the same. A request that would have ended at the memory
  ceiling in the slow form and does not in the fast form is not a difference that counts: a `FATAL`
  ends the request and no code observes it.
- **Only a possible failure stands in the way: the form stays as written and is a warning.** The
  callback writes nothing, but it has an error edge. The fast form calls it for fewer entries, so a
  failure for a later entry would not happen. That changes what the program does, so the compiler
  does not choose it. The warning names the fast form and carries it as an `alternative`
  (`rule:ide/a-quick-fix-is-a-diagnostics-own-suggestion`): a person picks it, and no batch applies it.
- **The callback has an effect, or is unknown: nothing.** A callback that writes a property, prints,
  or reaches a callable the compiler cannot name may be called for every entry on purpose. No
  warning, because nobody could act on it in one edit.

**The rows that need no proof (my call).** Each is matched on resolved calls with static types, in the
lowering, and emits the fast form directly:

| As written | Compiled as | Holds when |
|---|---|---|
| `Core\Regex::match(s, p)` used only in `== null`, `!= null` or `is null` | `Core\Regex::matches(s, p)` | no `from` option |
| `Core\Arr::count` of `keys(a)` or `values(a)` | `Core\Arr::count(a)` | always |
| `Core\Arr::first` of `keys(a)`, `first` of `values(a)` | `firstKey(a)`, `first(a)` | always |
| `Core\Arr::filter(a, fn(x) => !Core\Arr::contains(b, x))` and the same without `!` | `diff(a, b)`, `intersect(a, b)` | `b` is not assigned in the function, and the element type is not an array |
| `try { $v = X as T; } catch (C) { $v = null; }` | `$v = X as ?T;` | below |
| `try { $v = K::parse(X); } catch (C) { $v = null; }` | `$v = K::tryParse(X);` | below, and `K` declares `tryParse` |
| `foreach (Core\Arr::keys(m) as $k)`, `foreach (Core\Arr::values(m) as $v)` | the loop over `m` itself, binding the key or the value | always: a loop walks the array it began with |
| `foreach (Core\Arr::range(a, b, step) as $i)` | a counted loop whose direction is decided once, before the first step | the errors `range` throws for its arguments are thrown before the first step |
| a `foreach` key binding the body never reads | no key is made | always |
| an array literal whose keys are all constants, no two equal, with no spread | one `ArrayNew` with its entries | always |
| `static::m()` and `static::NAME` | the direct call and the constant | no class in the program overrides `m` or declares `NAME` again |
| `$x->m()` on a receiver of static class `C` | the direct call | no class at or below `C` declares `m` again, or `m` has exactly one body at or below `C` |
| `$x->m()` where the one body of `m` is `return $this->p;` | the read of `p` | the call is direct, and `p` has no hook |

The caught conversion holds when the `try` body is exactly that one assignment, `X` is a local, a
parameter, a literal or a property with no hook (so the conversion is the body's only failure), there
is one `catch` arm, its class is or contains the class the conversion throws, its variable is absent or
unread, its body is exactly `$v = null;`, and there is no `finally`. Anything wider stays as written.

**The rows that need the callback proven (my call).** The lowering emits the form as written and
records a candidate: the site, the row and the callback's function. `forms.rs` runs after
`crates/nvs-ir/src/effects.rs`, which `in-place-everywhere` builds, and reads the callback's summary:

| As written | Fast form |
|---|---|
| `Core\Arr::first(Core\Arr::filter(a, p))` | `Core\Arr::find(a, p)` |
| `Core\Arr::firstKey(Core\Arr::filter(a, p))` | `Core\Arr::findKey(a, p)` |
| `Core\Arr::count(Core\Arr::filter(a, p))` compared with `0`, and `Core\Arr::isEmpty` of it | `Core\Arr::any(a, p)`, negated where the test is for none |
| `sort`, `diff` or `intersect` with `comparator: fn(x, y) => E(x) <=> E(y)` | the same member with `by: fn(x) => E(x)` |
| the same comparator with `E(y) <=> E(x)` on `sort` with no `order` option | `by:` with `order: Core\Arr::Order::Desc` |

"Proven" is: the summary writes nothing, is not "anything", and the body has no error edge. A `FATAL`
is not an error edge. For the comparator rows, `E` is the same expression over each parameter and reads
no other parameter.

**The warnings (the person's call that they exist, my call for their shape).** Each takes the next
free `W1xxx` number when it lands, with a card in end-user prose.

- **A form the compiler could not choose.** Raised by `forms.rs` from its own decision, never from a
  second analysis, so the editor and the compiler cannot disagree. It reaches `nvs check` and the
  language server by the path `in-place-everywhere` stage 7 builds. A hint in the editor alone stays
  declined (`rule:expressions/misread-grouping-warns`).
- **The same value read twice.** Two calls of one reading member in one function, where the second
  one does the first one's work again. It carries no fix, because the fix needs a variable name. The
  rows are `Core\Session::get` (`crates/nvs-stdlib/src/session.rs:1357`, which decodes the record on
  every call), `get` on a cache store (which copies the whole value on every call,
  `rule:concurrency/a-cached-value-is-copied-across-the-boundary`), two or more
  `queryParameter` calls on one `Core\Uri` (`crates/nvs-stdlib/src/uri.rs:2844`, which parses the whole
  query on every call, where `Core\Uri::parseQuery` parses once), and two or more of `Core\IO::isFile`,
  `isDir`, `size` and `modifiedAt` on one path where `Core\IO::stat` (`crates/nvs-stdlib/src/io.rs:3799`)
  asks once. The shape is exact: the same receiver variable and the same literal key or path variable,
  neither assigned between the two calls, the second call reached only through the first, and between
  them no call to a Novis function, no callable call and no member of the same class that writes. The
  cache, `queryParameter` and file rows are measured in § *Why*: each read costs enough that the
  second one doubles it or worse. The session row cannot be measured outside a request, so stage 5
  measures it and builds it only at 2x or more, as § *Limits* says.

**Three members read only what their result needs (my call).** These are fixes inside the member,
with no row in the table, because nothing about the call changes:

- `Core\Str::split` with a positive `limit` stops counting pieces at the limit
  (`crates/nvs-stdlib/src/str.rs:2176`). Today it counts every piece of the subject first.
- A search that ignores case (`indexOf`, `lastIndexOf`, `contains` with `caseInsensitive`) folds bytes
  as it scans when the needle and the subject are both plain ASCII, and builds its tables
  (`str.rs:2831`) only otherwise. A letter outside ASCII can fold to an ASCII one, so the fast path
  needs both sides plain.
- `Core\Json::isValid` runs the reader without building the value, which its module doc already
  names as not built (`crates/nvs-stdlib/src/json.rs:141`). Its gain is not known until it exists, so
  it is kept only if it measures 2x or better against today's 62 ms.

**What is not a warning, and why (my call).**

- **A cost that depends on how often a line runs** is shown from a recorded request, never guessed
  while compiling: a query, an HTTP request, a cache read or a file open inside a loop. The fast form
  of each also behaves differently on a failure or when another process changes the file, so no fix is
  exact. `Core\IO::append` in a loop is one of these: it opens and closes the file on every call
  (`io.rs:3431`), and one opened handle writes to the old file if the file is replaced.
- **A type that could be narrower** is an editor action with no diagnostic behind it, as
  `rule:ide/narrow-an-annotation-to-its-literal` decided for `array<mixed>`: every deliberate `mixed`
  would otherwise need suppressing.
- **A closure that captured an array which is changed later** copies once, and moving the change above
  the closure changes what the closure sees. Two values really are used. It is a row of
  `in-place-everywhere`'s copy table and nothing more.

**A proven callback skips the check of each call (my call).** Where the checker accepts a callback
argument against a `Core` member's declared callback signature, and the subject's static element type
is what the callback's parameter declares or narrower, the lowering passes the callable marked proven
and `call_callable` (`crates/nvs-runtime/src/callable.rs:193`) skips `check_param_tags` and
`check_param_types` (`callable.rs:253`). The proof travels with the argument, never with the context,
so a member call nested inside the callback cannot inherit it. The default is a flag beside the
callable's tag in its argument slot. **Security and request isolation:** the check is what stops a
wrongly typed value from reaching compiled code that reads it unchecked. It is skipped only on the
proof `CallCallableProven` already rests on (`crates/nvs-ir/src/ir.rs:3471`), so no new trust is added.
A subject whose static type is `mixed`, `array<mixed>` or a union keeps the check. A debug build runs
the check on a proven callback too and aborts on a mismatch, so the tests and the fuzzer find a wrong
proof before a release build can.

**A recorded request names more sites (my call).** The debug index gives an `http` event, a cache
read or write, a session read or write and a file open the site its call was written at, the way a
`query` event has one (`rule:observability/a-query-span-names-its-call-site`). The lens that shows a
query site's count, time and repeats (`crates/nvs-lsp/src/recorded.rs:40`) shows the same three figures
for each, and "repeated" counts the most frequent host and path, key or file path. **What it spends:**
with recording off, nothing beyond the flag test a query site already makes. With recording on, one
site id for each recorded call.

**The editor action (my call).** On a parameter declared bare `callable` or `mixed`, *Write the type
every caller passes* is offered when the reference index finds at least one call of the function and
every call passes an argument of one static type. It writes that type and nothing else, under
`refactor.rewrite`, never under `quickfix` or `source.fixAll.nvs`. Where callers differ, or one passes
`mixed`, nothing is offered. Home: `rule:ide/a-code-action-writes-only-what-is-already-determined`.

**The off switch (my call).** A hidden `--no-cheaper-forms` on `run`, `test` and `serve` compiles every
form as written. `bun nv suite … --no-cheaper-forms-too` runs each case both ways and fails on any
difference in output. It is a test tool, not a setting: no `nvs.toml` key, no help text.

**What the goal spends:** while compiling, one table lookup for each call, `foreach`, `try` and
literal lowered, and one candidate record for each callback form. At run time, nothing: every row
removes work. A program holds no more memory for a request or a task.

**A `run` ladder measures its work apart from its setup (my call, stage 1).** `bun nv scaling`
judges a `run` ladder by the whole run's counts, so the two `expect constant` ladders count the linear
input they build, and would stay red after their fix. Stage 4 gives a `run` ladder
`// scaling: rounds N` in `tools/nv/cmd/scaling.ts`: the tool runs the work half and all of `N`
times and judges the difference, as `rampPerOp` already does for a member bench's `.scale.nvs`, so
setup cancels out. The two constant ladders then take the line. Moving them to member benches was
declined: the size a ladder grows is what they test. Marking them `proposal` was declined: it would
hide the red the fix must turn green. The difference ladder's quadratic shows in the clock alone,
because `Core\Arr::contains` is no counted call, so its red needs callgrind in WSL.

**Limits: what stops this goal.**

- Counts decide, never the clock. A row is checked by the instruction the lowering emits, and a
  quadratic row by its ladder.
- A row whose fast form does not measure 2x or better on the figures in § *Why*, or on the size its
  item names, is not built. Its number is recorded in the rule's Declined list.
- An item gets one attempt, and a stage gets two sessions, counted in `### State`. At the ceiling the
  stage's open items move to `### Backlog` as declined, with what was found.
- No row is added to the table during the run, and no target figure holds the goal open.

**Lanes.** Stages 1, 2 and 3 need nothing from another goal. Stage 6 touches
`crates/nvs-runtime/src/callable.rs`, `dispatch.rs` and the callback argument's lowering, and can run as
a side goal at any time. Stage 7 touches the debug index and `crates/nvs-lsp/src/recorded.rs`, and stage
8 `crates/nvs-lsp/src/actions.rs`: both are disjoint from every other stage and can run as side goals.
Stages 4 and 5 share `forms.rs` with stage 2 and run in order after it. Stage 9 shares the call
lowering with stages 2 and 3 and runs after them. Stage 10 touches `crates/nvs-stdlib/src/str.rs` and
`json.rs` alone and can run as a side goal at any time.

## Stages

### 1. the rule, the table, the off switch and a case for every row

**Split:** once `every_form_row_has_its_cases` prints the missing cases, writing subagents write them
in three batches, one per folder under `tests/conformance/forms/`: `calls/`, `loops/` and `callbacks/`.
The batches are disjoint files and run side by side. One more writing subagent writes the three
ladders, copying the shape of `benches/scaling/arrays/sort.nvs`.

- [x] `docs/rules/expressions/a-cheaper-form-with-the-same-result-is-compiled-or-named.md:1`: the rule,
  with the three outcomes, both tables, the warnings, what is not a warning, the cost lines and the
  Declined list. One line in `docs/ground-rules.md`. `bun nv rules --render`, then `bun nv rules --check`
  is green.
- [x] `crates/nvs-ir/src/forms.rs:1`: the table as data, one row per line of the two tables in
  § *Standing decisions*, each with its id, its outcome set and the proof it needs. No row is applied
  yet.
- [x] `--no-cheaper-forms` on `run`, `test` and `serve` in `crates/nvs-cli/src/main.rs`, hidden, reaching
  the IR's options beside `--no-in-place` where that exists. `tools/nv/cmd/suite.ts` gains
  `--no-cheaper-forms-too`.
- [x] `crates/nvs-ir/src/lower/tests.rs`: `every_form_row_has_its_cases` reads the table and fails
  naming each missing case. A row with no proof owes one case where it holds and one for each
  condition in its "Holds when" cell that fails. A callback row owes three: proven, can fail, has an
  effect. A case is named `<row id>-<outcome>.nvst` and prints what the form returns.
- [x] The cases, through the subagents.
- [x] Three ladders under `benches/scaling/arrays/`:
  `forms-a-difference-written-as-filter-and-contains.nvs` (`// scaling: expect linear`),
  `forms-a-first-match-written-as-first-of-filter.nvs` (the first entry matches,
  `// scaling: expect constant`) and `forms-a-map-over-rows-does-not-read-each-row.nvs` (100 rows, the
  ladder's size is the row width, `// scaling: expect constant`). They are red until stages 2, 4 and 6.

### 2. a call form that needs no proof compiles as its fast form

- [ ] `crates/nvs-ir/src/lower/expr.rs:5160`: before a `CoreCall` is emitted, the lowering asks
  `forms.rs` with the resolved member, the argument expressions and how the result is used. The rows:
  the null test of `match` (`crates/nvs-stdlib/src/regex.rs:2090` against `:1901`), and `count` or
  `first` of `keys` (`crates/nvs-stdlib/src/arr.rs:3220`) or `values` (`arr.rs:4037`).
- [ ] The `filter` and `contains` row. The callback is matched as a tree: one `return` of
  `Core\Arr::contains(<captured b>, <its parameter>)`, negated or not. The element-type condition is
  read from the static types. A differential test in `crates/nvs-stdlib/src/arr.rs` runs `filter` with
  `contains` and `diff` over the same pairs of `int`, `uint`, `float`, `string`, `bool` and object
  lists, with a `float` equal to an `int` among them, and both give the same entries.
- [ ] `crates/nvs-ir/src/lower/exception.rs:382` `lower_try`: the two caught-conversion rows, under the
  conditions in § *Standing decisions*. The conversion's miss arm is the one `as ?T` already takes.
- [ ] The four tests the `call-forms` check names, each asserting the instruction emitted with the
  table on and with it off. Run once.

### 3. a loop, a literal and a static call take the direct path

- [ ] `crates/nvs-ir/src/lower/control.rs:880` `lower_foreach`: a subject that is a direct
  `Core\Arr::keys` or `Core\Arr::values` call loops over the argument, binding the key or the value. The
  loop still takes its own reference to the array it began with.
- [ ] A subject that is a direct `Core\Arr::range` call (`crates/nvs-stdlib/src/arr.rs:4192`) is a
  counted loop. The argument checks `range` makes run once before the first step and throw the same
  errors. The direction is decided there too.
- [ ] `control.rs:1080`: `ArrayKeyAt` is emitted only when the body reads the key binding. The same
  holds for the key binding of a loop the two items above rewrote.
- [ ] `crates/nvs-ir/src/lower/expr.rs:6884`: a literal with no spread whose keys are all constants and
  all different takes the one-`ArrayNew` path with its keys. An integer key is written as the decimal
  string the array would store.
- [ ] `static::m()` and `static::NAME` (`expr.rs:5320` and `:481`) ask the override knowledge an
  instance call already asks (`expr.rs:5052`, filled by `mark_overridden_methods`,
  `crates/nvs-types/src/signatures.rs:1125`). Where the table has no such fact for a constant, the item
  adds it beside the method one.
- [ ] The five tests the `direct-paths` check names. Run once.

### 4. a form with a callback compiles as its fast form where the callback is proven, and is recorded where it is not

**Gate:** `crates/nvs-ir/src/effects.rs` exists, which is `in-place-everywhere` stage 5.

- [ ] The lowering records a candidate for each callback row at `crates/nvs-ir/src/lower/expr.rs:5160`:
  the site's span, the row, the callback's function and the operands the fast member takes. For a
  comparator row it also lowers the `by` function `fn(x) => E(x)`.
- [ ] `forms.rs` runs after `effects.rs` and before `crates/nvs-ir/src/ownership.rs`. For each candidate
  it reads the callback's summary and its error edges, and takes one of the three outcomes. A proven
  candidate has its instructions replaced by the one fast `CoreCall`. A candidate that can fail is
  kept in a list beside the unit's copy table, with its span and the fast form as text.
- [ ] The four tests the `callback-forms` check names. Run once.
- [ ] `bun nv suite tests/conformance/ --no-cheaper-forms-too` once, green.
- [ ] `tools/nv/cmd/scaling.ts`: `// scaling: rounds N` on a `run` ladder judges the difference of half
  and all of `N` rounds, as § *Standing decisions* says, with a test in `tools/nv/test/`. The two
  `expect constant` ladders under `benches/scaling/arrays/forms-` take it.
- [ ] `bun nv scaling --check arrays/forms-` once: the first two ladders grow as their files say.

### 5. a form only an edit can change is a warning with its fix, in nvs check and in the editor

**Gate:** `in-place-everywhere` stage 7 is done: `nvs check` runs the lowering, and
`crates/nvs-lsp/Cargo.toml` links `nvs-ir`.

**Split:** one writing subagent writes the two `.lspt` cases, copying the shape of an existing case
under `tests/lsp/diagnostics/`. A `reader` subagent runs `nvs check` over `docs/examples/`,
`benches/` and `website/snippets/` once the codes land and lists every file that now warns.

- [ ] The form warning in `crates/nvs-diagnostics/src/code/warnings.rs`, after the last code, with its
  card in end-user prose. Its message names the member to use. Its `Suggestion` is the fast form's
  text, marked `alternative`. `nvs check` prints it, and `crates/nvs-lsp/src/actions.rs` translates it
  as it does every other suggestion.
- [ ] The repeated-read warning, for the cache, `queryParameter` and file rows, matched in the
  lowering under the exact shape in § *Standing decisions*. One code and one card. The session row is
  measured first, once, under `nvs serve` with a 16 KiB session record: two `Core\Session::get` calls
  against one. Under 2x it is not built, and its number goes to the rule's Declined list.
- [ ] Every file the `reader` subagent lists is rewritten to the fast form, or keeps its form where
  the example is about that form. Its `.out` file does not change.
- [ ] The two tests the `warnings` check names and the two `.lspt` cases. Run once.

### 6. a callback the checker proved is called without the check of each call

- [ ] An `Explore` subagent lists first: where the checker accepts a callback argument against a
  `Core` member's signature (`crates/nvs-types/src/expr/calls.rs`), and where a callable argument of a
  `CoreCall` is written to its slot (`crates/nvs-codegen/src/emit.rs:3070`).
- [ ] The checker records, for each callback argument of a `Core` member call, whether every parameter
  of the callable is proven by the subject's static types. The lowering marks the argument.
- [ ] `crates/nvs-runtime/src/callable.rs:253`: `call_callable` takes the mark as `TagCheck::Proven` and
  skips both checks. In a debug build it runs them and aborts on a mismatch.
- [ ] The three tests the `proven-callbacks` check names. Run once.
- [ ] `bun nv scaling --check arrays/forms-a-map-over-rows` once: constant in the row width. It needs
  `// scaling: rounds N` from stage 4; a stage 6 that runs first builds that item itself.

### 7. a recorded request shows its HTTP, cache, session and file calls by line

**Split:** one writing subagent writes the two `.lspt` cases, copying the shape of an existing case
under `tests/lsp/lens/`.

- [ ] An `Explore` subagent lists first: where a `query` event takes its site, where an `http` event is
  made, and where `crates/nvs-cli/src/debug/index.rs` stores the site columns.
- [ ] An `http` event, a cache read or write, a session read or write and a file open each record the
  site of their call when a request is recorded. `rule:observability/trace-events-carry-a-kind` gains
  the three new kinds, and `rule:observability/a-query-span-names-its-call-site` covers them.
- [ ] `crates/nvs-lsp/src/recorded.rs:40` `Ran` serves every kind, and the lens line names the kind.
  `rule:ide/a-query-lens-shows-what-one-recorded-request-ran-at-a-site` says so.
- [ ] The two `.lspt` cases the `recorded-sites` check names. Run once.

### 8. a parameter declared callable or mixed takes the type every caller passes, as an editor action

**Split:** one writing subagent writes the two `.lspt` cases, copying the shape of an existing case
under `tests/lsp/actions/`.

- [ ] An `Explore` subagent lists first: where the action of
  `rule:ide/narrow-an-annotation-to-its-literal` is computed in `crates/nvs-lsp/src/`, and how a
  reference from the workspace index reaches the checker's type for an argument.
- [ ] The action, under the conditions in § *Standing decisions*. For a `callable` it writes the
  callable type with the parameter and result types the callers' values declare.
- [ ] `rule:ide/a-code-action-writes-only-what-is-already-determined` gains the action.
- [ ] The two `.lspt` cases the `typed-parameter-action` check names. Run once.

### 9. a method call the compiler can name is direct, and a method that returns one property reads it

- [ ] `crates/nvs-types/src/signatures.rs:1125` `mark_overridden_methods` marks a method by the class
  that declares it, so one override anywhere makes every call of it virtual. The fact becomes one per
  receiver class: whether any class at or below it declares the method again, and whether the method
  has exactly one body at or below it. `crates/nvs-ir/src/lower/expr.rs:5052` reads it for the
  receiver's static class. An interface or abstract method with one body in the program is direct by
  the same fact.
- [ ] A direct call whose one body is `return $this->p;`, with `p` a property that has no hook, is
  lowered as the read of `p` on the receiver. A class that implements a property observer or guard
  gets the same read the method body would have made.
- [ ] The three tests the `named-calls` check names. Run once.

### 10. a split with a limit, a search that ignores case and a JSON validity test read only what their result needs

**Split:** one writing subagent writes the ladder, copying the shape of
`benches/scaling/strings/at.nvs`.

- [ ] `benches/scaling/strings/forms-a-key-split-off-a-long-line.nvs`
  (`Core\Str::split($line, '=', { limit: 2 })`, the ladder's size is the line's length,
  `// scaling: expect constant`).
- [ ] `crates/nvs-stdlib/src/str.rs:2176`: the count of pieces stops at a positive `limit`. The memory
  check before the build still asks for what will be built.
- [ ] `str.rs:2831`: the plain-ASCII path of a search that ignores case. The item first reads how a
  string knows its text is plain ASCII, and uses that. Without such a fact it tests the needle once
  and the subject as it scans, and falls back at the first byte outside ASCII.
- [ ] `crates/nvs-stdlib/src/json.rs:141`: `isValid` runs the reader with nothing built. Measure it once
  against the figure in § *Why*. Under 2x it is reverted, and its number goes to the rule's Declined
  list.
- [ ] The three tests the `members` check names, and `bun nv scaling --check strings/forms-`. Run once.

### 11. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stage 1 is done, in one session. The rule is
`rule:expressions/a-cheaper-form-with-the-same-result-is-compiled-or-named` (status `designed`, owned
by this goal). `crates/nvs-ir/src/forms.rs` holds the eighteen rows as `ROWS`, each with a
`Proof::Static` list of `Condition`s or `Proof::Callback`, a `Folder` and the `stage` that applies it;
`Row::cases` gives each case's file and expected `Outcome`. No row is applied: the lowering does not
read the table yet. `--no-cheaper-forms` reaches `nvs_ir::lower::without_cheaper_forms` through
`crates/nvs-cli/src/in_place.rs`, which now holds both differential switches, and
`cheaper_forms_on()` in `crates/nvs-ir/src/lower/mod.rs` is what each row's site asks.
`bun nv suite --no-cheaper-forms-too` runs it. The 55 cases under `tests/conformance/forms/` pass, and
`every_form_row_has_its_cases` is green.

The clause "the catch variable is unread" cannot fail alone, so its two cases also fail "the catch
body is `$v = null;`". A failed `as int` and `Core\Uuid::parse` throw `RuntimeError`.

The ladders are red, as planned; stage 4 adds `// scaling: rounds N` (§ *Standing decisions*).
Sessions used: stage 1, one. The sweep's `env-secret-proofs` red is stale: no goal defines it.

### Next

Stage 2 from its first item: `crates/nvs-ir/src/lower/expr.rs:5160`, where a `CoreCall` is emitted,
asks `crate::forms` and `cheaper_forms_on()`. Each row's site matches the rows with `stage: 2` in
`forms.rs`, and each of the four `call-forms` tests asserts the instruction with the table on and
inside `without_cheaper_forms`. Stages 6, 7, 8 and 10 can start as side goals at any time; stage 6's
ladder needs stage 4's `rounds` item, or builds it.

Tests expected red: none. The three ladders under `benches/scaling/arrays/forms-` stay red until
stages 4 and 6, and no sweep runs them before stage 4's check.

### Backlog

- From `strings-in-place`, which took it from `arrays-in-place`: `Core\Arr::overlayDeep` on a sole
  subject still copies each nested map it merges into. A nested map that is itself sole could be
  changed in place. No ladder shows it yet.
