---
looper: 1
title: no object enters a program with a class it may not hold
summary: "Reached when Core\\Serialize::decode refuses every class that encode refuses, every live crossing asks one receiving side, a published topic message reaches each subscriber with a class of the subscriber's own program, and topic names belong to one application."
context: [crates/nvs-runtime/src/graph.rs, docs/rules/classes/graph-copy.md, docs/rules/security/isolate-values-cross-by-copy.md]
stages:
  - id: 1
    title: decode refuses what encode refuses
    status: open
    context: [crates/nvs-runtime/src/object.rs, crates/nvs-stdlib/src/serialize.rs, crates/nvs-stdlib/src/instance.rs, docs/rules/classes/serialize-is-a-closed-format.md, tests/hostile/README.md]
  - id: 2
    title: "every live crossing asks one receiving side, and a test lists the crossings"
    status: open
    context: [crates/nvs-host/src/isolate.rs, crates/nvs-host/src/placed.rs, crates/nvs-runtime/src/ctx/error.rs, crates/nvs-runtime/src/dispatch.rs]
  - id: 3
    title: "a published value reaches each subscriber as bytes it decodes into its own classes"
    status: open
    context: [crates/nvs-stdlib/src/topic.rs, crates/nvs-stdlib/src/bus.rs, crates/nvs-runtime/src/peer.rs, crates/nvs-stdlib/src/socket.rs, crates/nvs-stdlib/src/sse.rs, docs/rules/core-classes/topic.md]
  - id: 4
    title: a topic name belongs to one application
    status: open
    context: [crates/nvs-stdlib/src/topic.rs, crates/nvs-stdlib/src/bus.rs, crates/nvs-stdlib/src/cache.rs, crates/nvs-cli/src/events.rs, docs/rules/core-classes/topic.md]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: decode-refuses-what-encode-refuses
    stage: 1
    name: "decode refuses a callable's class, a generator's state class, a host-handle class, a compiler-made class and a class with a secret property, before it makes an object"
    run: [cargo, test, --lib, --, decode_refuses_every_class_encode_refuses, a_refused_class_is_refused_before_its_properties_are_compared, a_class_the_compiler_made_does_not_cross_and_a_shape_does, every_core_class_that_holds_a_host_handle_is_refused_by_decode]
    timeout: 1800
    tests:
      - decode_refuses_every_class_encode_refuses
      - a_refused_class_is_refused_before_its_properties_are_compared
      - a_class_the_compiler_made_does_not_cross_and_a_shape_does
      - every_core_class_that_holds_a_host_handle_is_refused_by_decode
  - id: every-door-one-definition
    stage: 2
    name: "every public function of the graph module refuses the same classes, and a Core value crosses a live boundary"
    run: [cargo, test, --lib, --, every_door_of_the_graph_module_refuses_the_same_classes, a_core_value_crosses_a_live_boundary_into_any_program]
    timeout: 1800
    tests:
      - every_door_of_the_graph_module_refuses_the_same_classes
      - a_core_value_crosses_a_live_boundary_into_any_program
  - id: every-crossing-site
    stage: 3
    name: "every call of a graph door in the tree names its receiving side"
    run: [cargo, test, --test, crossings]
    timeout: 1800
    tests:
      - every_crossing_site_names_its_receiving_side
  - id: topic-delivers-into-the-subscribers-classes
    stage: 3
    name: "a published object arrives as the subscriber's own class, a subscriber that lacks the class gets an error, and a publish encodes once"
    run: [cargo, test, --lib, --, a_published_object_arrives_as_the_subscribers_own_class, a_subscriber_that_lacks_the_class_catches_an_error_and_reads_the_next_message, a_message_one_subscriber_cannot_read_still_reaches_the_others, a_publish_encodes_once_however_many_subscribed]
    timeout: 1800
    tests:
      - a_published_object_arrives_as_the_subscribers_own_class
      - a_subscriber_that_lacks_the_class_catches_an_error_and_reads_the_next_message
      - a_message_one_subscriber_cannot_read_still_reaches_the_others
      - a_publish_encodes_once_however_many_subscribed
  - id: topic-names-per-application
    stage: 4
    name: "two applications in one process that use one topic name do not reach each other, on one core or across cores"
    run: [cargo, test, --lib, --, two_applications_with_one_topic_name_do_not_reach_each_other, a_topic_reaches_its_application_across_a_reload, a_value_published_on_another_core_reaches_only_its_application]
    timeout: 1800
    tests:
      - two_applications_with_one_topic_name_do_not_reach_each_other
      - a_topic_reaches_its_application_across_a_reload
      - a_value_published_on_another_core_reaches_only_its_application
---

# no object enters a program with a class it may not hold

## Why

**`Core\Serialize::decode` builds what `encode` refuses.** The walk refuses a callable, a generator
and an object holding a host handle (`refusable`, `crates/nvs-runtime/src/graph.rs:444`). Its one
caller is the walk (`crates/nvs-runtime/src/graph.rs:365`). `Reader::object`
(`crates/nvs-runtime/src/graph.rs:1056`) never calls it. Its resolver (`Ctx::class_desc`,
`crates/nvs-runtime/src/ctx/error.rs:328`) finds every class of the unit's table by name, compiler-made
ones included. Those classes carry no field types, so `check_fields` admits any value.

- A payload naming a generator's state class (`{name}$gen`, `crates/nvs-ir/src/lower/generator.rs:472`)
  decodes. Compiled code then reads a slot at the type the body was compiled with. A run ended the
  process with a segmentation fault. A closure's class (`crates/nvs-types/src/expr/calls.rs:2854`) and
  a first-class callable's (`crates/nvs-ir/src/lower/expr.rs:3446`) resolve the same way.
- A payload naming a host-handle class (`crates/nvs-stdlib/src/instance.rs:143`) with a chosen key
  decodes. A run read a file the code was never given.

`decode` needs no grant, and bytes a program builds are not tainted. So a package can do this, and so
can a peer whose bytes the application asserted trusted. Commits `5f9c423e11` and `4b8503ca80` added
type checks and frees to this path. Neither decides which classes may be decoded at all.

**`Core\Topic::publish` copies with no receiving class table.** `cross`
(`crates/nvs-stdlib/src/topic.rs:645`) calls `copy_graph`, which is `copy_graph_into(value, None)`
(`crates/nvs-runtime/src/graph.rs:636`). `Live::admit` then admits every class
(`crates/nvs-runtime/src/graph.rs:505`). The copy keeps the publisher's descriptor and reaches the
subscriber unchanged (`crates/nvs-stdlib/src/socket.rs:1069`). A subscriber is a connection script,
which is its own compiled unit. `spawn` and `upgrade` guard this same crossing
(`crates/nvs-host/src/isolate.rs:724`, `crates/nvs-stdlib/src/socket.rs:1008`). The subscriber's
`mixed` call runs the publisher unit's code, and nothing keeps that unit's class table alive for the
object. The path to a freed descriptor after a deploy is traced, not run. The other-core path decodes
with the draining connection's table for every subscriber (`crates/nvs-stdlib/src/topic.rs:696`). It
drops the message for all of them when that one lacks the class or is at its memory limit
(`crates/nvs-stdlib/src/topic.rs:711`, `crates/nvs-stdlib/src/topic.rs:708`).

**Topic names are one namespace for the process.** The table and the hand-off between cores use the
bare name (`crates/nvs-stdlib/src/topic.rs:395`, `crates/nvs-stdlib/src/bus.rs:322`). The memory cache
prefixes the application (`scoped`, `crates/nvs-stdlib/src/cache.rs:1553`). Two `[[app]]` blocks in
one `nvs serve` that both use one name read each other's messages.

**The live carrier refuses every `Core` value.** Its resolver asks the program's table alone
(`crates/nvs-runtime/src/graph.rs:677`, `crates/nvs-host/src/isolate.rs:1539`,
`crates/nvs-host/src/placed.rs:387`). `decode` at the cache and at `Core\Serialize` asks `Core` second.
So `spawn script` with a `Core\Uri` in `args:` throws "is not a class the receiving side declares",
seen on the release binary. It fails closed. It is the same question with two answers.

## Standing decisions

**One function says whether a class may cross, and the walk and the decoder both call it.**
`refusable` becomes that function. It refuses a callable's class, a generator's state class, a class
that holds a host handle, a class with a `secret` property and every other class the compiler made.
`Reader::object` calls it after the class resolves and before it compares a property or makes an
object. So a refused payload builds nothing, and its error does not print the class's layout.
**What it spends:** a few flag reads and one scan of the class's `secret` flags per decoded object.
No allocation, nothing per request. Safe default: where one of the five is awkward to move into the
function, the decoder still refuses it at the same place. Declined: a grant for `decode`, which
`rule:classes/serialize-is-a-closed-format` already declines.

**A class the compiler made is known by `$` in its name, and a shape is the one that crosses.** No
declared class can hold `$`, and the generator and shape tests already rest on that
(`crates/nvs-runtime/src/object.rs:1452`, `crates/nvs-runtime/src/object.rs:1466`). A `$shape{…}` class
is an anonymous object and is data. Every other `$` name is refused. A new compiler-made class is
therefore refused until somebody names it as data. **What it spends:** one byte scan of the name,
made once when the class is defined and kept as a flag. Declined: a list of suffixes, which a new
lowering would have to remember.

**The `secret` refusal moves into the same function.** The walk refuses a `secret` property field by
field today (`crates/nvs-runtime/src/graph.rs:373`). `encode` can never write such a class, so `decode`
refuses one too. The function names the first `secret` property, as the walk's message does.

**The receiving side is one resolver: the program's table first, then `Core`.** That is the order
`Ctx::class_desc` uses. Every live crossing and every decode takes its resolver from one function,
given the receiving program's table. So a `Core` value crosses at `spawn`, at `await` and at
`upgrade`, as it does through the cache. **What it spends:** one more name lookup per object at a
live crossing, and only when the program's table misses. Usability gains: a `Core\Uri` in `args:`
works. Declined: resolving a program's class by name at the live carrier. That carrier keeps the
descriptor it was built with, so the same class means the same address
(`crates/nvs-runtime/src/graph.rs:504`). A copy that must change class is the bytes carrier.

**No live copy is made without a receiving side.** The table-less `copy_graph` leaves the public API.
`copy_graph_to` with no table means a receiving side that declares no program class, which is how
the out crossing already reads it (`crates/nvs-host/src/isolate.rs:1537`). `Fixtures::copy`
(`crates/nvs-runtime/src/dispatch.rs:1092`) takes the test context's table, as its `decode` sibling
does. Safe default: a Rust test that hands a native program an object passes that object's table.

**The crossings are these, and a test finds them in the code.** They are `Core\Serialize::encode` and
`decode`, the two cache tiers, the session record, `spawn` in and out on one core, `spawn` placed on
another core, both `upgrade` members, the test fixtures and `Core\Topic::publish`. Each calls a
public function of `crates/nvs-runtime/src/graph.rs`. `Core\Queue::push` is not one: it writes `$args`
as JSON (`crates/nvs-stdlib/src/queue.rs:3442`), which names no class. Two tests hold this. One reads
the graph module's own `pub fn` lines and drives every refused class through each. The other reads
every shipped file under `crates/*/src` and fails on a call that names no receiving side.

**A topic carries bytes on every path, and each subscriber decodes them itself.** `publish` encodes
the value once, before the walk and whether or not anyone joined. That one encoding is the refusal a
publish makes. One shared buffer goes into each local subscriber's queue and into the hand-off to
other cores. `receive()` decodes it with the subscriber's own resolver. So an object arrives as the
class of that name in the subscriber's program, checked as `Core\Serialize::decode` checks it. **What
it spends:** the publisher pays one encode per publish where it pays one live copy per local
subscriber today. Each subscriber pays one decode on its own turn, CPU time and memory balance. The
buffer is the process's bytes, held once until the last queue drops it. Each queue counts the
buffer's full length against its subscriber's limit, so a shared buffer is counted once per queue.
That over-counts, which is the safe direction. Simpler: one carrier where there are two, and both
message-loss exits of the drain go. Declined: a live copy per subscriber with that subscriber's
table. A program's class would then reach a subscriber on another core and never one on the same
core. Safe default if the queue cannot hold bytes: `cross` takes a resolver that answers `Core`
classes only, so `publish` throws a `LogicError` for an object of a program's class.

**A subscriber that cannot decode a message gets an error it can catch.** `spawn` and `upgrade`
refuse such an object by name and never stub it (`rule:classes/graph-copy`). A publisher has
returned by then, so the subscriber's `receive()` throws a `ParseError` naming the topic and the
class. That is the error `Core\Cache\Store::get` throws for an entry it cannot decode
(`crates/nvs-stdlib/src/cache.rs:2766`). The message is taken off the queue, so the next `receive()`
reads the next one. Other subscribers are not affected. After a deploy, a connection that kept its
old unit ends on the first message whose class changed, unless it catches. That is loud, and the
client reconnects into the new unit. Declined: dropping the message silently, which leaves an old
connection open and missing messages.

**A topic name is scoped by the application, with the identity the memory cache uses.** The key is
`application(ctx)` (`crates/nvs-stdlib/src/cache.rs:1580`), a NUL byte, then the name. It keys the
subscriber table, the counts each core reports and the envelope. The generation is left out, because
a connection outlives a reload (`rule:concurrency/a-connection-keeps-its-compiled-unit`). The program
still reads the bare name in `Message->topic`. `NAME_CAP` still bounds the bare name. No new setting.
**What it spends:** one key build per member call, the block's path plus the name, and that many more
bytes per table row. `publish_text` (`crates/nvs-stdlib/src/topic.rs:743`) takes the application of
the program it feeds, so `nvs run --events` reaches a program that has an `[[app]]` block.

**`Core` slots stay unchecked by `decode` in this goal.** A `Core` class is defined with slot names
alone (`crates/nvs-stdlib/src/instance.rs:584`), and each member checks the slot it reads. The
Backlog names the sweep that would prove it.

**Lanes:** one lane. Stages 1 and 2 both change `graph.rs`. Stage 3 removes the call stage 2's site
test would fail on, and stages 3 and 4 both change `topic.rs` and `bus.rs`. They run in order.

## Stages

### 1. decode refuses what encode refuses

**Does:** make one function answer whether a class may cross, and call it from the decoder before
any object is made.

- [ ] `ClassDesc` gains one flag for a compiler-made class: a `$` in the name and not a shape. It is
  set when the class is defined (`rule:classes/graph-copy`, `crates/nvs-runtime/src/object.rs:1452`,
  `crates/nvs-runtime/src/object.rs:2108`).
- [ ] `refusable` becomes the one function. It also refuses a compiler-made class and a class with a
  `secret` property, naming the property. The walk's own `secret` loop goes
  (`rule:classes/graph-copy`, `crates/nvs-runtime/src/graph.rs:444`,
  `crates/nvs-runtime/src/graph.rs:373`).
- [ ] `Reader::object` calls it after the resolve and before the property comparison
  (`rule:classes/serialize-is-a-closed-format`, `crates/nvs-runtime/src/graph.rs:1067`,
  `crates/nvs-runtime/src/graph.rs:1079`).
- [ ] Test `decode_refuses_every_class_encode_refuses` in `graph.rs` tests. One table holds a class of
  each refused kind and one plain class. For each, `encode` of an instance and `decode` of a payload
  naming it give the same sentence, and `decode` builds no object
  (`crates/nvs-runtime/src/graph.rs:1247`).
- [ ] Test `a_refused_class_is_refused_before_its_properties_are_compared`: a payload naming a
  host-handle class with a wrong property count gets the class refusal
  (`crates/nvs-runtime/src/graph.rs:1247`).
- [ ] Test `a_class_the_compiler_made_does_not_cross_and_a_shape_does`: a `$gen`, a `$fn0` and a `$fcc0`
  name are refused by both carriers, and a `$shape{n}` instance round-trips
  (`crates/nvs-runtime/src/graph.rs:1247`).
- [ ] Test `every_core_class_that_holds_a_host_handle_is_refused_by_decode` in `instance.rs` tests. It
  walks the process's `Core` descriptors, not a list in the test. For each one the function refuses,
  a payload with the class's own slot names is refused with that sentence
  (`crates/nvs-stdlib/src/instance.rs:544`, `crates/nvs-stdlib/src/instance.rs:143`).
- [ ] `.nvst` case `serialize-decode-refuses-a-class-the-compiler-made`. The program builds `bytes`
  that name its own generator's state class, its own closure's class and `Core\IO\File`. Each
  `decode` throws a `ParseError`
  (`tests/conformance/core/serialize-decode-refuses-a-class-the-compiler-made.nvst:1`).
- [ ] Attack `02-bytes-that-name-a-class-encode-refuses.nvs`. Its payloads name a generator's state
  class with a wrong-kind slot, a host-handle class with a chosen key, and a callable's class. Every
  step should throw (`rule:testing/feature-proofs`,
  `tests/hostile/core/Serialize/decode/02-bytes-that-name-a-class-encode-refuses.nvs:1`).
- [ ] The fuzz check's class table gains a class of each refused kind, and it fails when `decode`
  returns an object of one (`crates/nvs-fuzz-checks/src/serialize.rs:1`).
- [ ] `rule:classes/serialize-is-a-closed-format` says `decode` refuses every class `encode` refuses,
  before any object is made, with the cost line. `rule:classes/graph-copy` lists the five refusals
  and says one function is called by the walk and by the decoder
  (`docs/rules/classes/serialize-is-a-closed-format.md:1`, `docs/rules/classes/graph-copy.md:7`).
- [ ] The `decode` card's `ParseError` row and the `decode` `about.md` say, in one plain sentence,
  that bytes naming a generator, a function value or an open resource are not read
  (`crates/nvs-stdlib/src/serialize.rs:123`, `docs/examples/core/Serialize/decode/about.md:1`,
  `docs/reference/core/Serialize.md:1`).

### 2. every live crossing asks one receiving side, and a test lists the crossings

**Does:** give every crossing one resolver for the receiving side, remove the live copy that has
none, and pin both with tests that read the code.

- [ ] One function builds the receiving resolver from a program's table: that table first, then the
  `Core` descriptors, as `Ctx::class_desc` does (`rule:security/isolate-values-cross-by-copy`,
  `crates/nvs-runtime/src/ctx/error.rs:328`, `crates/nvs-runtime/src/graph.rs:167`).
- [ ] Every site takes its resolver from it: `copy_graph_to` (`crates/nvs-runtime/src/graph.rs:670`),
  the out crossing (`crates/nvs-host/src/isolate.rs:1539`), the placed spawn's two decodes
  (`crates/nvs-host/src/placed.rs:387`, `crates/nvs-host/src/placed.rs:264`,
  `crates/nvs-host/src/placed.rs:347`) and the fixtures (`crates/nvs-runtime/src/dispatch.rs:1168`).
- [ ] The table-less `copy_graph` leaves the public API, and `copy_graph_to` with no table is a
  receiving side with no program class. `Fixtures::copy` takes a table and its callers pass one
  (`crates/nvs-runtime/src/graph.rs:636`, `crates/nvs-runtime/src/graph.rs:674`,
  `crates/nvs-host/src/isolate.rs:719`, `crates/nvs-runtime/src/dispatch.rs:1107`). The call in
  `crates/nvs-stdlib/src/topic.rs:646` stays until stage 3 and is the one site the new test names.
- [ ] Test `every_door_of_the_graph_module_refuses_the_same_classes` in `graph.rs` tests. It reads the
  file's shipped half for `pub fn` names and fails naming one it has no driver for. Each driver puts
  every refused kind through that door (`crates/nvs-runtime/src/graph.rs:1247`; the scan's shape is
  `crates/nvs-stdlib/src/cache.rs:3119`).
- [ ] Test `a_core_value_crosses_a_live_boundary_into_any_program`: a `Core` instance crosses
  `copy_graph_to` into a table that does not declare it, and a program's object still does not
  (`crates/nvs-runtime/src/graph.rs:1247`).
- [ ] Test `every_crossing_site_names_its_receiving_side` in a new integration test. It takes the door
  names from `graph.rs`, finds every shipped call under `crates/*/src`, and fails naming a call with
  no receiving side or with a resolver written at the site (`crates/nvs-stdlib/tests/crossings.rs:1`).
  It is red until stage 3 removes `topic.rs:646`, so its check `every-crossing-site` belongs to
  stage 3, and the handoff says so.
- [ ] `.nvst` case: a `Core\Uri` in `args:` reaches a `spawn script` method entry and comes back in
  its answer (`tests/conformance/isolate/a-core-value-crosses-into-a-spawned-script-and-back.nvst:1`).
- [ ] `rule:security/isolate-values-cross-by-copy` says the receiving side resolves its own classes
  and every `Core` class. `rule:classes/graph-copy` lists the crossings and says `Core\Queue::push`
  carries JSON (`docs/rules/security/isolate-values-cross-by-copy.md:9`,
  `docs/rules/classes/graph-copy.md:13`).

### 3. a published value reaches each subscriber as bytes it decodes into its own classes

**Does:** make the bus carry one encoding per publish on every path, and decode it at the
subscriber's own `receive()`.

- [ ] `Delivery` holds the shared encoded payload, not a `Value`, and needs no context to drop. The
  context's teardown and `take_delivery` follow (`rule:core-classes/topic`,
  `crates/nvs-runtime/src/peer.rs:109`, `crates/nvs-runtime/src/ctx/mod.rs:1774`,
  `crates/nvs-runtime/src/ctx/isolate.rs:211`).
- [ ] `publish` encodes once before the walk and queues the one buffer to each local subscriber and
  to `hand_off`. `cross` and the `copy_graph` import go. Each queue is asked for room with the
  buffer's length (`rule:core-classes/topic`, `crates/nvs-stdlib/src/topic.rs:794`,
  `crates/nvs-stdlib/src/topic.rs:645`, `crates/nvs-stdlib/src/topic.rs:666`,
  `crates/nvs-stdlib/src/bus.rs:299`).
- [ ] `deliver_from_other_cores` moves each envelope into the local queues and decodes nothing. Its
  `return` on the budget and its `break` on a class go (`crates/nvs-stdlib/src/topic.rs:689`,
  `crates/nvs-stdlib/src/topic.rs:708`, `crates/nvs-stdlib/src/topic.rs:711`).
- [ ] Both doors' `receive()` decode the payload with the connection's own resolver. A refused
  payload throws a `ParseError` naming the topic and the class, and the message is off the queue
  (`rule:classes/graph-copy`, `crates/nvs-stdlib/src/socket.rs:1069`,
  `crates/nvs-stdlib/src/socket.rs:1197`, `crates/nvs-stdlib/src/sse.rs:801`,
  `crates/nvs-stdlib/src/sse.rs:855`).
- [ ] The existing tests that build a `Delivery` from a `Value` follow
  (`crates/nvs-stdlib/src/topic.rs:1106`, `crates/nvs-stdlib/src/topic.rs:1206`,
  `crates/nvs-stdlib/src/socket.rs:1445`, `crates/nvs-stdlib/src/socket.rs:1670`,
  `crates/nvs-stdlib/src/sse.rs:1518`).
- [ ] Test `a_published_object_arrives_as_the_subscribers_own_class`: two class tables declare one
  name, and the subscriber's object has the subscriber table's descriptor
  (`crates/nvs-stdlib/src/topic.rs:870`).
- [ ] Test `a_subscriber_that_lacks_the_class_catches_an_error_and_reads_the_next_message`
  (`crates/nvs-stdlib/src/topic.rs:870`).
- [ ] Test `a_message_one_subscriber_cannot_read_still_reaches_the_others`, on the local path and
  through the drain (`crates/nvs-stdlib/src/topic.rs:870`).
- [ ] Test `a_publish_encodes_once_however_many_subscribed`: a counter in the test reads the same
  number of encodes and payload allocations for two subscribers and for eight
  (`crates/nvs-stdlib/src/topic.rs:870`).
- [ ] Attack `03-an-object-a-generator-and-an-open-file.nvs`, with its `.nvsp`. A connection joins a
  topic and publishes an object of its own class, a generator, a closure and an open file. The
  object should arrive as its own class and the other three should throw
  (`rule:testing/feature-proofs`,
  `tests/hostile/core/Topic/publish/03-an-object-a-generator-and-an-open-file.nvs:1`).
- [ ] The module docs of `topic.rs` and `bus.rs` are rewritten for one carrier
  (`crates/nvs-stdlib/src/topic.rs:123`, `crates/nvs-stdlib/src/topic.rs:142`,
  `crates/nvs-stdlib/src/bus.rs:1`).
- [ ] `rule:core-classes/topic` says a value is encoded once, each subscriber decodes it into its own
  classes, and what a subscriber that cannot gets, with the cost line
  (`docs/rules/core-classes/topic.md:4`).
- [ ] The `publish` card and its `about.md` say an object arrives as the subscriber's class of the
  same name. The `receive` cards of both doors name the `ParseError`
  (`crates/nvs-stdlib/src/topic.rs:352`, `docs/examples/core/Topic/publish/about.md:1`,
  `crates/nvs-stdlib/src/socket.rs:1190`, `crates/nvs-stdlib/src/sse.rs:841`).

### 4. a topic name belongs to one application

**Does:** key every topic by its application, on one core and across cores, with the identity the
memory cache uses.

- [ ] One function builds the key from `application(ctx)`, a NUL byte and the checked name. All three
  members use it for the table, the reported counts and the hand-off
  (`rule:core-classes/topic`, `crates/nvs-stdlib/src/cache.rs:1580`,
  `crates/nvs-stdlib/src/topic.rs:412`, `crates/nvs-stdlib/src/topic.rs:485`,
  `crates/nvs-stdlib/src/topic.rs:570`, `crates/nvs-stdlib/src/topic.rs:607`,
  `crates/nvs-stdlib/src/bus.rs:258`, `crates/nvs-stdlib/src/bus.rs:279`).
- [ ] A delivery carries the bare name to the program, on the local path and from the envelope
  (`crates/nvs-stdlib/src/topic.rs:834`, `crates/nvs-stdlib/src/topic.rs:717`,
  `crates/nvs-stdlib/src/bus.rs:112`).
- [ ] `publish_text` takes the application of the program it feeds, and both `--events` paths pass it
  (`crates/nvs-stdlib/src/topic.rs:743`, `crates/nvs-stdlib/src/lib.rs:290`,
  `crates/nvs-cli/src/events.rs:203`, `crates/nvs-cli/src/main.rs:2202`).
- [ ] Test `two_applications_with_one_topic_name_do_not_reach_each_other`: two contexts under two
  `[[app]]` blocks, one name, and a publish from each reaches its own subscriber alone
  (`crates/nvs-stdlib/src/topic.rs:870`).
- [ ] Test `a_topic_reaches_its_application_across_a_reload`: a subscriber of one generation receives
  what a publisher of the next generation of the same block publishes
  (`crates/nvs-stdlib/src/topic.rs:870`).
- [ ] Test `a_value_published_on_another_core_reaches_only_its_application`, through `hand_off` and
  the drain (`crates/nvs-stdlib/src/topic.rs:870`, `crates/nvs-stdlib/src/bus.rs:452`).
- [ ] The table's ceiling paragraph in the module doc states the new byte bound
  (`crates/nvs-stdlib/src/topic.rs:58`).
- [ ] `rule:core-classes/topic` says a topic name belongs to one application and what the key costs
  (`docs/rules/core-classes/topic.md:13`).
- [ ] The `subscribe` `about.md` says, in one sentence, that two applications on one server do not
  share topics (`docs/examples/core/Topic/subscribe/about.md:1`).

### 5. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Nothing is built yet. `Reader::object` builds any class the program can resolve by name,
`Core\Topic::publish` copies with no receiving table, and the topic table is keyed by the bare name.
`spawn script` refuses a `Core` value in `args:`.

### Next

Stage 1, decode refuses what encode refuses. Files: `crates/nvs-runtime/src/graph.rs`,
`crates/nvs-runtime/src/object.rs`, `crates/nvs-stdlib/src/instance.rs`,
`crates/nvs-stdlib/src/serialize.rs`, and the two rule fragments.

- [ ] `crates/nvs-runtime/src/object.rs:1452`: `ClassDesc` gains one flag for a compiler-made class.
  It is a `$` in the name and not a shape, set when the class is defined
  (`rule:classes/graph-copy`, `crates/nvs-runtime/src/object.rs:2108`).
- [ ] `crates/nvs-runtime/src/graph.rs:444`: `refusable` becomes the one function. It also refuses a
  compiler-made class and a class with a `secret` property, naming the property. The walk's own
  `secret` loop goes (`rule:classes/graph-copy`, `crates/nvs-runtime/src/graph.rs:373`).
- [ ] `crates/nvs-runtime/src/graph.rs:1067`: `Reader::object` calls it after the resolve and before
  the property comparison (`rule:classes/serialize-is-a-closed-format`,
  `crates/nvs-runtime/src/graph.rs:1079`).
- [ ] `crates/nvs-runtime/src/graph.rs:1247`: test `decode_refuses_every_class_encode_refuses`. One
  table holds a class of each refused kind and one plain class. For each, `encode` of an instance
  and `decode` of a payload naming it give the same sentence, and `decode` builds no object.
- [ ] `crates/nvs-runtime/src/graph.rs:1247`: test
  `a_refused_class_is_refused_before_its_properties_are_compared`. A payload naming a host-handle
  class with a wrong property count gets the class refusal.
- [ ] `crates/nvs-runtime/src/graph.rs:1247`: test
  `a_class_the_compiler_made_does_not_cross_and_a_shape_does`. A `$gen`, a `$fn0` and a `$fcc0` name
  are refused by both carriers, and a `$shape{n}` instance round-trips.
- [ ] `crates/nvs-stdlib/src/instance.rs:544`: test
  `every_core_class_that_holds_a_host_handle_is_refused_by_decode`. It walks the process's `Core`
  descriptors, not a list in the test. For each one the function refuses, a payload with the
  class's own slot names is refused with that sentence (`crates/nvs-stdlib/src/instance.rs:143`).
- [ ] `tests/conformance/core/serialize-decode-refuses-a-class-the-compiler-made.nvst:1`: a new case.
  The program builds `bytes` that name its own generator's state class, its own closure's class and
  `Core\IO\File`. Each `decode` throws a `ParseError`.
- [ ] `tests/hostile/core/Serialize/decode/02-bytes-that-name-a-class-encode-refuses.nvs:1`: a new
  attack. Its payloads name a generator's state class with a wrong-kind slot, a host-handle class
  with a chosen key, and a callable's class. Every step should throw
  (`rule:testing/feature-proofs`).
- [ ] `crates/nvs-fuzz-checks/src/serialize.rs:1`: the fuzz check's class table gains a class of each
  refused kind, and it fails when `decode` returns an object of one.
- [ ] `docs/rules/classes/serialize-is-a-closed-format.md:1`: the rule says `decode` refuses every
  class `encode` refuses, before any object is made, with the cost line. `rule:classes/graph-copy`
  lists the five refusals and says one function is called by the walk and by the decoder
  (`docs/rules/classes/graph-copy.md:7`).
- [ ] `crates/nvs-stdlib/src/serialize.rs:123`: the `decode` card's `ParseError` row and the `decode`
  `about.md` say, in one plain sentence, that bytes naming a generator, a function value or an open
  resource are not read (`docs/examples/core/Serialize/decode/about.md:1`,
  `docs/reference/core/Serialize.md:1`).

No existing test is expected to go red in stage 1.

### Backlog

- `decode` does not type-check a `Core` class's slots (`crates/nvs-stdlib/src/instance.rs:584`). A
  sweep that decodes every `Core` class with wrong-kind slots and calls its members would prove it.
- `ClassTable::id_of` is a linear scan, and `decode` calls it once per object
  (`crates/nvs-runtime/src/object.rs:2878`).
