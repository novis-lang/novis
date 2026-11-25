---
looper: 1
title: "A query value may be absent, the time values and a bool cross a route as text, the value types cross JSON as text, parse in a final class may return an existing object, and a link writes every value its route reads"
summary: "Reached when a #[Core\\Query] parameter declared ?T $x = null is optional and null when its key is absent, a bool query value reads 1 or 0, Date, TimeOfDay and Instant convert as path captures and query values and DateTime as a query value, read by the matcher itself, Core\\Router::url writes each as its ISO text, a Parses class writes itself back with format and Core\\Router::url calls it, Core\\Router::url refuses when the program compiles a written value whose type its parameter cannot take, Date, TimeOfDay, DateTime, Duration, Zone and Core\\Uuid cross JSON as text like Instant, a final class's parse may return an existing object of its class, the query example shows the string fallback, and the rules, the API document, the feature proofs and the website say so."
context: [docs/rules/security/route-capture-is-laundered-by-its-type.md, docs/rules/routing/a-query-parameter-is-declared-like-a-capture.md]
stages:
  - id: 1
    title: "a query value declared ?T $x = null is optional and null when its key is absent, and its link and its API entry say so"
    status: done
    context: [crates/nvs-types/src/routes.rs, crates/nvs-types/src/crossing.rs, crates/nvs-runtime/src/routes.rs, crates/nvs-stdlib/src/router.rs, crates/nvs-cli/src/openapi.rs, docs/rules/core-api/required-optional-and-nullable.md, docs/rules/routing/a-leftover-link-key-is-a-query-string.md]
  - id: 2
    title: "a bool query value reads 1 or 0, and a bool path capture still does not compile"
    status: open
    context: [crates/nvs-types/src/routes.rs, crates/nvs-types/src/crossing.rs, crates/nvs-runtime/src/routes.rs, crates/nvs-stdlib/src/router.rs, crates/nvs-cli/src/openapi.rs, docs/rules/routing/a-bad-query-value-is-a-400.md]
  - id: 3
    title: "a final class's parse may return an existing object of its class, and allows checks it"
    status: open
    context: [crates/nvs-types/src/check.rs, crates/nvs-types/src/conformance.rs, crates/nvs-types/src/signatures.rs, docs/rules/statements/static-is-a-member-modifier.md]
  - id: 4
    title: "Core\\Router\\Match holds each converted query value, so allows checks a record a query value's parse loaded"
    status: open
    context: [crates/nvs-stdlib/src/router.rs, crates/nvs-types/src/routes.rs, docs/rules/routing/a-bad-query-value-is-a-400.md, docs/rules/routing/matched-once-before-the-handler.md]
  - id: 5
    title: "the ISO readers move into nvs-runtime, and the matcher converts Date, TimeOfDay and Instant captures and query values and DateTime query values"
    status: open
    context: [crates/nvs-runtime/src/uuid.rs, crates/nvs-stdlib/src/time.rs, crates/nvs-runtime/src/routes.rs, crates/nvs-types/src/routes.rs, crates/nvs-types/src/commands.rs, crates/nvs-types/src/crossing.rs, crates/nvs-stdlib/src/router.rs, docs/rules/errors/http-message-defects.md]
  - id: 6
    title: "Core\\Router::url, Core\\Uri::buildQuery, the API document and a command's arguments take the time values"
    status: open
    context: [crates/nvs-stdlib/src/router.rs, crates/nvs-stdlib/src/uri.rs, crates/nvs-cli/src/openapi.rs, crates/nvs-runtime/src/commands.rs, crates/nvs-types/src/commands.rs, docs/rules/routing/link-target-and-params-are-checked.md, docs/rules/tooling/commands-are-compiled.md]
  - id: 7
    title: "Date, TimeOfDay, DateTime, Duration, Zone and Core\\Uuid cross JSON as text"
    status: open
    context: [crates/nvs-types/src/derive.rs, crates/nvs-stdlib/src/json.rs, crates/nvs-runtime/src/object.rs, crates/nvs-cli/src/openapi.rs, docs/rules/core-classes/derive-property-list.md, docs/rules/core-classes/db-column-types.md]
  - id: 8
    title: "a Parses class writes itself with format, Core\\Router::url calls it, and url checks the type of each value written in its call"
    status: open
    context: [crates/nvs-types/src/iter_lib.rs, crates/nvs-stdlib/src/registry.rs, crates/nvs-stdlib/src/router.rs, crates/nvs-stdlib/src/uri.rs, crates/nvs-types/src/links.rs, crates/nvs-diagnostics/src/code/types_e07.rs, docs/rules/routing/link-target-and-params-are-checked.md, docs/rules/expressions/try-parse.md]
  - id: 9
    title: "stages 1 to 4 have their feature proofs, and the query example shows the string fallback"
    status: open
    context: [docs/examples/lang/attributes/core-route-and-core-access-the-route-table/about.md, docs/examples/lang/classes/parses/about.md]
  - id: 10
    title: "stages 5 to 8 have their feature proofs, and the website says what changed"
    status: open
    context: [website/src/content/docs/syntax/attributes/routes.mdx, website/src/content/docs/syntax/attributes/json.mdx, website/src/data/changes.json]
  - id: 11
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: optional-query
    stage: 1
    name: "a ?T $x = null query value is null when its key is absent and converted when present, ?T without = null does not compile and names = null, and a link leaves out a null query value"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/an-optional-query-value-is-null-when-its-key-is-absent.nvst, --case, tests/conformance/reject/a-nullable-query-value-needs-a-null-default.nvst, --case, tests/conformance/core/a-link-leaves-out-a-query-key-whose-value-is-null.nvst]
    timeout: 1800
  - id: optional-query-document
    stage: 1
    name: "the API document lists a ?T $x = null query value as not required, typed by T"
    run: [cargo, test, --test, openapi, a_nullable_query_value_]
    tests: [a_nullable_query_value_is_optional_and_typed_by_its_inner_type]
    timeout: 1800
  - id: bool-query
    stage: 2
    name: "a bool query value reads 1 and 0 and is a 400 for every other text, and a bool, true or false path capture does not compile"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/a-bool-query-value-is-1-or-0.nvst, --case, tests/conformance/reject/a-bool-route-capture-does-not-compile.nvst]
    timeout: 1800
  - id: bool-query-runtime
    stage: 2
    name: "the bool conversion reads 1 and 0 and nothing else, and true and false admit only their own digit"
    run: [cargo, test, --lib, a_bool_query_value_]
    tests: [a_bool_query_value_reads_1_and_0_and_nothing_else, a_bool_query_value_of_a_single_value_type_admits_its_own_digit]
    timeout: 1800
  - id: bool-query-document
    stage: 2
    name: "the API document lists a bool query value as the integer 0 or 1"
    run: [cargo, test, --test, openapi, a_bool_query_value_]
    tests: [a_bool_query_value_is_the_integer_0_or_1]
    timeout: 1800
  - id: final-parse
    stage: 3
    name: "a final class may return any object of its class where it declares static and may write its own name where its parent writes static, a non-final class still may not, and allows reads the object a capture's parse loaded"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/class/a-final-class-may-return-any-object-of-itself-where-it-declares-static.nvst, --case, tests/conformance/class/a-final-class-may-declare-its-own-name-where-its-parent-declares-static.nvst, --case, tests/conformance/core/a-capture-parsed-to-a-stored-object-reaches-allows.nvst, --case, tests/conformance/class/a-static-return-must-be-the-called-class.nvst]
    timeout: 1800
  - id: query-in-match
    stage: 4
    name: "Core\\Router\\Match holds each converted query value under its parameter's name, allows reads a record a query value's parse loaded, Core\\Request::route() reads null for a query value, and a parameter both captured and marked #[Core\\Query] does not compile"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/a-query-value-parsed-to-a-stored-object-reaches-allows.nvst, --case, tests/conformance/core/a-match-reads-a-query-value-by-its-name-only-inside-dispatch.nvst, --case, tests/conformance/reject/a-parameter-filled-by-a-capture-cannot-also-be-a-query-value.nvst]
    timeout: 1800
  - id: time-captures
    stage: 5
    name: "the matcher converts Date, TimeOfDay, Instant and DateTime text with the readers fromIso uses, on the segment as it arrived"
    run: [cargo, test, --lib, time_capture_]
    tests: [time_capture_date_converts_and_refuses_what_is_not_one, time_capture_time_of_day_converts_and_refuses_what_is_not_one, time_capture_instant_converts_and_refuses_what_is_not_one, time_capture_datetime_reads_its_offset_and_zone, time_capture_reads_the_segment_as_it_arrived, time_capture_agrees_with_from_iso_over_one_table]
    timeout: 1800
  - id: time-routes
    stage: 5
    name: "a time capture that does not convert is a 404, a time query value that does not convert is a 400, and a DateTime path capture does not compile"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/a-time-capture-that-does-not-convert-is-a-404-and-a-time-query-value-a-400.nvst, --case, tests/conformance/reject/a-datetime-path-capture-does-not-compile.nvst]
    timeout: 1800
  - id: time-links
    stage: 6
    name: "Core\\Router::url and Core\\Uri::buildQuery write a time value as its ISO text, the route matches the link it built, and a command argument takes a time value"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/a-link-writes-a-time-value-as-its-iso-text-and-its-route-matches-it.nvst, --case, tests/conformance/core/a-command-argument-takes-a-time-value.nvst]
    timeout: 1800
  - id: time-document
    stage: 6
    name: "the API document gives each time parameter the JSON Schema format that fits its text, and none where none fits"
    run: [cargo, test, --test, openapi, a_time_parameter_]
    tests: [a_time_parameter_has_the_format_json_schema_defines_for_it]
    timeout: 1800
  - id: json-text
    stage: 7
    name: "every Core value type on the JSON list round-trips as text, and a bad text is a ParseError naming its path"
    run: [cargo, test, --lib, value_text_]
    tests: [value_text_round_trips_every_core_value_type, value_text_refuses_a_bad_text_with_its_path, value_text_duration_writes_what_parse_reads, value_text_datetime_keeps_its_zone]
    timeout: 1800
  - id: json-cases
    stage: 7
    name: "a derived codec, decodeAs and shapeAs carry Date, TimeOfDay, DateTime, Duration, Zone and Core\\Uuid as text, and a Core class with no JSON form is still E0756"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/a-derived-codec-carries-dates-durations-zones-and-uuids-as-text.nvst, --case, tests/conformance/core/decode-as-and-shape-as-read-the-value-types-from-text.nvst, --case, tests/conformance/reject/a-json-derive-field-of-a-core-value-type-with-no-json-form-is-refused.nvst]
    timeout: 1800
  - id: parses-links
    stage: 8
    name: "a Parses class must declare format, Core\\Router::url and Core\\Uri::buildQuery write a Parses value as its format text, the route matches that link, and any other object still throws naming Parses"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/a-link-writes-a-parses-value-as-its-format-text-and-its-route-matches-it.nvst, --case, tests/conformance/reject/a-parses-class-without-format-does-not-compile.nvst]
    timeout: 1800
  - id: link-types
    stage: 8
    name: "a value written in a Core\\Router::url call whose type its parameter cannot take does not compile, and a string, a mixed value and a computed params array still compile"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/reject/a-link-value-whose-type-its-parameter-cannot-take-does-not-compile.nvst, --case, tests/conformance/core/a-link-value-of-text-or-mixed-type-still-compiles.nvst]
    timeout: 1800
  - id: proofs
    stage: 10
    name: "every feature the goal's work reached has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, route-values]
    timeout: 1800
  - id: close
    stage: 11
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, route-values]
    timeout: 900
---

# A query value may be absent, the time values and a bool cross a route as text, the value types cross JSON as text, parse in a final class may return an existing object, and a link writes every value its route reads

## Why

A route that takes a date range or a page number from the query string meets four walls today.

- `#[Core\Query] ?uint $page = null` does not compile. `converts_from_segment`
  (`crates/nvs-types/src/routes.rs:2988`) sees `uint|null`, which no conversion reaches, and `E0752`
  says the type cannot arrive. The only way to make a key optional is a non-`null` default, so "no
  value given" has no spelling.
- `Core\Time\Date` and the other time classes are not route conversions and have no JSON form. A
  program declares the value `string` and calls `Date::fromIso` in every handler, and a
  `#[Core\Json\Derive]` property of one of them is `E0756` since commit 3d4584620c
  (`is_core_json_value`, `crates/nvs-types/src/derive.rs:2711`, admits `Instant` alone).
- A `Parses` class cannot be the obvious wrapper either. `parse` must return `static`, so a `parse`
  that loads a stored record is `E0741`, and declaring the class's own name is `E0404`, even in a
  `final` class where `static` can only mean that class.
- A yes-or-no query value has no type: `bool` is `E0752`, so `?archived=1` is read as a `string`.

`rule:routing/a-bad-query-value-is-a-400` keeps its decline of a lenient mode, and the user docs do
not yet show the string fallback that rule names.

The link is the other half of the route, and it writes no object. A `Parses` class can be a capture
or a query value, but `Core\Router::url` throws `RuntimeError` for its value, a `Stringable` one
included: `segment_text` (`crates/nvs-stdlib/src/router.rs:1370`) calls `value_to_string`, which
writes no object, and never `toString`. So every link writes the class's text by hand, in the form
the route reads. A value whose type its parameter cannot take, such as a `Date` for a `uint`
capture, compiles and throws only when the call runs, because `literal_args`
(`crates/nvs-types/src/links.rs:482`) reads folded literals alone.

## Standing decisions

**The user's calls (2026-11-24).**

- **`#[Core\Query] ?T $x = null` is an optional query value.** The key may be absent, and then the
  value is `null`. It works for every type a query value takes, a `Parses` class included, and it
  means what row 4 of `rule:core-api/required-optional-and-nullable` means. `?T` without a default
  stays `E0752`, and its help says to add `= null`. `Core\Router::url` leaves out a key whose value is
  `null`. The generated API document marks the parameter optional.
- **`Core\Time\Date`, `TimeOfDay`, `DateTime` and `Instant` are built-in route conversions**, read by
  the matcher itself like `Core\Uuid`, so no program code runs at the door. A path segment that does
  not convert is not a match (`404`), and a query value that does not convert is a `400`. Each reads
  and writes its ISO text. `Core\Router::url` writes each value as that text, percent-encoded where the
  query string needs it. They are built-in conversions and not `Parses` classes, among other reasons
  because `time-values` gives `Date::parse` two arguments.
- **In JSON, `Date`, `TimeOfDay`, `DateTime`, `Duration`, `Zone` and `Core\Uuid` cross as text**, as
  `Instant` already does: in a `#[Core\Json\Derive]` property, in `Core\Json::decodeAs`, in
  `Core\Arr::shapeAs` and in the encoder. `Duration`'s text is the one `Duration::parse` reads, `Zone`'s
  is its IANA name and `Core\Uuid`'s is its canonical form.
- **A `DateTime`'s text carries its offset and zone name (RFC 9557)**, as the goal `time-values`
  builds it: `2026-01-05T08:30:00+01:00[Europe/Vienna]`. This goal builds on that text and does not
  build it.
- **In a `final` class, `static` means exactly that class.** So `E0741` accepts any value of the class
  in a body declaring `static`, and an override in a `final` class may declare the class itself where
  the parent declares `static` (`E0404`). This lets a `parse` load a stored record. The order of
  dispatch does not change: every typed capture and query value is bound before `Gate::allows`
  (`crates/nvs-stdlib/src/router.rs:3189`). The example and `about.md` show `refused` setting `404`
  for both `Refusal::BadCapture` and `Refusal::Denied`, so a visitor cannot learn which ids exist (a
  missing id is otherwise a `400` and a forbidden one a `403`). The rule fragment records the cost:
  the lookup runs for every request that matches the route, signed in or not.
- **A `bool` query value is written `1` or `0` only.** `bool` is allowed for `#[Core\Query]` and stays
  `E0752` for a path capture, and the single values `true` and `false` follow the same split. `1` is
  `true` and `0` is `false`. Any other text is a `400`: `true`, `on`, an empty value and a bare key
  included. `Core\Router::url` writes `1` or `0`. The rule's old position moves to its Declined list,
  and a `bool` path capture stays declined.
- **The query example shows the fallback for the declined lenient mode**:
  `#[Core\Query] string $page = ''` and `($page as ?uint) ?? 1`.
- **`Core\Router\Match` holds each converted `#[Core\Query]` value**, so `Gate::allows` checks a record
  loaded through `#[Core\Query] Order $order` exactly as it checks one from a path capture. `dispatch`
  already converts the query values before `allows` (`crates/nvs-stdlib/src/router.rs:3052`), but after
  the match is built (`router.rs:3043`); the match now carries them. `Core\Router\Match::parameter`
  (`router.rs:2840`) and `parameters` (`router.rs:2687`) read a query value by its parameter's name.
  `Core\Request::route()` outside `dispatch` converts no query value, so it reads `null` for one, and
  the rule and the card say so.

**The user's calls on the link (2026-11-24, later that day).**

- **`Parses` gains a second required member, `format(): string`**, an instance method that writes the
  text `parse` reads. The pair is `parse`/`format` by `rule:core-api/symmetric-names`. `Core\Router::url`
  and `Core\Uri::buildQuery` call it for a value whose class implements `Parses`, and write its text
  like any other: percent-encoded into its own segment, form-encoded in a query string. An object of
  any other class still throws, and the message names `Parses` and `format`. A `Stringable` object is
  not written through `toString`. Every `Parses` class in the tree gains the method, and the commit
  carries `BREAKING CHANGE`. `Core\Uuid` and the `Core` queue's id class, the two classes
  `implements_parses` (`crates/nvs-stdlib/src/registry.rs:4717`) finds, gain a `format()` that writes
  their `toString` text, and that check also needs `format`, so the roster stays those two.
- **`Core\Router::url` checks the static type of each value written in a `$params` array literal**,
  for a key that names a capture or a `#[Core\Query]` parameter. A value whose type is known and
  cannot be the parameter's is a compile error at the value: a `Date` for a `uint`, an `int` for a
  `Date`, one `Parses` class for another. A value passes when its type is the parameter's type or a
  subtype of it, when it is a `string` of any qualifier or `mixed`, or when it is a scalar whose text
  the parameter's conversion reads: `int` and `uint` for `int`, `uint` and `decimal`, `float` for
  `decimal`, any scalar or enum for `string`, and its backing type for a backed enum. The `null` part
  of a value's type is stage 1's to judge, not this check's. A computed `$params` stays unchecked, and
  the literal check of `rule:routing/link-target-and-params-are-checked` runs as before on what
  passes. The code is new, beside `E0772` (`crates/nvs-diagnostics/src/code/types_e07.rs:1287`), and
  its card names both types; `E0772` stays the code for a literal its conversion refuses.

**Calls the authoring session made (2026-11-24), each with its fallback.** The user has not confirmed
these; a session builds on them and files a `looper decision ask --built-on <n>` only if the code
forces the opposite.

- **The `%2F` question, settled: a `DateTime` is a query value only.** The matcher splits the path on
  `/` before anything decodes it (`crates/nvs-runtime/src/routes.rs:975`, and the module doc at
  `:73` keeps every capture percent-encoded until it crosses). But the server's door answers `400` to
  any path with `%2F` or `%5C` in it (`path_is_refused`, `crates/nvs-server/src/mount.rs:738`, asked
  at `crates/nvs-server/src/serve.rs:2102`, `rule:errors/http-message-defects`). Almost every zone
  name holds a `/`, so a `DateTime` in a path could never arrive. A `#[Core\Query] DateTime` is
  allowed, and a `DateTime` path capture is `E0752`, whose help names `#[Core\Query]` and `Instant`.
  The door is not relaxed: it stops a proxy and this server from reading one path two ways, which is
  request isolation. `Zone` is not a route conversion at all, for the same reason and because nobody
  asked for one.
- **How the match carries the query values.** `dispatch` builds the match with its captures
  (`match_value`, `crates/nvs-stdlib/src/router.rs:3043`), converts the query values
  (`query_values`, `router.rs:3086`), then writes each into the match's parameters slot under its
  parameter's name before it calls `allows` (`router.rs:3189`). A query value that does not convert
  is still the `400`, and `refused` gets the match with its captures only, as today (`router.rs:3061`).
  An absent `?T $x = null` key is absent from `parameters()`, as an absent optional capture is, so
  `parameter()` reads `null`; an absent key with another default holds that default. The `CAPTURE`
  union (`router.rs:695`) gains the types a query value adds (`bool`, and `DateTime` in stage 5).
  What it spends: one array entry per query value per matched request, which the bound value already
  is, so no copy. Fallback, if writing into the built match's slot fights its layout: build the match
  once, after the query values, and give `refused` a match built from the captures alone.
- **A capture and a query parameter never share a name.** The route table does not refuse it today:
  a path `/orders/{id}` with `#[Core\Query] uint $id` compiles with no error (checked on 2026-11-24
  with the debug `nvs`), because `check_captures` finds the parameter by name
  (`crates/nvs-types/src/routes.rs:2810`) and `query_params` (`routes.rs:3192`) takes every
  `#[Core\Query]` parameter, so one parameter is claimed twice. Stage 4 makes that a compile error at
  the parameter, with a new code after the last `E07xx` in
  `crates/nvs-diagnostics/src/code/types_e07.rs`, whose help says to remove `#[Core\Query]` or the
  capture. A method's parameter names are unique, so with this check every key of `parameters()`
  names exactly one capture or one query value, and no order between the two is needed.
- **The optional key's representation.** `nvs_runtime::routes::QueryParam::default`
  (`crates/nvs-runtime/src/routes.rs:286`) becomes `QueryDefault { Required, Null, Given(Param) }`,
  and no `Param::Null` arm is added, because a capture is never `null`. The checker records the type
  of a `?T $x = null` query value as `T` on its `RouteParam` (`crates/nvs-types/src/routes.rs:3225`),
  so the crossing, the API document and the link check read `T` with no union to strip.
- **`?T $x = <a value that is not null>` is `E0752`**, with help to drop the `?` or write `= null`. A
  query string cannot carry a `null`, so the `?` would say nothing.
- **The `bool` conversion.** `CaptureConv::Bool(Option<bool>)`, where `None` is `bool` and `Some(v)`
  is the single value `true` or `false`, admitting only its own digit, and `Param::Bool(bool)`.
  `converts_from_segment` splits into a path half, which still refuses `bool`, `true`, `false` and
  `DateTime`, and a query half that admits them. The API document lists a `bool` query value as
  `{"type": "integer", "enum": [0, 1]}`, because a generated client given `"type": "boolean"` writes
  `true`, which is a `400`. The example's `about.md` tells a form author to write `value="1"` on a
  checkbox, because a browser sends `on` by default.
- **A time capture is read as the segment arrived, never percent-decoded.** One value has one
  spelling, which is why `%34` is not `4` for a `uint` (`crates/nvs-runtime/src/routes.rs:73`).
  `Core\Router::url` writes a time value's ISO text into a path segment unencoded, because every byte
  of the three path grammars (digits, `-`, `:`, `.`, `T`, `Z`, `+`) is legal in a segment and none is
  `/`, `?`, `#` or `%`. In a query string the text goes through `Core\Uri::buildQuery`'s form encoding
  like every value, so `+` is written `%2B`. A `+` typed by hand into a query value is a space when it
  is decoded, and that value is a `400`.
- **The ISO readers move into `nvs-runtime`**, as `crate::uuid::read` did
  (`crates/nvs-runtime/src/uuid.rs:44`): a new `crates/nvs-runtime/src/iso.rs` holds the four
  `fromIso` grammars that live at `crates/nvs-stdlib/src/time.rs:3802` today, and `time.rs` and
  `validate_iso` (`time.rs:3831`) call them. One grammar per class, read by `fromIso`, by the checker,
  by the matcher, by a command's argument and by JSON. `nvs-runtime` already depends on `jiff`
  (`crates/nvs-runtime/Cargo.toml:59`), so no crate is added. The zone is looked up with
  `jiff::tz::TimeZone::get`, as `Zone` looks it up, and a bracket longer than the longest name in the
  database is refused before the lookup.
- **A `DateTime` read from a route, a command line or JSON needs its `[Zone]`.** There is no `$zone`
  argument to fall back on, so a text without one does not convert.
- **A command's argument takes the time values too.** `converts_from_string`
  (`crates/nvs-types/src/commands.rs:979`) is the one roster a route and a command share, and its doc
  says the route pass reads it so that no second list grows. The four classes join it, and
  `nvs_runtime::commands::ArgConv` (`crates/nvs-runtime/src/commands.rs:62`) gains their arms; a word
  that does not convert is a usage error, as for `Core\Uuid`. A `DateTime` argument is allowed, since
  no door refuses a `/` in an argument.
- **`Core\Uri::buildQuery` writes a time value too.** Its `scalar_text` (`crates/nvs-stdlib/src/uri.rs:2482`)
  is the query writer `Core\Router::url` uses, so one writer gains the rows and both members follow.
- **JSON's representation.** `CodecTy::Instant` (`crates/nvs-runtime/src/object.rs:1161`) becomes
  `CodecTy::ValueText`, and the class label the field already carries (`Erased::class`,
  `crates/nvs-types/src/derive.rs:1444`) says which reader and writer. `is_core_json_value` becomes a
  closed list `JSON_TEXT_CLASSES` of the seven names, each taken from its module's constant as
  `DB_COLUMN_CLASSES` (`derive.rs:2619`) takes them, and that row list does not change. `Core\Uri`
  stays off the JSON list.
- **The API document's formats.** `Date` is `{"type": "string", "format": "date"}` and `Instant`
  `"format": "date-time"`. `TimeOfDay` and `DateTime` are `{"type": "string"}` with no format, because
  JSON Schema's `time` and `date-time` need an offset, and `date-time` has no zone in brackets. A
  validator would refuse the value under either. `Duration` and `Zone` are `{"type": "string"}`, and
  `Core\Uuid` keeps `"format": "uuid"`.
- **The `final` rule, in code.** In a `final` class (`class_is_final`,
  `crates/nvs-types/src/signatures.rs:2477`), a body declaring `static`, `?static` or `array<static>`
  may return any expression whose type the declared type accepts, with `static` read as the class.
  A non-`final` class keeps the list at `crates/nvs-types/src/check.rs:1627`. An override in a `final`
  class may write the class's own name or `self` wherever the parent or the interface writes `static`.
- **Gate:** the goal `time-values` is reached (`docs/goals/time-values.md` is gone) before stage 5,
  because stages 5 to 7 read and write the `toIso` and `fromIso` text that goal builds, `DateTime`'s
  zone included. Stages 1 to 4 do not wait for it.

**Declined, each written into the rule the table names in the stage that changes that rule.**

| Proposal | Reason | Rule |
|---|---|---|
| A `bool` path capture | A path names a resource and a yes-or-no filters it, which is the query string's job. A segment `1` would also be claimed by a `bool` and a `uint` capture at one position | `rule:security/route-capture-is-laundered-by-its-type` |
| Reading `true`, `false`, `on`, `yes` or an empty value as a `bool` | One value has one spelling, and `Core\Router::url` and `Core\Uri::buildQuery` already write `1` and `0` (`crates/nvs-stdlib/src/uri.rs:2483`). A second spelling makes two URLs for one page | `rule:security/route-capture-is-laundered-by-its-type` |
| A `DateTime` path capture with its `/` written `%2F` | The door refuses `%2F` to keep one path from being read two ways, and that is not traded for a spelling | `rule:security/route-capture-is-laundered-by-its-type` |
| A zone written another way in a path, such as `Europe~Vienna` | A spelling no other tool reads; `Instant` in the path or `DateTime` in the query already say the same moment | `rule:security/route-capture-is-laundered-by-its-type` |
| Percent-decoding a time capture before it converts | Two spellings of one URL, which the `uint` rule already refuses | `rule:security/route-capture-is-laundered-by-its-type` |
| `?T $x = 1`, a nullable query value with a default that is not `null` | A query string cannot carry `null`, so the `?` says nothing | `rule:routing/a-query-parameter-is-declared-like-a-capture` |
| `Core\Router::url` writing an object through `Stringable` | `toString` is the text a person reads, and it may differ from the text `parse` reads. A link written from it is a `404` | `rule:routing/link-target-and-params-are-checked` |
| A second interface beside `Parses` that only writes the text | A class could then be a capture that no link can be built to, and the language gains a name for half a contract | `rule:routing/link-target-and-params-are-checked` |
| Refusing a `string` value for a `uint` or a `Date` parameter | A program often has the text already, such as a value read from a form. The route checks that text when the link is followed | `rule:routing/link-target-and-params-are-checked` |

**What it spends, and which priority it buys.**

- *Optional query value*: nothing at run time; an absent key gives `null` in place of a converted
  default. Buys usability and simplicity of the language surface: one spelling for "may be absent".
- *`bool` query value*: one comparison of the value's text with `1` and `0`. Buys usability. Security
  does not move: a path capture still has no `bool`.
- *Time conversions in the matcher*: a fixed grammar of at most about 50 bytes for a path capture,
  run at the door for a request that reaches a time capture's row, with no program code and no
  allocation but the converted value. A `DateTime` query value also looks up its zone in the bundled
  database once. A `Param` grows by the largest new arm, a `jiff::Zoned`, held per in-flight request:
  O(in-flight), as `rule:programs/memory-priority` asks. Buys usability, and security over the
  `string`-and-convert-by-hand it replaces, because the type is checked before the handler runs.
- *Unencoded time text in a path*: no cost. Buys one spelling per value.
- *JSON as text*: one string per encoded value and one parse per decoded value, charged to the
  request, as `Instant` already is. Buys usability.
- *`final` class `parse`*: no cost in the compiler. The program's lookup runs for every request that
  matches the route, before `allows`, signed in or not. That is a read of the program's own store per
  matched request, chosen by the program when it declares the type. The time the lookup takes is not
  hidden, so a program that must not show whether an id exists even by timing declares the capture
  `string` and loads the record in the route method after `allows`. The rule fragment says so. The
  same holds for a query value of such a class, which now reaches `allows` too.
- *Query values on the match*: one array entry per bound query value per matched request, the value
  `dispatch` already holds. Buys security: `allows` sees every input the route method will see.
- *`Parses::format`*: one method call per `Parses` value per link built, charged to the request that
  builds it, and one more method in every `Parses` class. Buys simplicity: one class carries both
  directions, so a change to its text changes the reader and the writer together.
- *The link type check*: compile time only, one type comparison per written key. Buys usability: a
  link its route cannot read fails when the program compiles, not when it runs.

**What stops it.** One attempt per item and two sessions per stage. An item that cannot land in one
attempt is recorded in the handoff with what failed, and the stage goes on. A count, never a timing,
decides whether a cost changed. Every behaviour change starts with its failing test.

**Lanes.** Stages 1 and 2 share `crates/nvs-types/src/routes.rs`, `crates/nvs-types/src/crossing.rs`,
`crates/nvs-runtime/src/routes.rs`, `crates/nvs-stdlib/src/router.rs` and
`crates/nvs-cli/src/openapi.rs`, so stage 2 follows stage 1. Stage 3 touches only `check.rs`,
`conformance.rs` and `signatures.rs` in `crates/nvs-types/` and the diagnostic cards, and may run
beside stages 1 and 2. Stage 4 changes `crates/nvs-stdlib/src/router.rs` and
`crates/nvs-types/src/routes.rs`, so it follows stage 2, and its test of a `final` class's `parse`
needs stage 3. Stage 5 waits on the gate above and on stage 4, whose files it shares. Stage 6
follows stage 5. Stage 7 waits on the gate and on stage 5, because both change
`crates/nvs-stdlib/src/time.rs`. Stage 8 follows stages 6 and 7: it shares `router.rs`, `uri.rs` and
the link rule with stage 6, and `types_e07.rs` with stage 7. It does not wait on the gate. Stage 9
needs stages 1 to 4, and stage 10 needs everything.

## Stages

### 1. a query value declared ?T $x = null is optional and null when its key is absent, and its link and its API entry say so

**Split:** none, one session.

- [x] The failing cases first, at the paths the check `optional-query` names:
  `tests/conformance/core/an-optional-query-value-is-null-when-its-key-is-absent.nvst:1` (a `?uint`, an
  enum, a set of allowed values and a `Parses` class, each absent, present and bad),
  `tests/conformance/reject/a-nullable-query-value-needs-a-null-default.nvst:1` (`?uint $p` and
  `?uint $p = 1`, each `E0752` with its help) and
  `tests/conformance/core/a-link-leaves-out-a-query-key-whose-value-is-null.nvst:1`. Each run once by
  name and red.
- [x] `crates/nvs-types/src/routes.rs:3178` (`query_params`): a declared `?T` whose default is the
  written `null` converts as `T` and records `T` as its `ty` (`:3225`); `?T` without a default, or
  with another default, is `E0752` (`:3203`) with the help § *Standing decisions* gives. The `E0752`
  card (`crates/nvs-diagnostics/src/code/types_e07.rs:983`) names `= null`.
- [x] `crates/nvs-runtime/src/routes.rs:278`: `QueryParam::default` becomes `QueryDefault`;
  `crates/nvs-types/src/crossing.rs:183` (`query_default`) writes `QueryDefault::Null` for the `null`
  constant; `crates/nvs-stdlib/src/router.rs:3139` (`query_value`) binds `null` for an absent key with
  that default.
- [x] `crates/nvs-stdlib/src/uri.rs:2589` already drops a `null` value; the link case pins it for
  `Core\Router::url`, and `crates/nvs-stdlib/src/router.rs:1490` (`substitute`) needs no change unless
  the case is red.
- [x] `crates/nvs-cli/src/openapi.rs:778` (`parameter`): the test the check `optional-query-document`
  names, in `crates/nvs-cli/tests/openapi.rs:683`'s neighbourhood, run once by name.
- [x] `docs/rules/routing/a-query-parameter-is-declared-like-a-capture.md:10`: "a default is what makes
  the key optional" says that `?T $x = null` is the default `null`, that `?T` needs `= null`, and the
  Declined row of § *Standing decisions*. `docs/rules/core-api/required-optional-and-nullable.md:1`:
  the query door answers rows 1, 3 and 4, and row 2 is `E0752` there because a query string cannot
  carry `null`. `docs/rules/routing/a-leftover-link-key-is-a-query-string.md:1`: a `null` value writes
  no key.

### 2. a bool query value reads 1 or 0, and a bool path capture still does not compile

**Split:** none, one session.

- [ ] The failing case first: `tests/conformance/core/a-bool-query-value-is-1-or-0.nvst:1` (`1`, `0`,
  `true`, `on`, an empty value, a bare key, an absent key with a default, and a `true` single value),
  run once by name and red. `tests/conformance/reject/a-bool-route-capture-does-not-compile.nvst:1`
  gains a `true` capture and stays green.
- [ ] `crates/nvs-types/src/routes.rs:2988`: `converts_from_segment` splits into a path half (`:2870`,
  still refusing `bool`, `true` and `false`) and a query half (`:3200`) that admits them; the help at
  `:3210` lists `bool` for a query value.
- [ ] `crates/nvs-runtime/src/routes.rs:129` and `:217`: `CaptureConv::Bool(Option<bool>)`;
  `crates/nvs-runtime/src/routes.rs:292`: `Param::Bool(bool)`; `crates/nvs-types/src/crossing.rs:129`
  (`capture_conv`) maps the three types; `crates/nvs-stdlib/src/router.rs:2555` (`capture_value`)
  builds the `bool`. The tests the check `bool-query-runtime` names, beside `convert`, run once by
  name.
- [ ] `crates/nvs-cli/src/openapi.rs:837` (`schema`): a `bool` query value is
  `{"type": "integer", "enum": [0, 1]}`. The test the check `bool-query-document` names, run once by
  name.
- [ ] `crates/nvs-stdlib/src/uri.rs:2483` already writes `1` and `0`; the case above builds one link
  with `Core\Router::url` and follows it.
- [ ] `docs/rules/security/route-capture-is-laundered-by-its-type.md:6`: the `bool` paragraph says a
  query value is `1` or `0` and why, a path capture stays `E0752`, and the first two Declined rows of
  § *Standing decisions* join the Declined list. `docs/rules/routing/a-bad-query-value-is-a-400.md:1`:
  a `bool` value other than `1` or `0` is a `400`.

### 3. a final class's parse may return an existing object of its class, and allows checks it

**Split:** none, one session.

- [ ] The failing cases first, at the paths the check `final-parse` names:
  `tests/conformance/class/a-final-class-may-return-any-object-of-itself-where-it-declares-static.nvst:1`,
  `tests/conformance/class/a-final-class-may-declare-its-own-name-where-its-parent-declares-static.nvst:1`
  and `tests/conformance/core/a-capture-parsed-to-a-stored-object-reaches-allows.nvst:1` (an `Order`
  loaded by `parse` from a path capture, `allows` reading it with `$route->parameter('order')`, and
  `refused` setting `404` for `BadCapture` and `Denied`). Each run once by name and red.
  `tests/conformance/class/a-static-return-must-be-the-called-class.nvst:1` stays green.
- [ ] `crates/nvs-types/src/check.rs:1587` (`reject_static_return_of_another_class`): in a `final`
  class (`crates/nvs-types/src/signatures.rs:2477`), a returned expression is accepted when the
  declared type accepts its type with `static` read as the class; the list at `check.rs:1627` stays
  for every other class. The `E0741` card
  (`crates/nvs-diagnostics/src/code/types_e07.rs:793`) and the help at `check.rs:1617` name the
  `final` case.
- [ ] `crates/nvs-types/src/conformance.rs:449` (the interface half, `reject_dropped_static_return`
  at `:631`) and `:931` (the override half): in a `final` class, the class's own name or `self` in the
  position of the parent's `static` keeps the promise. The `E0404` card
  (`crates/nvs-diagnostics/src/code/types.rs:35`) and the help at `conformance.rs:663` say a `final`
  class may write its own name.
- [ ] `docs/rules/statements/static-is-a-member-modifier.md:70`: in a `final` class `static` is the
  class, so a body may return any value of it and an override may name it.
  `docs/rules/security/route-capture-is-laundered-by-its-type.md:36` (*Memory*): a capture whose
  `parse` loads a record runs that lookup for every request matching the route, before `allows`,
  signed in or not, and the timing paragraph of § *Standing decisions*.

### 4. Core\Router\Match holds each converted query value, so allows checks a record a query value's parse loaded

**Split:** none, one session.

- [ ] The failing cases first, at the paths the check `query-in-match` names:
  `tests/conformance/core/a-query-value-parsed-to-a-stored-object-reaches-allows.nvst:1` (the
  stage 3 `Order` as `#[Core\Query] Order $order`, `allows` reading it with
  `$route->parameter('order')`, and `refused` setting `404` for `BadCapture` and `Denied`),
  `tests/conformance/core/a-match-reads-a-query-value-by-its-name-only-inside-dispatch.nvst:1` (a
  query value inside `dispatch`, an absent `?T $x = null` key, a default, and `null` from
  `Core\Request::route()`) and
  `tests/conformance/reject/a-parameter-filled-by-a-capture-cannot-also-be-a-query-value.nvst:1`. Each
  run once by name and red.
- [ ] `crates/nvs-stdlib/src/router.rs:3043` to `:3061` (`dispatch`) and `:3086` (`query_values`):
  each bound query value is written into the match's parameters slot under its parameter's name
  before `dispatch_matched` (`router.rs:3182`) calls `allows`; a bad value is refused with the match
  as today. `router.rs:695` (`CAPTURE`) gains `bool`.
- [ ] `crates/nvs-stdlib/src/router.rs:2687` and `:2840`: the doc lines and the cards of `parameters`
  and `parameter` say they read query values inside `dispatch`, and `null` for one from
  `Core\Request::route()`.
- [ ] `crates/nvs-types/src/routes.rs:2810` (`check_captures`) or `:3192` (`query_params`): a
  `#[Core\Query]` parameter that a path capture names is a compile error at the parameter, with the
  new code and card in `crates/nvs-diagnostics/src/code/types_e07.rs`.
- [ ] `docs/rules/routing/a-bad-query-value-is-a-400.md:1`: the query values are bound into the match
  before `allows`, and `Core\Request::route()` reads `null` for them.
  `docs/rules/routing/matched-once-before-the-handler.md:1`: what the match holds.
  `docs/rules/routing/a-query-parameter-is-declared-like-a-capture.md:10`: a parameter is filled by a
  capture or by the query string, never both.

### 5. the ISO readers move into nvs-runtime, and the matcher converts Date, TimeOfDay and Instant captures and query values and DateTime query values

**Gate:** `docs/goals/time-values.md` is gone. **Split:** one `Explore` subagent lists every caller of
the `fromIso` grammar functions in `crates/nvs-stdlib/src/time.rs:3802`'s section (`iso_instant`,
`iso_date_prefix`, `iso_time_prefix`, `iso_datetime`, `validate_iso`), as `file.rs:NN` and one line
each, before the first edit.

- [ ] The failing cases first: the tests the check `time-captures` names, beside
  `crates/nvs-runtime/src/routes.rs:1388` (`a_uuid_capture_converts_and_refuses_what_is_not_one`), and
  the cases the check `time-routes` names. Each run once by name and red.
- [ ] `crates/nvs-runtime/src/iso.rs:1`: the four grammars, moved from
  `crates/nvs-stdlib/src/time.rs:3802` with their tests, returning `jiff` values; the module doc says
  why they live here, as `crates/nvs-runtime/src/uuid.rs:1` does. `time.rs` and `validate_iso`
  (`time.rs:3831`) call them, and nothing in `time.rs` reads ISO text another way.
- [ ] `crates/nvs-runtime/src/routes.rs:129`, `:217` and `:292`: `CaptureConv` and `Param` gain
  `Date`, `TimeOfDay`, `Instant` and `DateTime`; `convert` reads the segment as it arrived. The module
  doc at `:73` says a time capture is never decoded and why.
- [ ] `crates/nvs-types/src/commands.rs:1002`: the class arm of `converts_from_string` admits the four
  classes by name, from `nvs_stdlib::time`'s constants; `crates/nvs-types/src/routes.rs:2988`'s path
  half refuses `DateTime`, and the help at `routes.rs:2889` names `#[Core\Query]` and `Instant`.
  `crates/nvs-stdlib/src/router.rs:1342` (`native_conv`) and `crates/nvs-types/src/crossing.rs:129`
  (`capture_conv`) map the four names.
- [ ] `crates/nvs-stdlib/src/router.rs:2555` (`capture_value`): each new `Param` builds its object
  through `time.rs`'s own builders; `router.rs:695` (`CAPTURE`) gains `Date`, `TimeOfDay`,
  `Instant` and `DateTime` (the last as a query value, which a match holds since stage 4), and the
  doc lines at `router.rs:2687` and `:2840` follow.
- [ ] `docs/rules/security/route-capture-is-laundered-by-its-type.md:12`: the native conversions are
  `int`, `uint`, `decimal`, `Core\Uuid`, `Date`, `TimeOfDay` and `Instant`, and `DateTime` for a
  query value, with the door's `%2F` reason; the union the parameters cross as gains the four classes;
  the last four Declined rows of § *Standing decisions* that name this rule join it.
  `docs/rules/routing/a-query-parameter-is-declared-like-a-capture.md:10`: the type list gains the
  four classes and `bool`.

### 6. Core\Router::url, Core\Uri::buildQuery, the API document and a command's arguments take the time values

**Split:** none, one session.

- [ ] The failing cases first, at the paths the checks `time-links` and `time-document` name, each run
  once by name and red.
- [ ] `crates/nvs-stdlib/src/router.rs:1370` (`segment_text`): a time value is its ISO text, and
  `router.rs:1555` writes it unencoded; `crates/nvs-stdlib/src/router.rs:2241` (the computed-key
  check) converts it with the capture's own conversion.
- [ ] `crates/nvs-stdlib/src/uri.rs:2482` (`scalar_text`): a time value is its ISO text, a `DateTime`
  with its zone, form-encoded as every value is. The `Core\Uri::buildQuery` card says so.
- [ ] `crates/nvs-cli/src/openapi.rs:837` (`schema`): the formats of § *Standing decisions*, beside the
  `Core\Uuid` arm at `:854`.
- [ ] `crates/nvs-types/src/commands.rs:269` (`conversion_of`) and
  `crates/nvs-runtime/src/commands.rs:62` (`ArgConv`): the four arms, beside `Uuid` (`:82`), through
  `crates/nvs-runtime/src/iso.rs`; a word that does not convert is a usage error naming the form.
- [ ] `docs/rules/routing/link-target-and-params-are-checked.md:34`: a time value is written into its
  segment unencoded, and why that cannot leave the segment.
  `docs/rules/routing/a-leftover-link-key-is-a-query-string.md:1`: a time value in the query string
  is its ISO text. `docs/rules/tooling/commands-are-compiled.md:1`: an argument takes the time values
  through the shared roster.

### 7. Date, TimeOfDay, DateTime, Duration, Zone and Core\Uuid cross JSON as text

**Gate:** `docs/goals/time-values.md` is gone, and stage 5 is done. **Split:** none, one session.

- [ ] The failing cases first: the tests the check `json-text` names, beside `crates/nvs-stdlib/src/json.rs:9385`'s
  `Instant` test, and the cases the check `json-cases` names. Each run once by name and red.
  `tests/conformance/reject/a-json-derive-field-of-a-core-value-type-with-no-json-form-is-refused.nvst:1`
  is expected red: its fields move to `Core\Uri`, which still has no JSON form, and the commit says why.
- [ ] `crates/nvs-types/src/derive.rs:2711`: `is_core_json_value` becomes `JSON_TEXT_CLASSES` beside
  `DB_COLUMN_CLASSES` (`:2619`); `derive.rs:1439` erases every listed class to `CodecTy::ValueText`;
  `crates/nvs-runtime/src/object.rs:1161` renames the arm; the comment at `derive.rs:2694` follows.
- [ ] `crates/nvs-stdlib/src/json.rs:1944` (the encoder's `Instant` branch) and `:6346` (the decoder's):
  one table from class label to writer and reader, the seven rows of § *Standing decisions*, with
  `crates/nvs-runtime/src/iso.rs` for the four time grammars; `json.rs:6590` describes each wire form.
  `Duration`'s and `Zone`'s readers are the ones `Duration::parse` and `Zone`'s constructor use, and
  `Core\Uuid`'s is `crates/nvs-runtime/src/uuid.rs:44`.
- [ ] `crates/nvs-cli/src/openapi.rs:317`: a `CodecTy::ValueText` field's schema follows the class, with
  the formats § *Standing decisions* gives.
- [ ] The `E0756` card (`crates/nvs-diagnostics/src/code/types_e07.rs:1041`) and the `E0821` card
  (`crates/nvs-diagnostics/src/code/types_e08.rs:374`) name the `Core` classes that cross as text.
- [ ] `crates/nvs-stdlib/src/json.rs:150` (§ *A value type crosses as text*) names the seven classes and
  their texts. `docs/rules/core-classes/derive-property-list.md:26` names the `Core` value types that
  are reachable. `docs/rules/core-classes/db-column-types.md:1` does not change, and its list stays the
  row door's.

### 8. a Parses class writes itself with format, Core\Router::url calls it, and url checks the type of each value written in its call

**Split:** none for the code. Once the interface change has landed, one writing subagent adds
`format` to every `Parses` class in the tree, and the session builds when it ends.

- [ ] The failing cases first, at the paths the checks `parses-links` and `link-types` name, each run
  once by name and red.
- [ ] `crates/nvs-types/src/iter_lib.rs:134` (the seeded `Parses`): a third member, `format(): string`,
  required, not `static`. `PARSES`'s doc (`crates/nvs-hir/src/interfaces.rs:69`) and the docs at
  `crates/nvs-types/tests/routes.rs:77` and `crates/nvs-types/tests/commands.rs:25` say three members.
- [ ] `crates/nvs-stdlib/src/registry.rs:4717` (`implements_parses`) also needs a `format` with no
  parameters that returns `string`. `crates/nvs-stdlib/src/uuid.rs:214` and
  `crates/nvs-stdlib/src/queue.rs:2785` gain a `format` row beside `toString`, writing the same text.
  The test at `registry.rs:5828` pins that the roster is still those two classes.
- [ ] `crates/nvs-stdlib/src/router.rs:1370` (`segment_text`) and `crates/nvs-stdlib/src/uri.rs:2483`
  (`scalar_text`): an object whose class implements `Parses` is written as its `format()` text, called
  the way `stringify` (`crates/nvs-runtime/src/helpers.rs:2327`) calls `toString`, so both writers need
  the context it takes. Any other object throws, and the message names `Parses` and `format`.
- [ ] `crates/nvs-types/src/links.rs:482` (`literal_args`) records each value's static type, and a check
  beside `converts` (`links.rs:1057`) refuses a value its parameter cannot take, by the list in
  § *Standing decisions*, with the new code and its card.
- [ ] Every `Parses` class in the tree declares `format()`: the files
  `git grep -lE "implements[^{]*\bParses\b"` lists, about 26. Each `.out` that changes is read.
- [ ] `docs/rules/routing/link-target-and-params-are-checked.md:25`: a value is held to its
  parameter's type where its type is known, a `Parses` value is written as its `format` text, the
  sentence on what is not checked names what is left, and the three Declined rows of this stage.
  `docs/rules/expressions/try-parse.md:1`: `Parses` has `format` beside `parse`.
  `docs/rules/routing/a-leftover-link-key-is-a-query-string.md:1`: a `Parses` value in the query
  string is its `format` text.

### 9. stages 1 to 4 have their feature proofs, and the query example shows the string fallback

**Split:** after the session lists the owed proofs, two writing subagents in disjoint folders: one for
`docs/examples/lang/attributes/core-route-and-core-access-the-route-table/` and
`tests/hostile/lang/attributes/core-route-and-core-access-the-route-table/`, one for
`docs/examples/lang/classes/parses/` and `tests/hostile/lang/classes/parses/`. The session checks
each file against AGENTS.md § *Text an end user reads* and runs `bun nv proofs --comments` over them.

- [ ] `docs/examples/lang/attributes/core-route-and-core-access-the-route-table/07-a-query-parameter-and-a-bad-value.nvs:1`
  gains a second route with `#[Core\Query] string $page = ''` and `($page as ?uint) ?? 1`, and `?page=abc`
  shows page 1; its `.out` is regenerated and read. That folder's `about.md:1` and `details.md:1` say
  when to use it and that a typed value with a bad text is a `400`.
- [ ] `docs/examples/lang/attributes/core-route-and-core-access-the-route-table/08-a-query-value-that-may-be-missing.nvs:1`
  (`?uint $page = null` and a `?` `Parses` class) and `09-a-yes-or-no-query-value.nvs:1` (`bool`, `1`
  and `0`, and a checkbox with `value="1"`); `about.md` says both in one paragraph each.
- [ ] `docs/examples/lang/classes/parses/06-a-final-class-loads-a-stored-record.nvs:1`: a `final`
  `Order` whose `parse` loads a record, read once as a path capture and once as
  `#[Core\Query] Order $order`, `allows` checking it through `$route->parameter('order')` in both,
  and `refused` setting `404` for `BadCapture` and `Denied`. That folder's `about.md:1` says when the
  lookup runs, and that `allows` checks a query value as it checks a capture.
- [ ] `tests/hostile/lang/attributes/core-route-and-core-access-the-route-table/03-query-values-written-another-way.nvs:1`
  (`true`, `on`, an empty value and a bare key for a `bool`, and `page[]=1` for an optional value) and
  `tests/hostile/lang/classes/parses/02-a-visitor-guesses-ids.nvs:1` (a missing id and a forbidden id
  get the same `404`).
- [ ] `docs/examples/core/Router-Match/parameter/` and `parameters/` each gain an example of a query
  value read in `allows`, and `null` from `Core\Request::route()`.
- [ ] The rest of what `bun nv proofs --owed --goal route-values` names for stages 1 to 4, at the
  paths it prints, such as `lang:classes/methods-self-static-and-parent`'s example of `static` in a
  `final` class.

### 10. stages 5 to 8 have their feature proofs, and the website says what changed

**Split:** three writing subagents in disjoint folders: one for the route-table folder,
`docs/examples/core/Router/url/` and `docs/examples/core/Uri/buildQuery/` with the route-table hostile
folder; one for `docs/examples/lang/attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec/`,
`docs/examples/core/Json/` and `docs/examples/core/Arr/shapeAs/`; one for
`docs/examples/lang/attributes/core-api-and-the-openapi-document/` and
`docs/examples/lang/attributes/core-command-and-core-option-the-command-table/`. The session writes the
website.

- [ ] `docs/examples/lang/attributes/core-route-and-core-access-the-route-table/10-a-date-in-the-path-and-a-time-in-the-query.nvs:1`
  (a `Date` capture, an `Instant` capture, a `#[Core\Query] DateTime`, and the links
  `Core\Router::url` builds for them) and `about.md`'s paragraph on time values, including that a
  `DateTime` is a query value only.
- [ ] `docs/examples/core/Router/url/05-a-link-to-a-route-that-reads-a-class.nvs:1`: a `Parses` class
  `Isbn` with `parse` and `format`, a route that reads it, and the link `Core\Router::url` builds.
  `docs/examples/lang/classes/parses/about.md:1` says that `format` writes the text `parse` reads,
  and that a link uses it. The `Core\Router::url` reference card says so too.
- [ ] `tests/hostile/lang/classes/parses/03-a-format-that-writes-a-slash.nvs:1`: a `format` that
  returns `../admin?x=1` stays in its own segment, and the link does not reach another route.
- [ ] `tests/hostile/lang/attributes/core-route-and-core-access-the-route-table/04-time-values-written-another-way.nvs:1`:
  `%3A` for `:`, a tenth fraction digit, `[!Europe/Vienna]`, a `+` typed into a query value, a zone
  name of many thousand bytes and a `DateTime` with no `[Zone]`, each a `404` or a `400`.
- [ ] `docs/examples/lang/attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec/11-dates-durations-zones-and-ids-as-text.nvs:1`,
  that folder's `about.md:1` (the `Instant`-only sentence is rewritten), and the examples
  `bun nv proofs --owed --goal route-values` names for `Core\Json::encode`, `Core\Json::decodeAs`,
  `Core\Arr::shapeAs`, `Core\Router::url`, `Core\Uri::buildQuery`, the API document and the command
  table.
- [ ] `benches/members/lang/attributes/core-route-and-core-access-the-route-table.nvs:1` measures a
  request through a `Date` capture, and its growth row is rerecorded.
- [ ] `website/src/content/docs/syntax/attributes/routes.mdx:36`: an optional query value, a `bool`
  query value and the time values. `website/src/content/docs/syntax/attributes/json.mdx:9`: the value
  types that cross as text. `website/src/content/docs/tools/api-document/index.mdx:110`: an optional
  query value and the formats. `website/src/content/docs/syntax/attributes/commands-and-discovery.mdx:1`:
  an argument may be a time value. Each reread and stamped with `bun nv site --stamp`.
- [ ] `website/src/data/changes.json:1`: a `changed` entry for
  `lang:attributes/core-route-and-core-access-the-route-table` (optional query value, `bool` query
  value, time values), `Core\Router::url`, `Core\Uri::buildQuery`,
  `lang:attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec`,
  `Core\Json::encode`, `Core\Json::decodeAs`, `Core\Arr::shapeAs`, `lang:classes/parses` (every
  `Parses` class needs `format`),
  `Core\Router\Match::parameter` and `Core\Router\Match::parameters` (they read query values),
  `lang:attributes/core-api-and-the-openapi-document` and
  `lang:attributes/core-command-and-core-option-the-command-table`.
- [ ] `bun nv proofs --owed --goal route-values` names nothing.

### 11. the goal is verified once, and every failure is fixed

- [ ] The newcomer review of docs/agent/prose.md § *A newcomer reads it* over the pages stages 9 and 10 touched.
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stage 1 is done and its two checks are green (`optional-query`, `optional-query-document`).
`query_params` (`crates/nvs-types/src/routes.rs:3178`) strips the `null` of a `?T $x = null` and
records `T`; `report_nullable_query` beside it is the `E0752` for `?T` with no default or another
default. `nvs_runtime::routes::QueryDefault` (`Required`, `Null`, `Given`) replaced
`QueryParam::default`'s `Option<Param>`. The API document needed no change, because the row's `ty` is
`T` already (fixture `crates/nvs-cli/tests/fixtures/api/nullable-query.nvs`).

**A pre-existing bug was fixed on the way:** an enum path capture or enum query value never reached
its route method through `dispatch`. The match converts it to a plain `int` or `uint`, and the
method's parameter check refused that ("must be of type enum, uint given"). No case dispatched one.
`as_case` (`crates/nvs-stdlib/src/router.rs:3187`) now gives the value the enum's tag where
`call_handler` binds it; the match keeps the plain integer `CAPTURE` declares. Pinned by
`tests/conformance/core/an-enum-capture-reaches-its-route-method-as-a-case.nvst`. Stage 4 puts query
values into the match: it must decide whether an enum query value there is the plain integer, as an
enum capture is, and keep the two the same.

Expected red once later stages land:
`tests/conformance/reject/a-json-derive-field-of-a-core-value-type-with-no-json-form-is-refused.nvst`
(stage 7 moves it to `Core\Uri`), and `tests/conformance/reject/a-bool-route-capture-does-not-compile.nvst`
(stage 2: its `#[Query] bool $full` error goes away).

### Next

Stage 2, files `crates/nvs-types/src/routes.rs`, `crates/nvs-types/src/crossing.rs`,
`crates/nvs-runtime/src/routes.rs`, `crates/nvs-stdlib/src/router.rs` and
`crates/nvs-cli/src/openapi.rs`; no gate. Stage 3 may run beside it, and stage 4 follows stages 2
and 3.

- [ ] `tests/conformance/core/a-bool-query-value-is-1-or-0.nvst:1`: the failing case first, run once
  by name; `tests/conformance/reject/a-bool-route-capture-does-not-compile.nvst:1` loses its
  `#[Query] bool` route and gains a `false` capture.
- [ ] `crates/nvs-types/src/routes.rs:2988` (`converts_from_segment`): a path half and a query half;
  the query help at `query_params` (`:3178`) lists `bool`.
- [ ] `crates/nvs-runtime/src/routes.rs:129` (`CaptureConv`), `:217` (`convert`) and `:306` (`Param`):
  `CaptureConv::Bool(Option<bool>)` and `Param::Bool(bool)`; `crates/nvs-types/src/crossing.rs:129`
  (`capture_conv`); `crates/nvs-stdlib/src/router.rs:2555` (`capture_value`). The tests the check
  `bool-query-runtime` names, run once by name with `cargo test --lib a_bool_query_value_`.
- [ ] `crates/nvs-cli/src/openapi.rs:837` (`schema`): `{"type": "integer", "enum": [0, 1]}`, and the
  test `a_bool_query_value_is_the_integer_0_or_1` in `crates/nvs-cli/tests/openapi.rs`.
- [ ] `docs/rules/security/route-capture-is-laundered-by-its-type.md:6` and
  `docs/rules/routing/a-bad-query-value-is-a-400.md:1`, with the two Declined rows.

### Backlog

- `Core\Router::url` writes a `/` in a capture value as `%2F` (`crates/nvs-stdlib/src/router.rs:4176`'s
  test), a link the door answers `400` to (`crates/nvs-server/src/mount.rs:738`). Owner:
  `rule:routing/link-target-and-params-are-checked`; not this goal's work.
