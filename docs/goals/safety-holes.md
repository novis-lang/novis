---
looper: 1
title: Four holes the 0.0.5 review reproduced are closed
summary: "Reached when an earlier operand keeps its value while a later operand moves the same local, every helper's effect row matches what its runtime body runs, a class reached through a shape is held to the decode checks, every use form of a revoked object throws, each of the four has a test that enumerates its forms, and the features that moved have their proofs and their website entries."
context: [docs/rules/types/a-replaced-value-is-changed-in-place.md]
stages:
  - id: 1
    title: an earlier operand keeps its value when a later operand moves the same local
    status: done
    context: [docs/rules/expressions/an-operand-keeps-its-value-while-later-operands-run.md, crates/nvs-ir/src/lower/operand.rs]
  - id: 2
    title: "every helper's effect row matches what its runtime body runs"
    status: done
    context: [crates/nvs-ir/src/effects.rs, crates/nvs-ir/src/ownership/lent.rs]
  - id: 3
    title: a test build refuses a function whose borrowed value outlives the move of its owner
    status: done
    context: [crates/nvs-ir/src/ownership.rs]
  - id: 4
    title: a class reached through a shape is held to the decode checks
    status: done
    context: [docs/rules/security/derived-codec-qualifiers.md, crates/nvs-types/src/derive.rs]
  - id: 5
    title: every use form of a revoked object throws
    status: open
    context: [docs/rules/classes/revocable-objects.md, crates/nvs-codegen/tests/revocable.rs]
  - id: 6
    title: "the features that moved have their proofs, their pages and their change entries"
    status: open
    context: [website/src/data/changes.json]
  - id: 7
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: operand-moves
    stage: 1
    name: "an element, a property and a string read before a moving operand are read whole after it, for every operand site and both kinds of move"
    run: [cargo, test, --test, operand_moves]
    timeout: 1800
  - id: helper-effects
    stage: 2
    name: "a program whose toString, compareTo or encode hook replaces a lent value gives the same output with in-place on and off"
    run: [cargo, test, --test, helper_effects]
    timeout: 1800
  - id: borrow-verifier
    stage: 3
    name: "the verifier refuses a function that reads a borrowed value after its owner was moved, and passes every function the suites lower"
    run: [cargo, test, --test, borrow_verifier]
    timeout: 1800
  - id: decode-through-shape
    stage: 4
    name: "a class under a shape, at every depth and at every typed decoder, gets E0810 and E0820 as a class at the root does"
    run: [cargo, test, --test, decode_through_shape]
    timeout: 1800
  - id: revoked-uses
    stage: 5
    name: "every use form of an object has a row that says throws or does not throw, and a revoked object does what its row says"
    run: [cargo, test, --test, revocable]
    timeout: 1800
  - id: proofs
    stage: 6
    name: "every feature the goal's work reached has its proofs"
    run: [bun, nv, proofs, --gate, --goal, safety-holes]
    timeout: 1800
  - id: close
    stage: 7
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, safety-holes]
    timeout: 900
---

# Four holes the 0.0.5 review reproduced are closed

## Why

A review of the work between `v0.0.4` and `v0.0.5` ran small valid programs against the release
binary. Four of them show a hole in the first priority, security and request isolation. Each one is
written here as the program that shows it, so a session starts from a reproduction.

**1. A moved local frees a value an earlier operand still reads.** The lowering leaves `$a["k"]`
unretained when no later operand writes `$a` (`later_can_free`,
`crates/nvs-ir/src/lower/operand.rs:441`). The ownership pass then moves `$a` into a later operand's
call, because `$a` is not read again (`candidates`, `crates/nvs-ir/src/ownership.rs:1799`). The callee
frees the array, and the concatenation reads a freed string.

```nvs
class P {
    public static function len(array<string> $x): string {
        $x = [];
        array<string> $fill = [];
        for (int $i = 0; $i < 64; $i++) { $fill[] = Core\Str::repeat("y", 64); }
        return "n";
    }
    public static function main(): void {
        array<string> $a = ["k" => Core\Str::repeat("x", 64)];
        echo "v=" . $a["k"] . P::len($a), "\n";
    }
}
P::main();
```

It prints `v=n`. Under `--no-in-place` it prints `v=`, 64 `x` and `n`. `nvs run --dump-ir` shows
`array.get` with no `retain`, then the call with no `retain` before it. An owning `Core` member does
the same: `"v=" . $a[0] . Core\Arr::count(Core\Arr::withoutFirst($a))`.

**2. A helper is listed as running no program code, and it runs `toString()`.**
`Helper::ToStringOrNull` is in the `false` arm of `may_run_user_code` (`crates/nvs-ir/src/ir.rs:4629`).
Its body calls `convert_to_string` (`crates/nvs-runtime/src/helpers.rs:2398`). A function that reaches
program code only through `as ?string` gets an empty effect summary, so its caller lends a static
property to it.

```nvs
class Reg {
    public static array<string> $items = [];
    public static function add(array<string> $xs, mixed $m): array<string> {
        $xs[] = Core\Str::length(($m as ?string) ?? "") > 0 ? "t" : "f";
        return $xs;
    }
}
class Evil {
    public function toString(): string { Reg::$items = []; return "zz"; }
}
Reg::$items = [Core\Str::repeat("a", 40), Core\Str::repeat("b", 40)];
Reg::$items = Reg::add(Reg::$items, new Evil());
echo Core\Json::encode(Reg::$items), "\n";
```

The process ends with a panic at `crates/nvs-runtime/src/array.rs:2820`, exit 127. Under
`--no-in-place` it prints the three entries.

**3. A class reached through a shape skips the decode checks.** `Core\Request::jsonAs<User>()` is
`E0810` when `User::$name` is a plain `string`. `Core\Request::jsonAs<{ user: User }>()` compiles with
no error, and `$in->user->name` is request text typed as an untainted `string`. The shape branch of a
decode site records no deferred site (`crates/nvs-types/src/expr/args.rs:2350`), and the class walk
gives no label to a shape (`crates/nvs-types/src/derive.rs:2439`). `E0820` is skipped the same way, so
a class that cannot be built under a shape ends a request with a fatal
(`crates/nvs-stdlib/src/json.rs:5358`).

**4. Four uses of a revoked object do not throw.** With `class Post implements Revocable, Stringable`,
after `$post->revoke()`: `$post->title .= "!"` writes the property, `P::bump(inout $post->views)`
reads and writes it, a method reference made before the revoke runs the method, and `"{$post}"` runs
`toString()`. `$post->hello()` throws, as `rule:classes/revocable-objects` says every use does.

The priority ordering decides every trade here: security and request isolation are not traded for
latency. Each fix costs at most one retain and release pair, or one flag test, at the site it guards.

## Standing decisions

**Hole 1 is fixed where the borrow is made, in the lowering.** `later_can_free` answers `true` for a
local when any later operand is not inert and names that local, not only when it writes it. A local
passed by value, spread, joined or handed to a `Core` member can be moved by the ownership pass, so it
can free what the earlier operand read. Reason: one judgment at the one place every operand site
already asks, and the ownership pass needs no new knowledge. It costs one retain and release pair on
the earlier operand, only in an expression where a later operand uses the same local. Safe default if
a site leaves a borrowed read unretained without asking `later_can_free`: that site asks it too.
Declined: teaching the ownership pass which instructions borrow from which value and refusing the
move. Every instruction added later would have to be listed there, and a missing row is a use after
free.

**The rule is `rule:expressions/an-operand-keeps-its-value-while-later-operands-run`.** Its fragment
gains the moved local beside the written one, with this reason. No new rule is written.

**Hole 2: a helper's row is `true` unless its runtime body is read and runs no program code.** Stage 2
reads the body of every helper in the `false` arm once. Each row that stays `false` keeps a one-line
comment that names the runtime function it was read against. `ToStringOrNull` moves to `true`. A
helper whose body converts, compares, encodes or clones a value that can be an object runs program
code.

**A `Core` member that takes an array runs program code when the array's elements can be objects.**
`takes_an_object` (`crates/nvs-ir/src/effects.rs:256`) reads an array argument's element type. An
element type of a class, an interface, `mixed`, a union with one of them or a callable counts as an
object. An array of scalars stays quiet. Safe default where the element type is not known: it counts.

**A member that can park its task is not quiet, if a sibling task can run meanwhile.** Stage 2 settles
it by reading the scheduler, once: can another task of the same request run while a `Core` member is
parked inside a lend? If it can, the effect row gains "may suspend", and a lend does not cross a call
that has it. If it cannot, the fragment of `rule:types/a-replaced-value-is-changed-in-place` says why
in one sentence, and nothing is built.

**Stage 3's verifier runs in test and debug builds only.** After the ownership pass, it walks each
function: a value read from an element or a property with no retain of its own must not be read after
an instruction that consumes the value it was read from. A missing row in its list of borrowing
instructions is a missed check, never a wrong program, which is why this knowledge may live here and
not in the pass. It costs nothing in a release build.

**A call that may park counts as writing anything.** Stage 2 read the scheduler: a
`Core\Task::all` group's callables are children on the same core and share the request's objects and
static properties, so a sibling runs while a task waits. `effects::may_park` names every `Core` call
outside `Core\Str`, `Core\Math`, `Core\Arr` and `Core\Regex`, and such a call makes a summary
"anything". Reason: one prefix list, safe by default for a class added later. It costs the lend on a
callee that does I/O, which then copies at its change. Declined: a "may suspend" bit per member in the
registry. A missing bit would be a use after free.

**A release is not program code in an effect row.** A helper that reaches program code only through a
release stays `false`, as `InstKind::Release` does. Reason: the rows describe a call, and a release
is everywhere; the one release that runs program code, a suspended generator's `finally`, is a
question for every release and is in `### Backlog`.

**Hole 3: one walk for a decode target, and it steps through shapes.** The walks that hold a decoded
class to `E0810`, `E0820` and codec membership step into a shape's fields, an array's element and a
nullable, at any depth, and a shape at the root records its site. Every typed decoder is covered:
`Core\Request::jsonAs`, `queryAs`, `postAs`, `Core\Json::decodeAs`, `decodeEach` and
`Core\Arr::shapeAs`. A program that compiled through the hole gets `E0810` with its help line. That is
the fix, and the change entry says so. The run-time fatals at `crates/nvs-stdlib/src/json.rs:5358` and
`:6087` stay as the last line.

**Hole 4: the test is a table over use forms, written from the grammar.** Stage 5 lists every
expression and statement kind of `nvs-syntax` that can use an object, and gives each a row: throws, or
does not throw with the sentence of `rule:classes/revocable-objects` that says so. A kind the rule does
not name throws. A static method called through an instance throws too, because the rule's promise is
"every use". The check for a method reference runs when the reference is called, not when it is made.
The runtime's by-name call (`call_method`, `crates/nvs-runtime/src/dispatch.rs:93`) checks as
`call_erased_unchecked` does. Each check is compiled only where the receiver may be revocable, as now.

**Tests live in `crates/nvs-codegen/tests/`, one file per hole, and run a program both ways.**
`operand_moves.rs`, `helper_effects.rs` and `borrow_verifier.rs` are new, and `revocable.rs` gains the
table. `decode_through_shape.rs` is a checker test in `crates/nvs-types/tests/`. A use after free does
not always change the output, so `operand_moves.rs` also reads the lowered function and asserts the
`retain` before the moving call. Fixtures are written in the test files, with the neutral names
`Blog`, `Post`, `Shop` and `Cart`.

**What stops it.** One attempt per item, and two sessions per stage, counted in `### State`. Stage 3
is the only one that may be declined: if the verifier refuses functions the suites lower and the cause
is not a real borrow after a move, its items move to `### Backlog` with the count of refused functions,
the `borrow-verifier` check is deleted, and the goal goes on. No stage gains an item.

**Lanes.** One lane. Stages 1, 2, 3 and 5 share `crates/nvs-ir/src/lower/` and `ownership.rs` and run
in order. Stage 4 touches `crates/nvs-types` alone and may run as a side goal. This goal runs ahead of
`windows-rust-lld`, which keeps its own handoff.

## Stages

### 1. an earlier operand keeps its value when a later operand moves the same local

**Split:** an `Explore` subagent lists every site in `crates/nvs-ir/src/lower/` that leaves a read of
an element or a property unretained, as `file.rs:NN` and whether it calls `later_can_free`.

- [x] `crates/nvs-ir/src/lower/operand.rs:441`: `later_can_free` answers `true` for
  `ReadRoot::Local` when a later operand is not inert and names the local.
  `rule:expressions/an-operand-keeps-its-value-while-later-operands-run`
- [x] `crates/nvs-ir/src/lower/expr.rs:3417`: each site the subagent listed that does not ask
  `later_can_free` asks it. `emit_concat`'s leading piece at `:3462` is one.
  - [x] `emit_concat`'s leading piece is judged by `hold_borrowed` at `expr.rs:3417`, so the new
    answer reaches it with no change there.
  - [x] The four sites that do not ask need no hold: the switch subject (`control.rs:652`) is always
    retained when borrowed; the slot append's suffix (`stmt.rs:843`) and the carrier self-append
    (`stmt.rs:1472`) read a bare local whose own value stays live, so the ownership pass cannot move
    it; the `??` left side (`expr.rs:1586`) is consumed before its right side runs.
- [x] `crates/nvs-codegen/tests/operand_moves.rs:1`: the program of § *Why* 1, then one case per
  operand site (concatenation, binary operator, call argument, `Core` call argument) times the move
  (a known Novis callee, an owning `Core` member, a spread) times the value read (array element,
  object in an array then its property, string property). Each runs with in-place on and off and
  asserts the `retain`. It runs once, by name.
- [x] `tests/conformance/in-place/an-operand-read-before-a-moved-local-is-read-whole.nvst:1`: the
  first program, with its output.
- [x] `docs/rules/expressions/an-operand-keeps-its-value-while-later-operands-run.md:18`: the moved
  local, with the reason and the Declined line of the standing decision.
- [x] `cargo build` and `cargo clippy --all-targets -- -D warnings` are green.

### 2. every helper's effect row matches what its runtime body runs

**Split:** a `reader` subagent takes the `false` arm of `may_run_user_code` and reports, per helper,
the runtime function and whether it can reach `convert_to_string`, `call_method`, a compare, an encode
or a clone. The session decides each row from that list.

- [x] `crates/nvs-ir/src/ir.rs:4629`: `ToStringOrNull` is `true`, and so is every helper the list
  shows reaching program code. Each row left `false` names its runtime function in a comment.
  `rule:types/a-replaced-value-is-changed-in-place`
  - [x] The reader's list: `ToStringOrNull` is the one helper that calls program code.
  - [x] Four helpers reach it only by a release; they stay `false`, as `Release` does.
- [x] `crates/nvs-ir/src/effects.rs:256`: `takes_an_object` reads an array argument's element type, as
  the standing decision says.
  - [x] The checker records a plain array argument (`ExprTypeTable::is_plain_array`).
  - [x] `CoreCall` carries a `plain` mask, and `object_operands` counts every other array.
  - [x] `callbacks_are_quiet` refuses a lent subject that may hold objects.
- [x] `crates/nvs-host/src/scheduler.rs:1268`: the suspension question is settled by reading, and
  either `crates/nvs-ir/src/ownership.rs:1691` (`quiet`) refuses a member that can park, or the rule
  fragment gains its one sentence.
  - [x] Yes: `effects::may_park`, read by `of_inst`, `quiet` and `callbacks_are_quiet`.
- [x] `crates/nvs-codegen/tests/helper_effects.rs:1`: the program of § *Why* 2, then one case per
  helper moved to `true`, one for a `Core` member whose array holds objects with a hook that replaces
  the lent value, and one for a `Core\Task::all` pair if the suspension answer was yes. Each runs both
  ways. It runs once, by name.
- [x] `tests/conformance/in-place/a-conversion-that-runs-tostring-does-not-free-a-lent-array.nvst:1`.
- [x] `docs/rules/types/a-replaced-value-is-changed-in-place.md:57`: what a quiet call is, as built.
- [x] `cargo build` and clippy are green.

### 3. a test build refuses a function whose borrowed value outlives the move of its owner

- [x] `crates/nvs-ir/src/ownership.rs:1997`: a verifier beside `releases_after`, behind
  `cfg(any(test, debug_assertions))`, run after the pass on every function, as the standing decision
  gives it. Its module comment lists the borrowing instructions and says what a missing one costs.
- [x] `crates/nvs-codegen/tests/borrow_verifier.rs:1`: a hand-built function that reads a borrowed
  element after its owner's move is refused, and the two programs of § *Why* pass after stages 1
  and 2. It runs once, by name.
- [x] `cargo build` and clippy are green. The gate stage is where every suite meets the verifier.

### 4. a class reached through a shape is held to the decode checks

- [x] `crates/nvs-types/src/expr/args.rs:2350`: a shape at a decode site records its deferred sites.
  `rule:security/derived-codec-qualifiers`
- [x] `crates/nvs-types/src/derive.rs:2439`: the taint walk steps through a shape, an array element
  and a nullable (`check_decode_sites`, `check_shape_decode_site` at `:2405`, `codec_class_label` at
  `:2378`).
- [x] `crates/nvs-types/src/derive.rs:2048`: `check_json_sites` does the same for `E0820` and codec
  membership.
- [x] `crates/nvs-types/tests/decode_through_shape.rs:1`: for each typed decoder of the standing
  decision, a class at the root, under a shape, under a shape in a shape, in an array of shapes and in
  a class's shape property. Each plain `string` property is `E0810`, each `tainted string` one
  compiles, and a class that cannot be built is `E0820`. It runs once, by name.
- [x] `tests/hostile/`: one attack beside the typed decoders' existing one, `// Attack:` request text
  that reaches a plain `string` through a shape. `bun nv proofs --id 'Core\Request::jsonAs'` names its
  folder.
- [x] `docs/rules/security/derived-codec-qualifiers.md:1`: the walk through a shape.
- [x] `cargo build` and clippy are green.

### 5. every use form of a revoked object throws

**Split:** an `Explore` subagent lists every `ExprKind` and `StmtKind` of `crates/nvs-syntax/src/ast.rs`
that takes an object operand, one line each. The table's rows come from that list.

- [ ] `crates/nvs-ir/src/lower/stmt.rs:657`: `lower_slot_append` checks before it touches the slot.
  `rule:classes/revocable-objects`
- [ ] `crates/nvs-ir/src/lower/call.rs:1452`: an `inout` property argument checks, and so does the
  union arm at `:1405` and the write back at `crates/nvs-ir/src/lower/mod.rs:4588`.
- [ ] `crates/nvs-ir/src/lower/anon_fn.rs:753`: a method reference checks when it is called, here and
  at `:977`.
- [ ] `crates/nvs-ir/src/lower/expr.rs:1305`: string conversion checks before `toString()`, and
  `crates/nvs-ir/src/lower/operator.rs:237` before `compareTo`.
- [ ] `crates/nvs-ir/src/lower/control.rs:1751`: `emit_iface_call` checks, so a `foreach` over a
  revoked iterable throws. The delegation forward at `crates/nvs-ir/src/lower/call.rs:1665` and the
  static call through an instance at `crates/nvs-ir/src/lower/expr.rs:5130` check too.
- [ ] `crates/nvs-runtime/src/dispatch.rs:93`: `call_method` checks, so `call_compare_to` at `:831`
  and every stdlib caller does.
- [ ] `crates/nvs-codegen/tests/revocable.rs:1`: the table of the standing decision, one row per kind
  the subagent listed, each compiled and run. A kind with no row fails the test by name. It runs once,
  by name.
- [ ] `tests/conformance/class/`: one case each for `.=`, an `inout` property, a method reference and
  string conversion, named like their siblings there.
- [ ] `docs/rules/classes/revocable-objects.md:1`: the uses that throw, as the table has them.
- [ ] `cargo build` and clippy are green.

### 6. the features that moved have their proofs, their pages and their change entries

- [ ] `website/src/data/changes.json:1`: `changed` `lang:classes/revocable-objects` (the four uses
  throw), and `changed` for each typed decoder stage 4 touched (a class under a shape needs
  `tainted`). The in-place fixes change no documented behaviour and get the entry kind the file uses
  for a fix.
- [ ] `docs/examples/lang/classes/revocable-objects/details.md:1` and
  `docs/examples/types/Revocable/about.md:1` are reread against the table and stamped with
  `bun nv site --stamp`.
- [ ] The `about.md` of each typed decoder stage 4 touched says that a class at any depth is checked.
- [ ] `bun nv proofs --owed --goal safety-holes`, and every proof it names is paid.
- [ ] `bun nv site --check stale`, `changes` and `review` are green.

### 7. the goal is verified once, and every failure is fixed

- [ ] The newcomer review of [prose.md](../agent/prose.md) § *A newcomer reads it* over the texts
  stage 6 wrote.
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 and 2 are done: `later_can_free` (`crates/nvs-ir/src/lower/operand.rs`) and
`effects::may_park`; `operand_moves` and `helper_effects` each passed once. Stage 3 is done. `crates/nvs-ir/src/ownership/borrows.rs` is the borrow verifier, compiled under
`cfg(any(test, debug_assertions))`. `ownership::run` runs it on every function after the pass and
panics with the first refusal: a read of an unretained element or property after its owner is
released, moved or consumed. `cargo test --test borrow_verifier` passed once, three cases. No other
suite has met the verifier yet.

Stage 4 is done. `derive::classes_reached` is the one walk: a decode site's shape records a
`DecodeSite` and a `JsonSite` for each class it reaches (`expr/args.rs`, the shape branch of
`written_class_of`), and `check_decode_sites` and `check_json_sites` walk each field's declared type
with it, so a class's shape property is reached too. `codec_class_label` is deleted. A row decoder
records no class through a shape. `cargo test --test decode_through_shape` passed once, four tests over
six decoders and eight depths, and failed three of four on the tree before the fix.
`tests/hostile/core/Request/jsonAs/03-a-plain-string-under-a-shape.nvs` is the attack, refused with
`E0810`. The nightly Miri failure in `nvs-stdlib 2/12` ran on `7e97fa8444`; `709a1947b2` fixed it.

Sessions per stage: stage 1, one; stage 2, one; stage 3, one; stage 4, one.

### Next

Stage 5 from its first item: the table of use forms in `crates/nvs-codegen/tests/revocable.rs`, written
from `nvs-syntax`'s expression and statement kinds, then each check: `lower_slot_append`
(`crates/nvs-ir/src/lower/stmt.rs:657`), an `inout` property argument (`lower/call.rs:1452`), a method
reference when called (`lower/anon_fn.rs:753`), string conversion (`lower/expr.rs:1305`),
`emit_iface_call` (`lower/control.rs:1751`) and `call_method` (`crates/nvs-runtime/src/dispatch.rs:93`).
Stage 6 owes the `about.md` of each typed decoder a sentence that a class at any depth is checked, and
a `changes.json` entry for `rule:security/derived-codec-qualifiers`'s features.

Tests expected to go red at the gate: IR snapshots under `crates/nvs-ir/src/lower/snapshots/` that
hold an expression where a later operand passes on the local an earlier operand read. Each gains a
`retain` and `release` pair and is accepted. Snapshots and ownership unit tests that lend a slot to a
`Core` call given an array of non-scalars, or to a callee that calls a `Core` member outside `Str`,
`Math`, `Arr` and `Regex`, now keep the retain; each is accepted where the array may hold an object or
the callee may park. A suite whose program the borrow verifier refuses panics with "the borrow
verifier refused a function". A real borrow after a move is fixed where the read is lowered. A refusal
that is not one follows the standing decision *What stops it*. Neither conformance case has a
`// covers:` marker yet; stage 6 adds them with the change entries. A program in a suite that decodes
a class with a plain `string` property through a shape now gets `E0810`; each is fixed by writing
`tainted` on the property.

### Backlog

- A release of the last reference to a suspended generator runs its `finally` bodies
  (`crates/nvs-runtime/src/object.rs:5126`, `unwind_abandoned`). An `InstKind::Release` counts as no
  program code, so a lend may cross a release that runs a `finally` which writes the lent slot. Found
  by stage 2's reader; not reproduced.
