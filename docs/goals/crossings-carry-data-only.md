---
looper: 1
title: no object enters a program with a class it may not hold
summary: "Reached when Core\\Serialize::decode refuses every class that encode refuses, a class is made by name only if the program declares it and new could make it, a class reference builds only a class new could make, decode checks every slot of a Core class it builds, every Core class says what its slots hold, every live crossing asks one receiving side, a published topic message reaches each subscriber with a class of the subscriber's own program, and topic names belong to one application."
context: [crates/nvs-runtime/src/graph.rs, docs/rules/classes/graph-copy.md, docs/rules/security/isolate-values-cross-by-copy.md]
stages:
  - id: 1
    title: decode refuses what encode refuses
    status: open
    context: [crates/nvs-runtime/src/object.rs, crates/nvs-stdlib/src/serialize.rs, crates/nvs-stdlib/src/instance.rs, docs/rules/classes/serialize-is-a-closed-format.md, tests/hostile/README.md]
  - id: 2
    title: "a class is made by name only if the program declares it and `new` could make it"
    status: open
    context: [crates/nvs-runtime/src/dispatch.rs, crates/nvs-runtime/src/object.rs, crates/nvs-stdlib/src/reflect.rs, crates/nvs-stdlib/src/instance.rs, crates/nvs-types/src/layout.rs, docs/rules/security/reflection-enforces-visibility.md, docs/rules/core-classes/reflect.md]
  - id: 3
    title: "a class reference builds only a class `new` could make"
    status: open
    context: [crates/nvs-codegen/src/lib.rs, crates/nvs-codegen/src/emit.rs, crates/nvs-ir/src/lower/expr.rs, crates/nvs-runtime/src/object.rs, docs/rules/types/class-reference.md, docs/rules/types/class-reference-sites.md]
  - id: 4
    title: "`decode` checks every slot of a `Core` class it builds"
    status: open
    context: [crates/nvs-runtime/src/object.rs, crates/nvs-stdlib/src/instance.rs, crates/nvs-stdlib/src/registry.rs, crates/nvs-stdlib/src/http.rs, docs/rules/classes/serialize-is-a-closed-format.md]
  - id: 5
    title: "every `Core` class says what its slots hold"
    status: open
    context: [crates/nvs-stdlib/src/instance.rs, crates/nvs-stdlib/src/registry.rs, crates/nvs-stdlib/src/serialize.rs, docs/rules/classes/serialize-is-a-closed-format.md, tests/hostile/README.md]
  - id: 6
    title: "every live crossing asks one receiving side, and a test lists the crossings"
    status: open
    context: [crates/nvs-host/src/isolate.rs, crates/nvs-host/src/placed.rs, crates/nvs-runtime/src/ctx/error.rs, crates/nvs-runtime/src/dispatch.rs]
  - id: 7
    title: "a published value reaches each subscriber as bytes it decodes into its own classes"
    status: open
    context: [crates/nvs-stdlib/src/topic.rs, crates/nvs-stdlib/src/bus.rs, crates/nvs-runtime/src/peer.rs, crates/nvs-stdlib/src/socket.rs, crates/nvs-stdlib/src/sse.rs, docs/rules/core-classes/topic.md]
  - id: 8
    title: a topic name belongs to one application
    status: open
    context: [crates/nvs-stdlib/src/topic.rs, crates/nvs-stdlib/src/bus.rs, crates/nvs-stdlib/src/cache.rs, crates/nvs-cli/src/events.rs, docs/rules/core-classes/topic.md]
  - id: 9
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
  - id: made-by-name-only-where-new-makes-it
    stage: 2
    name: "construct refuses a class new could not make and allocates nothing, forClass is null for a Core class and a class the compiler made, and one table holds every door's answer"
    run: [cargo, test, --lib, --, every_class_kind_gets_the_tables_answer_at_each_door, construct_refuses_a_class_new_could_not_make_and_allocates_nothing, for_class_is_null_for_a_core_class_and_a_class_the_compiler_made, every_core_descriptor_is_refused_by_construct, an_abstract_class_and_an_interface_are_laid_out_with_no_instances]
    timeout: 1800
    tests:
      - every_class_kind_gets_the_tables_answer_at_each_door
      - construct_refuses_a_class_new_could_not_make_and_allocates_nothing
      - for_class_is_null_for_a_core_class_and_a_class_the_compiler_made
      - every_core_descriptor_is_refused_by_construct
      - an_abstract_class_and_an_interface_are_laid_out_with_no_instances
  - id: a-compiled-unit-says-what-new-could-not-make
    stage: 2
    name: "a compiled unit marks an abstract class and an interface as classes with no instances"
    run: [cargo, test, --test, objects, --, a_compiled_unit_says_which_classes_new_could_not_make]
    timeout: 1800
    tests:
      - a_compiled_unit_says_which_classes_new_could_not_make
  - id: a-class-reference-builds-only-what-new-builds
    stage: 3
    name: "a run-time string converts to no class reference of a class the compiler made, and a new whose class is known only at run time refuses a class with no instances"
    run: [cargo, test, --test, objects, --, a_run_time_string_names_no_class_the_compiler_made, new_through_a_class_reference_refuses_a_class_with_no_instances, new_static_refuses_an_abstract_class]
    timeout: 1800
    tests:
      - a_run_time_string_names_no_class_the_compiler_made
      - new_through_a_class_reference_refuses_a_class_with_no_instances
      - new_static_refuses_an_abstract_class
  - id: decode-checks-core-slots
    stage: 4
    name: "decode refuses a Core class with a slot of the wrong type or with no stated slot types, and every Core descriptor is one or the other"
    run: [cargo, test, --lib, --, decode_checks_every_slot_of_a_core_class, a_core_class_that_states_no_slot_types_does_not_decode, every_core_descriptor_states_its_slots_or_is_refused]
    timeout: 1800
    tests:
      - decode_checks_every_slot_of_a_core_class
      - a_core_class_that_states_no_slot_types_does_not_decode
      - every_core_descriptor_states_its_slots_or_is_refused
  - id: every-core-class-says-what-it-holds
    stage: 5
    name: "every Core class with instances holds a host handle, states its slot types or says why it is not read back, and a value a member made decodes back"
    run: [cargo, test, --lib, --, every_core_class_with_instances_says_what_its_slots_hold, a_core_value_a_member_made_decodes_back]
    timeout: 1800
    tests:
      - every_core_class_with_instances_says_what_its_slots_hold
      - a_core_value_a_member_made_decodes_back
  - id: every-door-one-definition
    stage: 6
    name: "every public function of the graph module refuses the same classes, and a Core value crosses a live boundary"
    run: [cargo, test, --lib, --, every_door_of_the_graph_module_refuses_the_same_classes, a_core_value_crosses_a_live_boundary_into_any_program]
    timeout: 1800
    tests:
      - every_door_of_the_graph_module_refuses_the_same_classes
      - a_core_value_crosses_a_live_boundary_into_any_program
  - id: every-crossing-site
    stage: 7
    name: "every call of a graph door in the tree names its receiving side"
    run: [cargo, test, --test, crossings]
    timeout: 1800
    tests:
      - every_crossing_site_names_its_receiving_side
  - id: topic-delivers-into-the-subscribers-classes
    stage: 7
    name: "a published object arrives as the subscriber's own class, a subscriber that lacks the class gets an error, and a publish encodes once"
    run: [cargo, test, --lib, --, a_published_object_arrives_as_the_subscribers_own_class, a_subscriber_that_lacks_the_class_catches_an_error_and_reads_the_next_message, a_message_one_subscriber_cannot_read_still_reaches_the_others, a_publish_encodes_once_however_many_subscribed]
    timeout: 1800
    tests:
      - a_published_object_arrives_as_the_subscribers_own_class
      - a_subscriber_that_lacks_the_class_catches_an_error_and_reads_the_next_message
      - a_message_one_subscriber_cannot_read_still_reaches_the_others
      - a_publish_encodes_once_however_many_subscribed
  - id: topic-names-per-application
    stage: 8
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

**`Core\Reflect\ClassInfo::construct` builds any class its name resolves to.**
`construct_erased_from` (`crates/nvs-runtime/src/dispatch.rs:505`) allocates the object
(`crates/nvs-runtime/src/dispatch.rs:514`) and returns it as it is when the class has no
`constructor` row (`crates/nvs-runtime/src/dispatch.rs:515`). `construct` hands it whatever
`Ctx::class_desc` finds (`crates/nvs-stdlib/src/reflect.rs:3113`): every class of the unit's table
and every `Core` class with instances. `refusable` is not asked here either, and no descriptor says
that a class is `abstract`. `Reflect::forClass` resolves the same way
(`crates/nvs-stdlib/src/reflect.rs:2168`), though its doc comment says a `Core` name is refused
(`crates/nvs-stdlib/src/reflect.rs:2163`). `forObject(...)->name()` prints the label of a generator
or a function value for the program to use.

- `construct([])` on a generator's state class returns it with every slot never written. Iterating
  it ended the process with a segmentation fault.
- `construct([])` on `Core\Http\TlsInfo` returns an instance that passes `as Core\Http\TlsInfo`.
  `version()` returned the never-written slot as a `string`, and its first use was a segmentation
  fault. `Core\IO\File`, `Core\Uri`, `Core\ObjectSet` and `Core\Reflect\ClassInfo` are built the same
  way.
- A function value's class, the `$callable` marker, an anonymous object's class, an `abstract`
  class, an interface and the compiler's own `Stringable` and `Iterator` are built too, each seen
  in a run.

**A run-time string reaches a generator's state class with no reflection.** `$name as
class<Iterator<int>>` accepts a generator's label, because the set the conversion is compiled
against holds every class that conforms (`index_conforming`, `crates/nvs-codegen/src/lib.rs:1598`).
`new $c()` then built the state class, and a `foreach` over it ended the process with a segmentation
fault, run on the release binary.

**A `new` whose class is known only at run time builds a class that has no instances.** `new $c()`
and `new static()` are one instruction, and it allocates from whatever descriptor it is handed
(`crates/nvs-codegen/src/emit.rs:903`, `crates/nvs-codegen/src/emit.rs:3303`). A `class<T>` made
from the name of an `abstract` class or an interface built an instance of each. `new static()`
reached through a static call on an `abstract` class built one too. Both ran on the release binary,
and the instance passed its type and answered a method call.

Reflection needs no capability (`rule:security/reflection-needs-no-capability`), so a package with
an empty grant line can do all of this. It is the `decode` hole with no payload to forge.

**`decode` builds a `Core` class from whatever a payload puts in its slots.** `decode` checks each
object it built against its class's slot types (`crates/nvs-runtime/src/graph.rs:1154`,
`crates/nvs-runtime/src/object.rs:3815`). A `Core` class is defined with slot names alone
(`crates/nvs-stdlib/src/instance.rs:584`), so `ClassDesc::check_slot` has no type to compare
(`crates/nvs-runtime/src/object.rs:1683`). A `Core` member returns its slot as it is
(`read_slot`, `crates/nvs-stdlib/src/instance.rs:1216`). Bytes naming `Core\Http\TlsInfo` with an
`int` in the `version` slot decoded and passed `as Core\Http\TlsInfo`. `version()` then ended the
process with a segmentation fault, run on the release binary. Refusing the classes `encode` refuses
does not close it, because `Core\Http\TlsInfo` holds no host handle.

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

**One question is asked by every door that makes an object from a name: may a program make an
instance of this class by name?** It is one function on `ClassDesc`. It is the function stage 1
gives the decoder, moved onto the descriptor so `nvs-stdlib` and `nvs-codegen` can ask it too. It
takes the door that asks, a copy boundary or `construct`, and returns the reason for a refusal.
`refusable` stays as the copy doors' caller of it and keeps its sentences. `construct_erased_from`
asks before it allocates, so its three callers all ask: `ClassInfo::construct`, the router's
handler (`crates/nvs-stdlib/src/router.rs:3259`) and the test fixtures
(`crates/nvs-runtime/src/dispatch.rs:1014`). **What it spends:** one call that reads a few flags,
and one branch, per `construct`. A refused `construct` allocates nothing, where today it allocates
first. Declined: a second predicate beside `refusable`, which is two lists to keep in step.

**The doors answer differently for three kinds, and the table is the function.**

| Class | `decode` | Live crossing | `construct` | `forClass` |
|---|---|---|---|---|
| A class the program declares, the error tree included | made | crosses | made | described |
| The same with a `secret` property | refused | refused | made | described |
| An `abstract` class or an interface | refused | has no instance | refused | described |
| An anonymous object's class, `$shape{…}` | made | crosses | refused | `null` |
| A function value's class, a generator's state class | refused | refused | refused | `null` |
| Every other class the compiler made | refused | refused | refused | `null` |
| A `Core` class that holds a host handle | refused | refused | refused | `null` |
| A `Core` class that states its slot types | made, each slot checked | crosses | refused | `null` |
| Every other `Core` class | refused | crosses | refused | `null` |

`encode` refuses every class `decode` refuses, with the same sentence. Stage 1 gives the function
its first rows, stage 2 the `construct` and `forClass` columns, and stage 4 the two `Core` rows that
differ at `decode`.

`construct` asks what `new` asks of a class the program declares. A class with a `secret` property
is made, because its constructor runs and nothing leaves the program. A `Core` value such as
`Core\Uri` crosses a boundary and is not made by name: a `Core` member makes it, and no constructor
of it reaches the runtime. `new Core\ObjectSet<T>()` is the one `new` this leaves out, because that
constructor is a native symbol the lowering calls (`crates/nvs-types/src/expr/calls.rs:1209`). An
anonymous object crosses as data and has no name a program writes. One test drives a class of every
row through every door and compares each answer with its row. Safe default: `construct` refuses a
kind the function has no row for.

**A class with no instances is one bit, carried from the class graph.** `new` refuses an `abstract`
class and an interface with one question, `ClassLinks::concrete`
(`crates/nvs-hir/src/hierarchy.rs:218`, `crates/nvs-types/src/expr/calls.rs:1276`). The layout
carries that answer down beside `is_callable`, and the compiler's own interfaces
(`crates/nvs-hir/src/interfaces.rs:45`) carry it too. `decode` asks the bit as well: no `abstract`
class has an instance to encode, so bytes that name one are forged. **What it spends:** one `bool`
per class per compiled unit, and one more flag read per decoded object. Safe default: a class a
native test defines by hand has instances. Declined: telling an `abstract` class from an interface
at run time, which is a second bit that changes one word of a message.

**A `Core` class is marked where `nvs_stdlib::instance` defines it.** One loop defines every `Core`
descriptor (`crates/nvs-stdlib/src/instance.rs:584`), so one line there marks them all, as
`set_host_handle` marks some. Declined: the `Core\` name prefix the erased call tests
(`crates/nvs-runtime/src/dispatch.rs:408`). Six classes of the error tree have such a name
(`crates/nvs-hir/src/errors.rs:115`). They are rows of the program's own table and `new` makes each
of them, so they stay made and described.

**`Reflect::forClass` returns `null` for every class `construct` refuses because of its kind.** That
is a `Core` class and every class the compiler made, an anonymous object's included. Its doc comment
and its `about.md` already say so for `Core`. It still describes an `abstract` class and an
interface, because their members are the program's own declarations. `Reflect::forObject` still
describes any object, and `construct` on that description throws. **What it spends:** the same call
once per `forClass`. Declined: `forObject` throwing for a generator or a `Core` object, which takes
`name()` away from a log line.

**A refused `construct` throws a `LogicError` the program can catch, never a `FATAL`.** The sentence
names the class and the reason. A kind `forClass` answers `null` for gets the reason a missing class
gets today, "is not a class this program declares" (`crates/nvs-stdlib/src/reflect.rs:3117`). A
class with no instances gets "is `abstract` or an interface, so it has no instances", the checker's
words for `new` (`crates/nvs-types/src/expr/calls.rs:1292`).

**A run-time string converts to no class reference of a class the compiler made.**
`index_conforming` leaves out every class the question refuses because of its kind, so the
conversion throws as it does for an unknown name. **What it spends:** nothing at run time, because
the set is built while compiling. A written-out name is not affected: the checker resolves declared
names only. The set still holds an `abstract` class and an interface, because a static call through
such a reference is legal.

**A `new` whose class is known only at run time refuses a class that can have no instances.** That
is `new $c()` over a `class<T>` and `new static()`, which are one instruction. The refusal is at the
allocation, and it asks the no-instances bit stage 2 adds. It throws a `LogicError` the program can
catch, with `construct`'s sentence for such a class. The allocation goes through a checked entry
point beside `nvs_object_new` (`crates/nvs-runtime/src/object.rs:4342`). **What it spends:** one
byte load and one branch per such `new`. A `new` of a class written by name pays nothing, because
the checker already refused it there. Declined: leaving it to the contained `FATAL` at the first
bodiless call. A value of a type that has no instances is a broken promise of the type system, and
a program cannot catch a `FATAL`. Safe default: where the allocation cannot take an error edge, the
lowering emits the test as its own fallible instruction in front of it.

**`decode` checks every slot of every class it builds, a `Core` class included.** Each `Core`
descriptor states the type of each slot in the three per-slot tables `ClassDesc::check_slot` reads
(`crates/nvs-runtime/src/object.rs:1683`). `decode` already runs that check on every object it
built, so one check serves a program's class and a `Core` class. **What it spends:** one tag test
per slot per decoded `Core` object, and one small table per `Core` class, built once per process. A
program that decodes nothing pays nothing per request.

**The slot types are one roster where `nvs_stdlib::instance` defines the descriptors.** It sits
beside `HOST_HANDLE_CLASSES` (`crates/nvs-stdlib/src/instance.rs:143`) and reaches every descriptor
of a name, as that roster does (`crates/nvs-stdlib/src/instance.rs:636`). A row is the class and
one `CoreTy` per slot, the type language a member's signature uses
(`crates/nvs-stdlib/src/registry.rs:242`). One function turns a `CoreTy` into a tag, the admitted
classes and a `SlotCheck` (`crates/nvs-runtime/src/object.rs:5932`). A row whose length is not the
slot count stops the table's build. Declined: a field on every `CoreClass` literal. Most of them
have no slots, and one loop can say it. Declined: a hand-written list of `Core` classes that are
"data". It states nothing `decode` can check, and it is the member-by-member shape that leaves a
sibling. Safe default: a `CoreTy` the function has no run-time form for fails the stage's test. It
never passes as "any value".

**A `Core` class with no row of slot types does not decode, and `encode` does not write it.** That
is the safe default: a class somebody adds later is refused until it says what it holds. A class
with no slots needs a row too, an empty one. The refusal is one arm of the one question, asked by
the bytes carrier in both directions. So `Core\Cache\Store::put` throws where a later `get` would.
A descriptor with no registry row, such as the cursor of a `foreach`, has no row and is refused.

**A row states types only when the types are everything a reader relies on.** A reader that uses a
slot as an index, a length or a key into another slot relies on more than a type. Such a class gets
a row that gives the reason, and it is not read back. Stage 5 decides each class by reading its
readers. Safe default: a class whose readers were not read gets a reason. Usability cost: such a
class no longer goes through the cache or `Core\Serialize`. The Backlog names what brings it back.

**A live crossing needs no slot check.** The live carrier copies an object that exists, and only a
`Core` member or `decode` made it. With `decode` checked and `construct` closed, each slot of it has
its stated type. The live doors are `spawn` in and out on one core, both `upgrade` members and
`Fixtures::copy`. The doors that read bytes are these.

| Door | Who wrote the bytes |
|---|---|
| `Core\Serialize::decode` | The program, which hands them over |
| The shared cache tier | Whoever can write the store, which is outside the process |
| The session record | Whoever can write the session store: the data folder's file or the shared store |
| The memory cache tier | This process's `encode` |
| `spawn` placed on another core | This process's `encode` |
| A topic message | This process's `encode` |

The first three read bytes a program or an outsider can write (`crates/nvs-stdlib/src/serialize.rs:181`,
`crates/nvs-stdlib/src/cache.rs:2760`, `crates/nvs-stdlib/src/session.rs:1140`). The last three
read bytes nothing forged (`crates/nvs-stdlib/src/cache.rs:2759`, `crates/nvs-host/src/placed.rs:264`,
`crates/nvs-host/src/placed.rs:347`, `crates/nvs-stdlib/src/topic.rs:712`). From stage 7 on, each
`receive()` is the topic's door. All six go through `Reader::object`, so all six are checked, and
the last three pay the tag tests too. Declined: skipping the check for bytes the process wrote,
which is a second decoder to keep in step.

**Declined: making every `Core` reader test its slot before it returns it.** `decode` checks each
slot at the one place the bytes come in, and `construct` makes no `Core` instance. So no `Core`
instance has a slot of a type its class does not state. A test in each reader is one check per
member, and a new member would have to remember it.

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

**Lanes:** one lane. Stages 1, 2, 4 and 6 all change `graph.rs`. Stage 2 also changes the function
stage 1 makes and shares `object.rs` and `instance.rs` with it, so it runs after stage 1, and it
shares `dispatch.rs` with stage 6. Stage 3 asks the bit stage 2 adds and shares `object.rs` and
`crates/nvs-codegen/src/lib.rs` with it. Stage 4 adds rows to the function stage 2 moves and shares
`instance.rs` with it. Stage 5 fills the roster stage 4 makes. Stage 7 removes the call stage 6's
site test would fail on, and stages 7 and 8 both change `topic.rs` and `bus.rs`. They run in order.

## Stages

### 1. decode refuses what encode refuses

**Does:** make one function answer whether a class may cross, and call it from the decoder before
any object is made.

- [x] `ClassDesc` gains one flag for a compiler-made class: a `$` in the name and not a shape. It is
  set when the class is defined (`rule:classes/graph-copy`, `crates/nvs-runtime/src/object.rs:1452`,
  `crates/nvs-runtime/src/object.rs:2108`).
- [x] `refusable` becomes the one function. It also refuses a compiler-made class and a class with a
  `secret` property, naming the property. The walk's own `secret` loop goes
  (`rule:classes/graph-copy`, `crates/nvs-runtime/src/graph.rs:444`,
  `crates/nvs-runtime/src/graph.rs:373`).
- [x] `Reader::object` calls it after the resolve and before the property comparison
  (`rule:classes/serialize-is-a-closed-format`, `crates/nvs-runtime/src/graph.rs:1067`,
  `crates/nvs-runtime/src/graph.rs:1079`).
- [x] Test `decode_refuses_every_class_encode_refuses` in `graph.rs` tests. One table holds a class of
  each refused kind and one plain class. For each, `encode` of an instance and `decode` of a payload
  naming it give the same sentence, and `decode` builds no object
  (`crates/nvs-runtime/src/graph.rs:1247`).
- [x] Test `a_refused_class_is_refused_before_its_properties_are_compared`: a payload naming a
  host-handle class with a wrong property count gets the class refusal
  (`crates/nvs-runtime/src/graph.rs:1247`).
- [x] Test `a_class_the_compiler_made_does_not_cross_and_a_shape_does`: a `$gen`, a `$fn0` and a `$fcc0`
  name are refused by both carriers, and a `$shape{n}` instance round-trips
  (`crates/nvs-runtime/src/graph.rs:1247`).
- [x] Test `every_core_class_that_holds_a_host_handle_is_refused_by_decode` in `instance.rs` tests. It
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
- [x] The fuzz check's class table gains a class of each refused kind, and it fails when `decode`
  returns an object of one (`crates/nvs-fuzz-checks/src/serialize.rs:1`).
- [x] `rule:classes/serialize-is-a-closed-format` says `decode` refuses every class `encode` refuses,
  before any object is made, with the cost line. `rule:classes/graph-copy` lists the five refusals
  and says one function is called by the walk and by the decoder
  (`docs/rules/classes/serialize-is-a-closed-format.md:1`, `docs/rules/classes/graph-copy.md:7`).
- [x] The `decode` card's `ParseError` row and the `decode` `about.md` say, in one plain sentence,
  that bytes naming a generator, a function value or an open resource are not read
  (`crates/nvs-stdlib/src/serialize.rs:123`, `docs/examples/core/Serialize/decode/about.md:1`,
  `docs/reference/core/Serialize.md:1`).

### 2. a class is made by name only if the program declares it and `new` could make it

**Does:** move the one question onto `ClassDesc` with the door that asks, and ask it before
`construct` allocates and where `forClass` resolves a name.

**Split:** once the Rust change builds, one writing subagent writes the attack, the `.nvst` case
and the `about.md` sentence. Its files are disjoint from the session's, and it does not build.

- [ ] `ClassDesc` gains two flags beside `is_callable`: a `Core` class, and a class with no
  instances. Each has a setter on `set_host_handle`'s terms (`rule:classes/graph-copy`,
  `crates/nvs-runtime/src/object.rs:629`, `crates/nvs-runtime/src/object.rs:2226`,
  `crates/nvs-runtime/src/object.rs:2282`).
- [ ] The question moves onto `ClassDesc` as one public function. It takes the door, a copy boundary
  or `construct`, and returns the reason for a refusal. Its body is the table in *Standing
  decisions*, one arm per row. `refusable` calls it for the copy doors, keeps its sentences and gains
  the refusal of a class with no instances (`rule:classes/graph-copy`,
  `crates/nvs-runtime/src/object.rs:1495`, `crates/nvs-runtime/src/graph.rs:444`).
- [ ] `nvs_stdlib::instance` marks every descriptor it defines as a `Core` class
  (`rule:core-classes/reflect`, `crates/nvs-stdlib/src/instance.rs:584`,
  `crates/nvs-stdlib/src/instance.rs:639`).
- [ ] `ClassLayout` gains the no-instances bit, read from `ClassLinks::concrete`. The error tree's
  classes have instances, and the compiler's own interfaces have none
  (`rule:security/reflection-enforces-visibility`, `crates/nvs-types/src/layout.rs:72`,
  `crates/nvs-types/src/layout.rs:441`, `crates/nvs-types/src/layout.rs:492`,
  `crates/nvs-types/src/layout.rs:555`, `crates/nvs-hir/src/hierarchy.rs:218`).
- [ ] `ir::Class` carries the bit, and each of its eight literals states it. The layout copy copies
  it, and the seven compiler-made classes say they have instances (`crates/nvs-ir/src/ir.rs:534`,
  `crates/nvs-ir/src/lower/mod.rs:1091`, `crates/nvs-ir/src/lower/mod.rs:1180`,
  `crates/nvs-ir/src/lower/mod.rs:1213`, `crates/nvs-ir/src/lower/mod.rs:2736`,
  `crates/nvs-ir/src/lower/mod.rs:4836`, `crates/nvs-ir/src/lower/anon_fn.rs:302`,
  `crates/nvs-ir/src/lower/anon_fn.rs:1127`, `crates/nvs-ir/src/lower/generator.rs:524`).
- [ ] `nvs-codegen` hands the bit to its setter where it hands `is_callable` to `set_callable`
  (`crates/nvs-codegen/src/lib.rs:1777`).
- [ ] `construct_erased_from` asks the question before `NvsObj::new` and returns a thrown
  `LogicError` with the sentences of *Standing decisions*. Its doc comment is rewritten, because the
  allocation now follows the check (`rule:security/reflection-enforces-visibility`,
  `crates/nvs-runtime/src/dispatch.rs:505`, `crates/nvs-runtime/src/dispatch.rs:514`,
  `crates/nvs-runtime/src/dispatch.rs:484`).
- [ ] `Reflect::forClass` returns `null` when the question refuses the class because of its kind,
  and still describes a class with no instances. Its doc comment and `construct`'s are rewritten to
  say what is now true (`rule:core-classes/reflect`, `crates/nvs-stdlib/src/reflect.rs:2168`,
  `crates/nvs-stdlib/src/reflect.rs:2159`, `crates/nvs-stdlib/src/reflect.rs:3092`).
- [ ] Test `every_class_kind_gets_the_tables_answer_at_each_door` in `object.rs` tests. One class
  table holds a class of every row. The test asks each door and compares the answer with the row
  (`crates/nvs-runtime/src/object.rs:6266`).
- [ ] Test `construct_refuses_a_class_new_could_not_make_and_allocates_nothing` in `reflect.rs`
  tests. For a class of each refused row, `construct` throws a `LogicError` that names the class,
  and the context's memory balance is what it was before the call. A plain class and `LogicError`
  are still built. `forObject` of an instance of a refused class describes it, and `construct` on
  that description throws (`crates/nvs-stdlib/src/reflect.rs:3380`).
- [ ] Test `for_class_is_null_for_a_core_class_and_a_class_the_compiler_made`:
  `Core\Http\TlsInfo`, a `$gen`, a `$fn0`, `$callable` and a `$shape{n}` name give `null`. An
  `abstract` class, an interface and a `Core\Db\DbError` of the program's own table are described
  (`crates/nvs-stdlib/src/reflect.rs:3914`).
- [ ] Test `every_core_descriptor_is_refused_by_construct` in `instance.rs` tests. It walks the
  process's `Core` descriptors, not a list in the test, and `construct_erased_from` refuses each
  (`crates/nvs-stdlib/src/instance.rs:1237`, `crates/nvs-stdlib/src/instance.rs:544`).
- [ ] Test `an_abstract_class_and_an_interface_are_laid_out_with_no_instances` in `layout.rs`
  tests. An `abstract` class, an interface and `Stringable` have none. A plain class, a subclass of
  the `abstract` one and `LogicError` have instances (`crates/nvs-types/src/layout.rs:1194`).
- [ ] Test `a_compiled_unit_says_which_classes_new_could_not_make` in the `objects` integration
  test. It compiles a unit with an `abstract` class, an interface, a generator and a function value,
  and reads each descriptor's answer for `construct` (`crates/nvs-codegen/tests/objects.rs:344`).
- [ ] `.nvst` case `reflect-construct-refuses-a-class-new-could-not-make`. The program asks
  `forClass` and `construct` for its own generator's class, its own function value's class, an
  anonymous object's class, `Core\Http\TlsInfo`, an `abstract` class and an interface. It prints
  `null` or the caught `LogicError` for each, and it builds a plain class and a `LogicError`
  (`tests/conformance/core/reflect-construct-refuses-a-class-new-could-not-make.nvst:1`).
- [ ] Attack `03-classes-a-program-cannot-make.nvs`. It builds its own generator's class by name
  and iterates the result, builds its own function value's class and calls it, builds
  `Core\Http\TlsInfo` and the class of a `Core\Uri` it holds and calls a method of each, and builds
  an `abstract` class and an interface. Every step should give `null` or throw. The goal
  `calls-by-name-are-checked` owns `02` in this folder (`rule:testing/feature-proofs`,
  `tests/hostile/core/Reflect-ClassInfo/construct/03-classes-a-program-cannot-make.nvs:1`).
- [ ] `rule:classes/graph-copy` holds the table with its four columns and says the function takes
  the door. `rule:security/reflection-enforces-visibility` says `construct` makes only a class the
  program declares and `new` could make, with the cost line and the declined choices.
  `rule:core-classes/reflect` says which names `forClass` returns `null` for and that `forObject`
  describes any object (`docs/rules/classes/graph-copy.md:7`,
  `docs/rules/security/reflection-enforces-visibility.md:8`, `docs/rules/core-classes/reflect.md:9`).
- [ ] The `construct` card's `LogicError` row and the `construct` `about.md` say, in one plain
  sentence each, that an `abstract` class, an interface and a class of `Core` cannot be created
  this way (`crates/nvs-stdlib/src/reflect.rs:1057`,
  `docs/examples/core/Reflect-ClassInfo/construct/about.md:7`).

No existing test is expected to go red. No case in the tree writes `forClass('Core…')`. Cases that
build a class name at run time were not searched.

### 3. a class reference builds only a class `new` could make

**Does:** leave every class the question refuses because of its kind out of the set a run-time
string converts against, and make a `new` whose class is known only at run time refuse a class with
no instances.

**Split:** an `Explore` subagent first lists every place that names `nvs_object_new`, as
`file.rs:NN` and one line each, so the new entry point gets the same rows. Once the Rust change
builds, one writing subagent writes the attack, the two `.nvst` cases and the `about.md` sentence.
Its files are disjoint from the session's, and it does not build.

- [ ] `index_conforming` leaves out every class the question refuses because of its kind. It keeps
  an `abstract` class and an interface (`rule:types/class-reference`,
  `crates/nvs-codegen/src/lib.rs:1598`).
- [ ] A checked entry point beside `nvs_object_new` reads the no-instances bit before it allocates.
  For such a class it allocates nothing and throws a `LogicError` with `construct`'s sentence
  (`rule:types/class-reference-sites`, `crates/nvs-runtime/src/object.rs:4342`,
  `crates/nvs-runtime/src/object.rs:1495`).
- [ ] `emit_new` calls that entry point when `dynamic` is set and takes the instruction's error
  edge. A `new` of a class written by name keeps `nvs_object_new`. The doc comment is rewritten
  (`rule:types/class-reference-sites`, `crates/nvs-codegen/src/emit.rs:3294`,
  `crates/nvs-codegen/src/emit.rs:3303`, `crates/nvs-codegen/src/emit.rs:903`).
- [ ] Both lowerings of the dynamic `new` are fallible, with or without a constructor: `new static()`
  and `new $cls()` (`rule:types/class-reference-sites`, `crates/nvs-ir/src/lower/expr.rs:3716`,
  `crates/nvs-ir/src/lower/expr.rs:3810`).
- [ ] Test `a_run_time_string_names_no_class_the_compiler_made` in the `objects` integration test.
  It runs `$name as ?class<Iterator<int>>` over a generator's label and gets `null`, and the same
  conversion over a declared class's name gives the reference (`crates/nvs-codegen/tests/objects.rs:344`).
- [ ] Test `new_through_a_class_reference_refuses_a_class_with_no_instances`: `new $c()` over the
  name of an `abstract` class and of an interface throws a `LogicError` and leaves the memory
  balance as it was. Over the name of a subclass it builds the subclass
  (`crates/nvs-codegen/tests/objects.rs:344`).
- [ ] Test `new_static_refuses_an_abstract_class`: a static method that returns `new static()`
  throws when it is called on its `abstract` class, and builds the subclass when called on one
  (`crates/nvs-codegen/tests/objects.rs:344`).
- [ ] `.nvst` case `a-class-reference-from-a-string-never-names-a-class-the-compiler-made`:
  `as class<Iterator<int>>` over a generator's label throws, and `as ?class<Iterator<int>>` gives
  `null`
  (`tests/conformance/class/a-class-reference-from-a-string-never-names-a-class-the-compiler-made.nvst:1`).
- [ ] `.nvst` case `a-new-of-a-class-known-at-run-time-refuses-a-class-with-no-instances`. It runs
  `new $c()` over the name of an `abstract` class and of an interface, and `new static()` through a
  static call on an `abstract` class. It prints the caught `LogicError` of each, and then builds a
  subclass through each of the three
  (`tests/conformance/class/a-new-of-a-class-known-at-run-time-refuses-a-class-with-no-instances.nvst:1`).
- [ ] Attack `03-names-of-classes-new-cannot-make.nvs`. It converts its own generator's label with
  `as ?class<Iterator<int>>`, builds an `abstract` class and an interface through a `class<T>`, and
  calls a method on each result. It also reaches `new static()` on an `abstract` class. Every step
  should give `null` or throw. The goal `calls-by-name-are-checked` owns `02` in this folder
  (`rule:testing/feature-proofs`,
  `tests/hostile/lang/types/callable-classes-object-shapes/03-names-of-classes-new-cannot-make.nvs:1`).
- [ ] `rule:types/class-reference` says a run-time string converts to no class the compiler made.
  `rule:types/class-reference-sites` says the dynamic `new` refuses a class with no instances, with
  the error, the cost line and the declined choice (`docs/rules/types/class-reference.md:8`,
  `docs/rules/types/class-reference-sites.md:5`, `docs/rules/types/class-reference-sites.md:14`).
- [ ] The `about.md` of the feature says, in one plain sentence, that `new $c()` throws a
  `LogicError` when the name is an `abstract` class or an interface
  (`docs/examples/lang/types/callable-classes-object-shapes/about.md:9`).

`crates/nvs-ir/tests/class_reference.rs:136` reads the dynamic `new` instruction. It goes red only
if the instruction's printed form changes, and then it follows in the same slice.

### 4. `decode` checks every slot of a `Core` class it builds

**Does:** let a `Core` descriptor state its slot types in the tables `check_slot` reads, refuse a
`Core` class that states none at the bytes carrier, and give the first two classes their rows.

- [ ] `ClassTable` gains one setter for a `Core` class's slot types. It fills the three per-slot
  tables and sets one flag, and `ClassDesc` gains the flag's reader beside `holds_host_handle`
  (`rule:classes/serialize-is-a-closed-format`, `crates/nvs-runtime/src/object.rs:2317`,
  `crates/nvs-runtime/src/object.rs:2513`, `crates/nvs-runtime/src/object.rs:2535`,
  `crates/nvs-runtime/src/object.rs:1495`).
- [ ] The question's door tells the bytes carrier from the live carrier. At the bytes carrier it
  refuses a `Core` class whose flag is not set, and the live carrier admits it. `Reader::object`
  and the encoder's `admit` both ask (`rule:classes/graph-copy`,
  `crates/nvs-runtime/src/graph.rs:444`, `crates/nvs-runtime/src/graph.rs:1067`,
  `crates/nvs-runtime/src/graph.rs:213`, `crates/nvs-runtime/src/graph.rs:741`).
- [ ] `nvs_stdlib::instance` gains the roster of slot types beside `HOST_HANDLE_CLASSES`. A row is a
  class and either one `CoreTy` per slot or the reason the class is not read back. The table's build
  hands each typed row to the setter, for every descriptor of the name
  (`rule:classes/serialize-is-a-closed-format`, `crates/nvs-stdlib/src/instance.rs:143`,
  `crates/nvs-stdlib/src/instance.rs:636`, `crates/nvs-stdlib/src/instance.rs:513`).
- [ ] One function in `instance.rs` turns a `CoreTy` into a tag, the admitted classes and a
  `SlotCheck`. It has an arm for each `CoreTy` a row uses and no arm that means "any value" by
  default (`crates/nvs-stdlib/src/registry.rs:242`, `crates/nvs-runtime/src/object.rs:5932`,
  `crates/nvs-runtime/src/object.rs:1683`).
- [ ] `Core\Http\TlsInfo` and `Core\Uri` get the first two rows, read from the code that fills
  their slots (`crates/nvs-stdlib/src/http.rs:2066`, `crates/nvs-stdlib/src/http.rs:3739`,
  `crates/nvs-stdlib/src/uri.rs:677`).
- [ ] In a debug build, `build` asserts that each slot value passes its class's stated type. So
  every test that makes a `Core` instance proves the row, and a release build pays nothing
  (`crates/nvs-stdlib/src/instance.rs:834`).
- [ ] Test `decode_checks_every_slot_of_a_core_class` in `instance.rs` tests. For each slot of
  `Core\Http\TlsInfo` in turn, a payload with a value of another kind in that slot is refused, and
  the sentence names the class and the slot. A payload with the stated kinds decodes, and
  `version()` returns its string (`crates/nvs-stdlib/src/instance.rs:1237`).
- [ ] Test `a_core_class_that_states_no_slot_types_does_not_decode`: a payload naming such a class
  is refused before an object is made, `encode` of an instance gives the same sentence, and a live
  copy of the instance still crosses (`crates/nvs-stdlib/src/instance.rs:1237`).
- [ ] Test `every_core_descriptor_states_its_slots_or_is_refused`. It walks every descriptor of the
  process's table, the ones with no registry row included, and holds no list of its own. A
  descriptor with stated types refuses a wrong-kind value in each slot. Every other descriptor is
  refused by `decode` whatever its payload holds. The test fails naming each descriptor that does
  neither (`crates/nvs-stdlib/src/instance.rs:1237`, `crates/nvs-stdlib/src/instance.rs:544`).
- [ ] Stage 2's table test gains the two `Core` rows that differ at `decode`
  (`crates/nvs-runtime/src/object.rs:6266`).
- [ ] `rule:classes/serialize-is-a-closed-format` says `decode` checks a `Core` class's slots as it
  checks a program's properties, and that a `Core` class with no stated slot types is not written
  or read, with the cost line and the declined choices. `rule:classes/graph-copy` holds the two new
  rows (`docs/rules/classes/serialize-is-a-closed-format.md:10`, `docs/rules/classes/graph-copy.md:7`).
- [ ] The module doc of `instance.rs` states what the slot tables spend, beside what the
  descriptors spend (`crates/nvs-stdlib/src/instance.rs:102`).

Until stage 5 lands, every `Core` class but the two is refused by the bytes carrier. Tests that put
another `Core` value through the cache, a session or `Core\Serialize` are expected to go red at the
gate if stage 5 misses a row. They are fixed by the row, never by changing the test.

### 5. every `Core` class says what its slots hold

**Does:** give every `Core` class with instances a row in the roster, its slot types or the reason
it is not read back, and prove the roster is complete from the descriptors themselves.

**Split:** an `Explore` subagent first lists every `CoreClass` with instances and no host handle,
as `file.rs:NN` and the slot names. `reader` subagents then take those files in groups and go out
together. Each reports, per class, what fills each slot and how each reader uses it. The session
writes the rows, because they share one file. Once the rows build, one writing subagent writes the
attack, the `.nvst` case and the reference sentences, and it does not build.

- [ ] The list of classes, from the `Explore` subagent, checked against the registry's own count of
  classes with instances (`crates/nvs-stdlib/src/instance.rs:661`,
  `crates/nvs-stdlib/src/registry.rs:1801`).
- [ ] Every class on the list gets its row. The row states types when the types are everything its
  readers rely on, and a reason otherwise, as *Standing decisions* says. A class with no slots gets
  an empty row (`rule:classes/serialize-is-a-closed-format`, `crates/nvs-stdlib/src/instance.rs:143`,
  `crates/nvs-stdlib/src/instance.rs:1216`).
- [ ] Test `every_core_class_with_instances_says_what_its_slots_hold` in `instance.rs` tests. It
  walks the registry's classes with instances and fails naming each one that holds no host handle
  and has no row. It also fails on a row whose class holds a host handle
  (`crates/nvs-stdlib/src/instance.rs:1237`, `crates/nvs-stdlib/src/instance.rs:768`).
- [ ] Test `a_core_value_a_member_made_decodes_back`: a `Core\Uri` a member made is encoded and
  decoded, and its members return what the first one's did
  (`crates/nvs-stdlib/src/instance.rs:1237`).
- [ ] `.nvst` case `serialize-decode-checks-a-core-class`. The program encodes a `Core\Uri` and
  reads it back. It then builds `bytes` that name a `Core` class with a value of the wrong type in
  one slot, and `decode` throws a `ParseError`
  (`tests/conformance/core/serialize-decode-checks-a-core-class.nvst:1`).
- [ ] Attack `03-bytes-that-name-a-core-class.nvs`. Its payloads name a `Core` class with a value
  of the wrong type in each slot in turn, and a `Core` class that is not read back. Every step
  should throw, and the program should still be running after the last one
  (`rule:testing/feature-proofs`,
  `tests/hostile/core/Serialize/decode/03-bytes-that-name-a-core-class.nvs:1`).
- [ ] The fuzz check's class table gains a `Core` class with stated slot types. It fails when
  `decode` returns an object of it whose slot does not pass the stated type
  (`crates/nvs-fuzz-checks/src/serialize.rs:1`).
- [ ] The `decode` card and its `about.md` say, in one plain sentence, that bytes with a value of
  the wrong type in an object are not read. The `encode` card and its `about.md` say that some
  `Core` objects cannot be written (`crates/nvs-stdlib/src/serialize.rs:123`,
  `docs/examples/core/Serialize/decode/about.md:1`, `crates/nvs-stdlib/src/serialize.rs:90`,
  `docs/examples/core/Serialize/encode/about.md:1`).
- [ ] `rule:classes/serialize-is-a-closed-format` says where the roster is, what a row holds and
  when a row is a reason (`docs/rules/classes/serialize-is-a-closed-format.md:10`).

### 6. every live crossing asks one receiving side, and a test lists the crossings

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
  `crates/nvs-stdlib/src/topic.rs:646` stays until stage 7 and is the one site the new test names.
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
  It is red until stage 7 removes `topic.rs:646`, so its check `every-crossing-site` belongs to
  stage 7, and the handoff says so.
- [ ] `.nvst` case: a `Core\Uri` in `args:` reaches a `spawn script` method entry and comes back in
  its answer (`tests/conformance/isolate/a-core-value-crosses-into-a-spawned-script-and-back.nvst:1`).
- [ ] `rule:security/isolate-values-cross-by-copy` says the receiving side resolves its own classes
  and every `Core` class. `rule:classes/graph-copy` lists the crossings and says `Core\Queue::push`
  carries JSON (`docs/rules/security/isolate-values-cross-by-copy.md:9`,
  `docs/rules/classes/graph-copy.md:13`).

### 7. a published value reaches each subscriber as bytes it decodes into its own classes

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

### 8. a topic name belongs to one application

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

### 9. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stage 1 is built except its `.nvst` case and its attack. `ClassDesc::is_compiler_made` is a `$` in the
name and not `$shape{`, set in `ClassTable::push_class`. `refusable` in `graph.rs` refuses a
callable's class, a generator's state class, a host-handle class, a compiler-made class and a class
with a `secret` property (naming the first one). The walk's own `secret` loop is gone, and
`Reader::object` asks `refusable` right after the class resolves, before the property comparison. The
four acceptance tests of `decode-refuses-what-encode-refuses` pass when run by name. The fuzz table,
both rule fragments, the `decode` card, its `about.md` and the reference page say so.

The sweep's `env-secret-proofs` red is stale: the check was retired with `derive-wider-types` and no
longer exists. Nightly run 43001191130 ran at f4c1d5c7e2. Its one red on all four runners,
`live_config`'s `a_changed_server_timeout_applies_to_the_next_connection`, was fixed by 305c04f660, and
it passes on this tree.

Still true from the goal's start: `Core\Topic::publish` copies with no receiving table, and the topic
table is keyed by the bare name. `spawn script` refuses a `Core` value in `args:`.
`ClassInfo::construct` builds any class its name resolves to. `new $c()` and `new static()` build an
`abstract` class. `decode` builds a `Core` class with a value of any type in its slots.

No test is expected to go red. One to watch at the gate: a test that sends a `$`-named class other
than a shape across `spawn`, the cache or `Core\Serialize` is now refused.

### Next

Finish stage 1, then set its status to done and take stage 2.

- [ ] `tests/conformance/core/serialize-decode-refuses-a-class-the-compiler-made.nvst:1`: the program
  builds `bytes` that name its own generator's state class (`{fn}$gen`), its own closure's class and
  `Core\IO\File`, and each `decode` throws a `ParseError`. The format is `NVS\x1b`, version byte `1`,
  node byte `9`, the class name as a LEB128 length and the bytes, the property count, each property
  name the same way, then one node per property (`0` is `null`). `payload_naming` in the graph tests
  writes exactly this (`crates/nvs-runtime/src/graph.rs:1293`). The plain way to get a closure's or a
  generator's label is to read it from the `ParseError` of `encode` on one, or from
  `nvs_ir::lower::generator` (`{name}$gen`). Write the bytes as `Core\Taint::assertTrustedBytes` over
  a literal the case builds, as `decode`'s existing cases do.
- [ ] `tests/hostile/core/Serialize/decode/02-bytes-that-name-a-class-encode-refuses.nvs:1`: payloads
  naming a generator's state class with a wrong-kind slot, a host-handle class (`Core\IO\File`) with a
  chosen key, and a callable's class. Every step should throw. Its comments follow AGENTS.md § *Text an
  end user reads* (`// Attack:` top line), and `bun nv proofs --comments <path>` checks the bounds.
- [ ] Then stage 2 from its first item (`crates/nvs-runtime/src/object.rs:1452` for the flags,
  `crates/nvs-runtime/src/graph.rs:444` for the function that moves onto `ClassDesc`).

### Backlog

- `ClassTable::id_of` is a linear scan, and `decode` calls it once per object
  (`crates/nvs-runtime/src/object.rs:2878`).
- A `Core` class whose roster row is a reason is not written or read by the bytes carrier. A check
  of its slot values at `decode` would bring it back (`crates/nvs-stdlib/src/instance.rs:143`,
  `rule:classes/serialize-is-a-closed-format`).
- A `Core` descriptor with no registry row, such as a syntax-tree node, is not read back
  (`crates/nvs-stdlib/src/instance.rs:124`, `crates/nvs-stdlib/src/instance.rs:565`).
- Row hydration (`queryAs`, `streamAs`) and `Core\Test::double` and `partial` were not traced for an
  object built without its constructor (`rule:security/reflection-enforces-visibility`).
- A `private` constant a subclass declares again is refused to that subclass by name. A
  declaring-class label on the row would make it exact (`crates/nvs-runtime/src/object.rs:1123`).
- `call_callable_with` keeps eight inline slots, so a callable call with more than seven arguments
  allocates once (`crates/nvs-runtime/src/callable.rs:201`).
