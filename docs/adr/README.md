# The decision records

A decision record is the reasoning behind a rule: what was asked, what was considered, why this
answer, and what it costs. The rule itself lives in [docs/rules/](../rules/) — the chapters are the
read surface, [docs/ground-rules.md](../ground-rules.md) is one line per rule, and
`docs/divergences.md` is every rule that departs from PHP, all three generated
from the rulebook by `bun nv rules --render`. A record is where you go to *change* a rule,
never to learn one; `bun nv brief --where <keyword>` routes a topic to the rule that owns it.

**Where they live.** `docs/decisions/NNNN.md`, one file per decision, frozen on acceptance. Each opens
with a YAML block — `status` (`accepted` or `retired`) and `changes`, the rules the decision
created and the ones it modified, derived from every rule's `because` — then the title, `Scope`,
`Depends on` and `Validated by` where the decision has them, the **In short** paragraph, and the full
reasoning under `## Context` / `## Decision` / `## Consequences` / `## Alternatives rejected` /
`## Revisiting` / `## Verification`. Nothing in a record is maintained after acceptance: there is no
`Amends:`, no `Amended by:` and no folding. A later decision that changes a rule edits the rule's
fragment under `docs/rules/` and names the earlier record only through its own `because`. This file
sits in `docs/adr/` for historical reasons; the records moved at the docs migration's unit C1.

**A section number is a public identifier.** `0007 § 3` is cited from `crates/`, from the goal
manifests and from other records. Sections are never renumbered, and `bun nv records --check`
reports a citation into a section that does not exist.

**Measured numbers.** A record quotes only its own measurements, and every one is guarded by a test in
[`benches/abi-probe`](../../benches/abi-probe/). The tests are authoritative; a number written anywhere
else is a copy that can go stale.

Numbering starts at 0002 and has gaps: **0032** was folded into [0029](../decisions/0029.md) § 1
before the freeze, and **0140** and **0141** were never claimed. The project-start decisions below
are what 0001 would have been. [tooling-parity.md](tooling-parity.md) — the PHP tool ecosystem, tool by tool, and where each job
landed — still lives beside this file.

## Decisions taken at project start

Recorded here rather than as individual ADRs. Promote one to its own file if it is ever seriously
challenged.

**Rust as the implementation language.** Memory safety in the runtime is a product requirement, not an
implementation preference; Cranelift, the async ecosystem and the pure-Rust protocol crates all live here.

**Cranelift JIT as the only execution tier, no interpreter.** Chosen for peak performance and a single
semantics implementation to keep correct. The cost is that the first runnable program requires the whole
front end plus a working backend. Mitigated by shipping a *baseline* tier where every operation lowers to a
call into a Rust runtime helper — mechanically close to an interpreter loop, therefore quick to get
correct — with typed inlining layered on later behind the same IR boundary.

**`exit` is a fourth ABI status, not a `FATAL` carrying a code.** `nvs_runtime::EXITED` sits beside `OK`,
`THROWN` and `FATAL`, and the status `exit(n)` named rides out on the request context rather than in the
status word. A `FATAL` is a *failure* — `rule:errors/escalation-ladder`'s tier 3, reported at
the request boundary as one — while `exit(0)` is the most ordinary end a PHP program has, so folding them
together would report every clean exit as an internal error. What the two do share is propagation: neither
is catchable, because `nvs_ir::ir::Terminator::Catch` admits only `THROWN`, and **neither runs a
`finally`**, which is PHP's own behaviour for `exit` — checked against `php -r`, not assumed, and therefore
priority 2 rather than a simplification. The cost is one more constant that every status check already
handles by comparing against `OK`, and one `i64` per request. The frame's locals are still released,
because `exit` lowers to an ordinary helper call carrying `rule:errors/propagation`'s error edge. `exit("message")` is PHP's
other spelling of the same construct: the message is written and the status is `0`; anything that is
neither an `int` nor a `string` is a type mismatch at the operand, since `rule:types/conversion` has no implicit
conversion to offer there.

**SIMD is a dependency's job, and the JIT emits scalar code.** No `target-cpu` flag is set anywhere, on any
platform: LLVM autovectorizes the Rust crates at each target's *baseline* ISA — SSE2 on x86_64, NEON on
aarch64 — and no further, because a `native` build produces a binary that faults on the next machine. The
wide, feature-detected SIMD that actually earns its keep arrives through dependencies that hand-wrote it and
dispatch at runtime: `memchr` behind every `regex` prefilter ([0056](../decisions/0056.md)), `blake3`
on the artifact-cache path ([0042](../decisions/0042.md)). That is the same trade the
pure-Rust-dependency rule below already makes — the `unsafe` lives in a fuzzed crate with a user base rather
than in ours, where `unsafe_code = "forbid"` and a stable-pinned toolchain (so no `std::simd`) bar it
anyway. When a byte-scanning leaf turns out to be hot — UTF-8 validation, grapheme scanning,
[0024](../decisions/0024.md)'s HTML auto-escape — reach for such a crate, never for
`core::arch` intrinsics. Cranelift, meanwhile, has no autovectorizer: its vector instructions exist to lower
wasm's fixed 128-bit SIMD, not to be discovered from scalar loops, so JIT-compiled Novis is scalar by design.
Little is lost — a request's hot path is refcounting, ordered-hash lookups and tagged dispatch, not the
dense homogeneous loops a vectorizer needs — and vectorizing a `float` reduction would reassociate its
additions, which the priority ordering's rank 2 forbids outright. Two things to know before anyone claims a
win: [0026](../decisions/0026.md)'s instruction counts flatter SIMD, because
callgrind does not model vector port throughput, and Cranelift's own vector support is shaped by wasm's
fixed 128-bit SIMD, which caps anything built on it there regardless.

**Thread-per-core, shared-nothing runtime.** One single-threaded executor pinned per core; a request is
assigned to a core and never migrates. This is what makes value refcounts *non-atomic* (a heap is only ever
touched by one thread), makes cross-request state contamination structurally impossible rather than merely
prevented, and still uses every core — parallelism comes from N independent executors. Compiled code is
immutable and therefore shared across all cores through `Arc` with no copying.

**Stackful coroutines for suspension.** Validated by spike #3, now the guard tests
`a_helper_can_suspend_with_jit_frames_live_above_it` and `a_coroutine_round_trip_stays_cheap` in
[`benches/abi-probe`](../../benches/abi-probe/). The decisive property is the absence of *function
colouring*: any Novis function may perform I/O and yield without being marked `async`, so converted PHP call
chains become concurrent with no rewriting. The cost is a stack per in-flight task (default 64 KiB,
configurable, grown lazily) and a small audited unsafe core for stack switching, taken as a dependency
(`corosensei`) rather than hand-rolled. The memory is paid deliberately, under
[0004](../decisions/0004.md).

**Isolated workers for CPU parallelism.** Work dispatched to another core gets its own heap; values
crossing the boundary are deep-copied, or moved when the refcount is 1. Data races are impossible by
construction rather than by discipline, which is what lets the refcounts stay non-atomic. The same rules
govern the script-level boundary in [0006](../decisions/0006.md), deliberately: one set of
value-crossing rules, not two — and [0023](../decisions/0023.md) gives that one
rule its formal definition, shared with `serialize()`/`unserialize()`.

**Strict shared-nothing requests.** Only compiled code survives a request. The consequence — reconnecting
to the database every request — is accepted for v1; `nvs-host` reserves an unused `PersistentRegistry` seam
so pooling can be added later without redesign. The same isolation is reachable from inside the language:
`spawn script` runs another `.nvs` file as a child isolate of the request tree, and an inbound request is
simply the root isolate of its tree, so both paths are one implementation.

**Safepoints emitted from the first backend commit.** A poll at every loop back-edge and function entry is
the single mechanism behind CPU-time limits, client-disconnect cancellation, the cycle collector, the
profiler, debugger breakpoints and later deoptimisation. Retrofitting it would mean rewriting codegen, so
it is not deferrable.

**Server-level configuration, not per-project.** `nvs.toml` is root-owned, TOML
([0064](../decisions/0064.md)), and per-app capability blocks live in the *root* config so an
application can never grant itself rights. What a script may change about its own configuration at runtime
is per-directive and is argued in [0005](../decisions/0005.md).

**Pure-Rust dependencies by default.** A memory-safe runtime cannot contain arbitrary C. Deviations are
explicit, argued and few — currently only SQLite (`rusqlite`), where no credible pure-Rust implementation
exists. The admission test is [0051](../decisions/0051.md) § 4's.

**Domain logic is an existing first-class Rust crate; compiler passes and scheduler primitives are ours.**
The neighbouring question to the one above — not *what may we depend on*, but *what may we write ourselves*.
Anything with an external specification (a protocol, a parser, a wire format, a cipher, a codec, a cron
expression, a timezone database) is a dependency, and **if no first-class crate exists, the feature is not
built** — a second-rate implementation of somebody else's specification is a security surface we would then
own forever. Anything about *Novis's own* compiler or scheduler has no possible crate and is ours by nature.
It is the same split the project already lives with: Cranelift compiles, and the IR lowering into it is
ours; `serde_json` parses, and the derive that emits Novis IR from Novis types
([0071](../decisions/0071.md)) could not be a crate if we wanted it to be. When a feature is half of each,
say which half is which before writing either.

**Checked-return call sites go through one code path.** See [0002](../decisions/0002.md): a missing
status check would silently swallow an exception, so no caller constructs a raw `call` instruction.

**`unsafe` is confined to named crates, each declaring its own policy.** The workspace sets
`unsafe_code = "forbid"`; crates that genuinely need it opt down to `deny` and allow individual blocks with
a stated reason. Currently `nvs-runtime`, `nvs-codegen`, `nvs-stdlib` and `benches/abi-probe`, which must
call JIT-compiled code to measure it. The probe is `publish = false` and is not a dependency of anything
shipped, so it does not widen the runtime's unsafe surface.

**`continue` inside a `switch` continues the enclosing loop.** PHP counts a `switch` as a looping structure
for `continue`, so a bare one there behaves as `break` — and PHP has warned since 7.3 that you probably
meant `continue 2`. Novis takes the meaning that warning points at: `switch` owns `break` and nothing else, so
`continue` always means the innermost enclosing loop. The alternative is a keyword that silently means one
thing inside a `switch` and another everywhere else, which the priority ordering's simplicity rule refuses
to buy for a compatibility PHP itself discourages. `nvs_ir::lower::Lowering::lower_switch` implements it and
`tests/differential/lang/a-switch-and-a-match-agree-with-php.nvst` pins it against PHP's `continue 2`.

**A written level counts the way PHP counts, and `continue` then walks outward.** `break N`/`continue N`
name the `N`-th enclosing statement, and a `switch` is one of them for *both* keywords — that is PHP's rule,
and departing from it would silently retarget `continue 2` inside a `switch` inside two nested loops from
the inner loop to the outer one, which is the one outcome worse than a diagnostic. A level that lands on a
`switch` frame then looks further out for a loop, which is the paragraph above generalized from the bare
keyword to a written level: it makes `continue 2` inside a `switch` mean in Novis exactly what it means in
PHP, and leaves the divergence exactly where it already was — a level naming a `switch` for `continue`
continues the loop rather than breaking the `switch`. A level with nothing to name is
`E0475` (`nvs_types::locals::check_exit_level`), never a panic: a non-literal level, a `0`, a level past the
enclosing depth, and `continue` with no loop at or outside its frame. PHP refuses all four at compile time
too. `nvs_ir::lower::Lowering::lower_break`/`lower_continue` lower the rest, and
`tests/conformance/lang/a-break-leaves-the-level-it-names.nvst` pins the whole file byte-for-byte against
PHP's output.

**A ternary's or a `match`'s branches join at the union's erasure, and widen at the binding.** Two branches
that lower to two representations are not reconciled by promoting one into the other: the checker has
already typed the whole expression as the *union* of its branches, and `nvs_ir::lower`'s `erase_checked_ty`
erases a union whose members do not share a representation to the tagged one, so the phi carries that and
`nvs_ir::lower::Lowering::join_representations` tags each branch in its own block. This is the same line
integer `/` draws and for the same reason ([0007](../decisions/0007.md) §§ 2 and 4): § 4's promotion
rows belong to an *operator*, whose result type that table fixes, and § 2's implicit `int`→`float` widening
happens at a `float` **position** — a binding, a parameter, a `return`. A ternary branch is neither, so
`$c ? 1 : 2.5` keeps PHP's answer on its truthy path (an `int`, not `1.0`, and exact past 2^53 where the
widening would have thrown) and `float $x = $c ? 1 : 2.5;` widens exactly once, where the declared type is.
An **arm-less** `match` is refused where it is written, `E0476`, rather than lowered: PHP parses one and
throws `UnhandledMatchError` on every evaluation, so no program that ran is lost, and a `match` is an
expression — one whose every path throws has nothing for the position it sits in to bind, pass or return,
and no value for a merge phi with no incoming edge to carry.

**A method call needs a class label, so an erased receiver is refused rather than dispatched.** `object` is
[0007](../decisions/0007.md) § 3's opaque top of every class type, and it erases to exactly the
pointer a named class does — `nvs_ir::lower`'s `erase_checked_ty` maps `CheckedTy::Object` and
`CheckedTy::Shape` onto the same `Ty::Object` a `CheckedTy::Class` gets, so nothing below the checker ever
wanted the label for *representation*, and `object` is a declared type in every position a class name is.
What does want it is *resolution*: `$o->m(...)` has no signature to check its arguments against and no
return type for the position it sits in. [0036](../decisions/0036.md) § 4 already answered the
**property** half of an erased receiver — a name-keyed runtime fetch, and a write checked against the
field's real declared type — and stopped at properties on purpose. The call half is therefore `E0477` where
it is written (`nvs_types::expr::calls::report_method_on_erased_receiver`), naming the two narrowings that
do resolve: `instanceof` proves the class inside the guarded branch, and `as ClassName` converts to it or
throws. There is no `__call` to fall back on ([0014](../decisions/0014.md)), and § 3's "a dynamic call
with runtime-checked arguments, at `mixed`'s cost" is deferred for `callable` and was never granted to
`object`. **It is one code across every receiver that names no class**, because it is one mistake and the
resolution it fails is the same one: a union naming no single class, an intersection, and the types that
can hold no object at all — a scalar, an `array<T>`, a `void` call's result — all take `E0477` too, with
only the help splitting (narrow it, or convert it, or nothing at all for a value that does not exist).
That is where the call half parts company with the property one, which splits a *deferral* off from
`E0495`: a property read through an erased receiver has a name-keyed fetch to defer to and a call has
nothing. `mixed` is the one receiver deliberately left out — [0007](../decisions/0007.md) § 2 makes
it the one unchecked position, so it defers rather than refuses, and until that lowering exists it is the
one shape `nvs-ir`'s own panic at `lower/expr.rs` still names; the paragraph below owns *how* it is
answered. Refusing is the reversible half of that pair: a later decision can turn this diagnostic into
dispatch, while a program that already dispatched could not be taken back.

**A call through a `mixed` receiver is marshalled by the receiver's own descriptor, not by a per-method
thunk.** [0036](../decisions/0036.md) § 4 grants the deferral and says nothing about the
convention, so this paragraph is its home. Almost nothing has to be marshalled at all, which is the fact the
design turns on: [0002](../decisions/0002.md) makes **one** calling convention normative for every
call, so `nvs_runtime::abi::NvsFn` is already a context, an array of 16-byte tagged `Value`s and one tagged
`out` slot, and `nvs-codegen`'s `store_value`/`load_value` already write each argument and each return
*with* its tag while a typed callee reads only the payload half. A site holding tagged values therefore has
tagged slots to fill, and the answer comes back tagged for the `mixed` the call's own type is — no
conversion in either direction, and a narrower binding takes `as T` exactly as one holding a `callable`'s
result does. What is missing is not the marshalling but the callee's **declared shape**: how many parameters
it takes and which tag each one requires, without which the callee reinterprets slot *i* at its own
representation and an `int` handed to a `string` parameter is an arbitrary dereference rather than a fault —
the identical hole `nvs_runtime::closure`'s own module docs describe for `callable`, arrived at from the
other side. So the method row on `nvs_runtime::ClassDesc` carries them, the way a closure object already
carries `FN_ARITY` and `FN_PARAM_TAGS`: the same nibble word, the same `CLOSURE_PARAM_TAG_ANY` for a
parameter whose representation *is* a tag, and `check_param_tags` as the one implementation both paths
share, so [0007](../decisions/0007.md) § 2's `int`-into-`float` widening is not written down a
second time to be got wrong differently. They are resolved once per class in `ClassTable::set_methods`, the
precedent `ClassDesc::renderer` and `ClassDesc::unwind` set, and cost a word and a byte per method per class
**once per process** — nothing per instance and nothing per call.

The alternative on the table was a tagged-ABI thunk per method, and it loses on three of the priority
ordering's five at once: `nvs-codegen` would emit the tag rules a second time, where a safety check wants
one implementation and not two (rank 1); a thunk is a second frame on the erased path and still needs the
same name lookup to be found at all, so it buys no dispatch (rank 3); and it spends a whole compiled
function per method in every unit whether any `mixed` receiver exists or not, against sixteen bytes on a
descriptor (rank 5). The statically typed path pays nothing either way and keeps its fixed label; the erased
path pays a tag test on the receiver, one binary search of the flattened method table by name, and a shift,
a mask and a compare per argument. Every failure a program can reach is a catchable throw on
[0002](../decisions/0002.md)'s error edge and never a fault, worded as the diagnostic that names the
same mistake where a static type shows it — a receiver whose tag is not an object (`E0477`'s reading), a
class whose table has no such name (`E0405`'s), and a count or a tag the callee does not admit (`E0402`'s
and `E0401`'s) — so one mistake reads one way whichever end sees it. Three shapes are answered by that
throw rather than by dispatch, each because the row cannot describe them and not as a rule about erasure: a
**non-`public`** member, since a `mixed` receiver is outside every class by construction and the row carries
the visibility bit that says so; a **variadic or `inout`** parameter list, which is packed and written back
at the *call site*, the limit `E0721` already names for [0043](../decisions/0043.md)
§ 4's synthesized forward; and a **`Core`**-owned class, whose members are native symbols that *borrow*
argument 0 where a compiled method owns its parameters — the very difference that made `renderer` its own
descriptor field rather than a row in the table, and reaching them from here wants a second field per
member that no case asks for yet.

**A closure parameter naming a class is checked against the argument's own ancestry, at the closure's
entry.** `nvs_ir::lower::param_tag_nibble` gives every class name — and `object`, and a shape — the same
nibble 7, a nibble naming a representation and four bits having no room for a label, so
`nvs_runtime::closure::check_param_tags` refuses a `string $c` handed a `Core\Cli\Text` and would accept
an unrelated `Marker $c` handed the same value. That acceptance was the **only** way a named-class binding
came to hold an instance of another class: every other position is checked where it is written, and
[0036](../decisions/0036.md) § 4's erased receiver carries no label at all and therefore defers
to a name-keyed fetch. A binding that *does* carry a label is read and written at a fixed offset, so the
lie is a type confusion rather than a wrong answer — two `final` classes and one wrong `Core\Arr::filter`
callback wrote an `int` over a `string` field and the next read dereferenced it — which is
[0004](../decisions/0004.md)'s priority 1 and not a matter of taste.

Two boundaries could pay, and the cheaper one is not the safer one's equal. Making every named-class
property access name-keyed would close it everywhere and spend priority 3 in every program, most of which
never write a closure at all. Checking the argument against the parameter's declared class **at the
closure's entry** spends one `nvs_object_instanceof` — one flattened linear scan of the ancestry
(`nvs_runtime::object::NvsObj::is_instance_of`) — per class-declared parameter per call, and only in the
position where nothing else looked. That is what `nvs_ir::lower::closure::check_param_class` emits: the
body's first block branches on the same `instanceof` `$x instanceof C` lowers to, and the miss throws a
`LogicError` in the sentence shape `check_param_tags` already writes. The tag word is unchanged and gains
no class channel — a per-closure-instance list of descriptors would spend an allocation at every closure
literal to answer a question the body's first block asks for free. Both lines are pinned from Novis by
`tests/conformance/core/out-a-callback-parameter-naming-a-class-checks-the-argument-class-at-entry.nvst`:
the tag word's around objecthood, the entry check's around ancestry, so a parent class and an implemented
interface both accept the instance an exact class does.

Three things the entry check deliberately does not do. A **`?C` parameter is unchecked on both lines**,
erasing to `nvs_ir::Ty::Tagged` before any class survives — the same nothing `mixed` gets, and for the
same reason. A **`Core` class** parameter keeps the objecthood-only check: a unit's class table is
`nvs_types::layout`'s declared tree, so there is no descriptor to compare against, and `instanceof` over a
`Core` class is not a spelling the checker admits either (`E0496`) — the only argument that could exercise
the row is the carrier a `Core` member hands its own callback, which is already of that class. And the
refusal **does not name the class that arrived**: no IR instruction reads an object's class name, so
widening `must be of type Marker, another class given` to PHP's `…, App\Holder given` means a new value
shape in `nvs-ir` and `nvs-codegen`, worth taking when something other than a message wants one.

**An `inout` parameter belongs only to a frame the call site outlives.** A by-reference parameter is a
contract between the two ends of one call: `nvs_ir::lower::call` stages a cell at the site, hands the callee
its address, and copies back when the call returns — sound precisely because the callee's frame dies first.
Two declarations break that ordering, and both are refused where they are written rather than lowered. A
**generator** inverts it outright: calling one runs none of the body, it allocates the state object and
returns ([0053](../decisions/0053.md) § 4), so the staged cell is gone before the first
`advance()` while the parked frame would still be addressing it — `E0492`,
`nvs_types::check::check_generator_by_ref_params`. A **closure** has no call site that could stage anything:
§ 2's by-value capture lets it outlive every frame in scope where it was written, so there is no frame whose
death the copy-back could be ordered against — `E0493`,
`nvs_types::expr::calls::report_by_reference_parameter`. A written signature
([0136](../decisions/0136.md)) does not reopen it: that gives a site a parameter list to
read and still no call site at which to stage a cell. Neither is a lowering we
chose not to write: there is no representation either could keep instead, because copying the value in would
stop being a reference, which is the whole observable point of `inout`. The replacements are the ones those
ADRs already name — for shared mutable state, § 2's ordinary object captured by value; for a generator,
taking the value and `yield`ing what the body computes from it. **Capturing** an enclosing `inout` parameter
is a different question and is *not* refused: § 2's capture is by value, so what it owes is a snapshot of
the cell's value at the literal, which `nvs-ir` has not written yet (its gap 9). **The spelling is
[0107](../decisions/0107.md)'s** — `inout` before the type and
again at the call site, `&` rejected in every by-reference position — and this paragraph states the rule in
it; the tree still spells it `&` until that ADR's M4 items land.

**Architecture assumptions are tested, not remembered.** Several decisions here rest on how Cranelift,
`corosensei` and Wasmtime behave rather than on our own code, and a dependency bump can invalidate them
silently. `benches/abi-probe/` checks them on every CI run, including the *premise* of
[0002](../decisions/0002.md) — that native unwinding through JIT frames is unavailable — so if that
ever changes we are told rather than left paying for a workaround that is no longer needed. It also guards a
premise about the *platform we are replacing*: that an OS process costs orders of magnitude more than a
task, which is the whole cost argument for [0006](../decisions/0006.md).

**A `static` property's storage is the request's, not the process's.** One slot per declared static per
in-flight request, armed from the declaration's own initializer when the request's `nvs_runtime::Ctx` is
built and released when it is dropped. The priority ordering settles it at rank 1: a process-global static
is a channel from one request into the next, so a token cached in one is readable by whoever sends the
next request, and no amount of care in user code closes that. `nvs run` cannot tell the two apart — a CLI
run is one request — so the divergence is invisible until `nvs serve` at M7, which is exactly when the
wrong default would have become expensive. What it costs is the PHP idiom of a process-lifetime memo,
which [0006](../decisions/0006.md) had already removed by making a script's world
per-execution; a cache that must outlive a request is a `Core` capability with a stated lifetime, not a
class variable. Because a static therefore has no constructor to assign it, its declaration must carry an
initializer unless its type admits `null` (`E0409`), and `static::$prop` — which PHP re-resolves against
the *called* class — is refused rather than silently answering the writing class's slot (`E0499`).
`nvs_runtime::ctx`'s module docs own the mechanism and what it spends.

**A diagnostic band is two digits wide, and a full one continues in a new band rather than running past
its end.** `E04xx` — types — filled at `E0499`, and the max-plus-one rule would have yielded `E0500`,
whose own digits read as `E05xx`: IR and codegen. A code is a promise that its number alone says which
stage produced it, and `nvs_diagnostics::code`'s legend table is where that promise is written down, so
`E0500` is never issued and the types band continues at **`E07xx`**, one more row in that table, opening
at `E0700`. Both alternatives cost more than a second range: widening every band to three digits
renumbers two hundred released codes and every `.nvst` case that names one, and filling the lowest hole
inside `E04xx` reuses a retired number, which the same promise forbids. `bun nv orient` reports a band
whose max-plus-one would leave it as **full** rather than handing out the number past its end, so the
next session reads this decision off the tool instead of re-deriving it. `E08xx` was held unallocated for
whichever band filled next, and types is what filled it — a second time, at `E0799` — so
[0136](../decisions/0136.md) opens **`E08xx` at `E0800`** for the diagnostics a
written `callable` signature needs, a third row in that legend table for the one stage. `E10xx` is the
reserve that replaces it, `E09xx` being internal compiler errors.

**A `class`, `interface` or `enum` is declared at file scope, or not at all.** PHP declares a nested type
when the statement *runs*, so `if ($legacy) { class Session { … } }` makes the very existence of a name a
run-time fact. Novis resolves every type name against a static table built before any code runs — the
`require`/autoload graph of [0021](../decisions/0021.md), then `nvs_hir`'s member and
hierarchy tables, then `nvs_types::layout`'s field offsets — and none of those has a reading to give a
name that may or may not exist yet. So a declaration written inside a method body, a property hook, a
closure body or a nested block at file scope is `E0233` where it is written, reported by
`nvs_types::locals`' per-body walk, which is reached only from inside a body and therefore needs no
"am I nested" test of its own. Both cheaper readings were rejected: declaring it unconditionally at file
scope silently changes the program PHP wrote, and admitting a conditional entry in the class table would
put a run-time question inside every name resolution, layout and dispatch decision below it — priority 4,
paid for once here rather than at every later lookup. What it costs is PHP's conditional-class idiom,
whose two real uses — a polyfill and a feature switch — are `require` of one file or the other, which is
static and already works. An anonymous class — PHP's `new class { … }` — is the same nested declaration in
expression position and is refused the same way: it has no name for the static table to hold, and its two
real uses, a one-off implementation of an interface and a test double, are a named class in the same file
or a closure ([0031](../decisions/0031.md)).

**A `try` has at least one `catch` or a `finally`, and a `catch` clause names exactly one class.** PHP
refuses a bare `try { … }` too, and the parser here accepted it only by omission. `catch (A | B $e)` is
refused: `$e` carries one static type ([0007](../decisions/0007.md) § 1's `catch` row), and a body
shared by two classes is written as two clauses or as one clause on their common ancestor — the shape
[0119](../decisions/0119.md) § 4 already requires of
an expression arm. Both are parse-time refusals in the rejected-PHP band, and each names its rewrite.

**A class or interface constant writes its type.** PHP 8.3's untyped `public const LIMIT = 9;` parsed
here and took the type of the value it folded to. That reading is a guess the declaration never made,
and it is not available everywhere the form is: an interface constant records no value for `nvs-ir` to
inline and panicked the lowerer at the first use, and a value with no constant form at all (`[1, 2]`)
has nothing to read a type from either. Every other binding
[0007](../decisions/0007.md) § 1 governs — property, parameter, return, local, `foreach`,
`catch` — already writes one, so the omission was the single hole in that rule, and closing it removes
both panics by construction rather than one shape at a time. It is `E0246` at the `const` keyword, in
the rejected-PHP band, and the rewrite is the type the value already has. What it costs is one word
per constant in a migrated file; what the inference did is now a check against what was written.

**`namespace X;` is the only namespace statement — once per file, before any declaration.** The braced form
`namespace X { … }`, and with it a file holding two namespaces, is refused at parse time.
[0112](../decisions/0112.md) keys authority on the namespace enclosing the
code and [0104](../decisions/0104.md) keys an application on its entry file; a file
that is two namespaces is a file whose authority is a function of the line number, and nothing below either
ADR has a reading to give that. The rewrite is one file per namespace.

**One `use` names one import; PHP's group form is refused.** `use App\Models\{User, Post};` is `E0238`
where the `\{` is written, sibling to [0015](../decisions/0015.md) § 2's `E0212` on the other half of
the same statement. That ADR's rule is that a name is reachable under exactly the spelling it was declared
with, and the group form does not break it — every short name it introduces is still that name — so this
is priority 4 rather than priority 2: a second spelling of a statement that already exists, whose payoff
is fewer lines and whose cost is that the set of short names a file introduces is no longer a `grep` for
`^use` returning one name per line. Novis takes the line-per-import reading, the same trade the alias
refusal already made. The parser reports and then skips the brace group, so the statement still yields a
`UseDecl` for the prefix that was written and nothing downstream sees a half-parsed import.
