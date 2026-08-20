# Working on MWL

MWL is a JIT-compiled, memory-safe language for web servers and the command line. It exists to run web
requests and CLI programs **securely and fast**.

This is the only file you always need to read. It carries the rules that are easy to get wrong, plus a
table telling you the *one* other file to open for whatever you are doing.

**Every fact in this repository has exactly one home.** If two documents state the same thing, the one
named below is authoritative and the other is a bug — fix it rather than reconciling it in your head.

## Where to look

Read this file, then **one** row below. Do not read the docs tree breadth-first; it is 120 KB and most of
it is reasoning you only need when you are about to overturn a decision.

**First, run `sh .claude/brief.sh`.** One call, ~11 KB: the plan's status block, the current and next
milestone, every ADR's decision paragraph, what the guard tests actually hold with their thresholds, and
what exists on disk. It stores no facts — it slices the live files and names each source, so it cannot go
stale, and it says so loudly if a slice comes back empty. Open a row below when you need the *reasoning*
behind a decision, or the detail of something the brief only names.

| Doing this | Open this |
|---|---|
| Deciding what to build next; scoping a milestone; checking what exists | [docs/implementation-plan.md](docs/implementation-plan.md) — the status block at the top, then your milestone. This is the plan of record. |
| Exceptions, the call ABI, helper signatures, panic containment | [ADR 0002](docs/adr/0002-error-propagation.md). It holds the only normative copy of the calling convention, and it *supersedes* any unwinding language you find elsewhere. |
| Extensions, wasm, WIT, `.mwlx` | [ADR 0003](docs/adr/0003-extension-system.md) |
| Weighing memory against safety, speed or simplicity | [ADR 0004](docs/adr/0004-memory-for-simplicity.md) |
| `mwl.ini`, `ini_set`, limits, capabilities | [ADR 0005](docs/adr/0005-config-changeability.md). Holds the only copy of the directive layout. |
| `spawn script`, isolates, the request boundary | [ADR 0006](docs/adr/0006-isolated-script-execution.md) |
| Types, `uint`, `array<T>`, unions, `mixed`, conversions, array keys | [ADR 0007](docs/adr/0007-explicit-type-system.md). Holds the only copy of the type grammar, the conversion table, the arithmetic result types and the list of deliberate divergences from PHP. |
| `enum`, enum cases, backing type, anything enum-shaped | [ADR 0010](docs/adr/0010-enums-are-a-value-type.md). Holds the only copy of enum semantics — a closed, named integer type like C#'s, not PHP's class-like construct; PHP's enum design is deliberately disregarded in full. |
| `static`, `global`, scoping, closure capture, where state may live at all | [ADR 0008](docs/adr/0008-static-and-global.md). Holds the only copy of the list of storage classes, and the one place `static`'s five PHP meanings are sorted into kept and rejected. |
| Free functions, global constants, the `Core` namespace, where a built-in lives | [ADR 0011](docs/adr/0011-functions-and-constants-are-class-members.md). Holds the only copy of the rule that every callable and every constant is a class member, and the `Core` domain-class shape built-ins are organised into. |
| `$_SERVER`, `$_GET`/`$_POST`, `$_SESSION`, `$_ENV`, `$GLOBALS`, `$_REQUEST`, `$argv`, or anything else PHP populates ambiently | [ADR 0012](docs/adr/0012-no-superglobals.md). Holds the only copy of the rule that no variable is ever host-populated — each becomes a `Core\Server`/`Core\Request`/`Core\Session`/`Core\Env`/`Core\Cli`/`Core\Script` call, and `$GLOBALS`/`$_REQUEST` have no replacement at all. |
| Comparing two objects with `<`/`>`/`<=`/`>=`/`<=>`, operator overloading, `Comparable`, `compareTo` | [ADR 0013](docs/adr/0013-comparable-interface.md). Holds the only copy of the rule that ordering two objects requires implementing `Comparable`; PHP's ambient property-walk fallback is rejected outright, and there is no cross-class overload. |
| Property hooks, `__get`/`__set`, `PropertyObserver`, undefined properties, `__call`/`__callStatic` | [ADR 0014](docs/adr/0014-property-observer.md). Holds the only copy of the rule that a property access runs its own hook first and a declared `PropertyObserver` second; accessing an undeclared property is always a hard error, and `__call`/`__callStatic` are not implemented at all. |
| `class_alias`, `use … as …`, trait-use `as`, or a `type` alias | [ADR 0015](docs/adr/0015-no-name-aliasing.md). Holds the only copy of the rule that nothing gets a second runtime-reachable name — `class_alias` does not exist, import renaming is rejected, and trait composition keeps only `insteadof` — while a `type` alias is kept as a distinct, compile-time-only synonym for a type expression, never for a single bare class. |
| A decision with no ADR — thread-per-core, value layout, safepoints, the unit cache, shared-nothing requests | [docs/adr/README.md](docs/adr/README.md) § *Decisions taken at project start* for **why**; the plan's § *Architecture* for the **mechanics**. That split is deliberate. |
| Any measured number, or checking whether an architecture assumption still holds | the guard tests in [benches/abi-probe/](benches/abi-probe/). The tests are the source of truth; docs quote them and can lag. |
| What the language should *do* | nothing yet — `docs/spec/` is unwritten. Say so rather than inferring semantics. |

Each ADR opens with a metadata block and reaches `## Decision` within ~60 lines. Read those two. The
`## Context`, `## Investigation` and `## Alternatives rejected` sections are for when you intend to
*change* the decision — skip them otherwise.

## The priority ordering

Highest first. A lower item is spent to buy a higher one, never the reverse. Reasoning and bounds:
[ADR 0004](docs/adr/0004-memory-for-simplicity.md).

1. **Security and request isolation** — not traded for anything.
2. **Correctness of language semantics** — PHP-compatible observable behaviour.
3. **Latency and throughput on the request path.**
4. **Simplicity** — of the language surface first, then of the implementation.
5. **Memory footprint** — last, and spent deliberately to buy any of the above.

When choosing between designs:

- Prefer the safer, faster or simpler one even when it holds more memory. MWL is **not** a low-footprint
  runtime; "allocates less" is not on its own a reason to change anything.
- If a change spends memory, **say what it spends** — per request or per task — in the doc comment or ADR
  that records it.
- Memory must stay attributable to a request and under an enforceable cap, and must be O(in-flight) rather
  than O(requests served). Growth with total traffic is a leak, not a trade-off.
- Bytes *moved* are not cheap. An allocation or extra cache miss on a hot path is a latency question
  (priority 3), not a footprint one.
- Saving memory at the cost of an invariant every future contributor must remember is the wrong direction —
  that is the account the unsafe modules are already drawing on.

## Ground rules enforced elsewhere

- **`unsafe` is forbidden workspace-wide**; only `mwl-runtime`, `mwl-codegen` and `benches/abi-probe` opt
  down to `deny` with narrow, reasoned allows. Lint policy is in [Cargo.toml](Cargo.toml).
- **Nothing unwinds through a JIT frame.** Errors propagate as a checked `i32` status after every call, and
  helpers are `extern "C"` wrapping `catch_unwind` — never `extern "C-unwind"`
  ([ADR 0002](docs/adr/0002-error-propagation.md)). `panic = "unwind"` is load-bearing in every profile:
  `abort` would turn a containable bug into a process kill. No caller constructs a raw `call` instruction;
  a missing status check would silently swallow an exception.
- **Pure-Rust dependencies by default**, enforced by [deny.toml](deny.toml) in CI. Deviations are argued
  individually.
- **Extensions are sandboxed wasm, never `dlopen`** ([ADR 0003](docs/adr/0003-extension-system.md)).
- **An isolate shares nothing but compiled code, and spends its parent's budget.** `spawn script` runs
  another `.mwl` file in-process ([ADR 0006](docs/adr/0006-isolated-script-execution.md)). Three invariants
  no optimisation may trade away: values cross by copy under the *same* rules as worker dispatch (never a
  pointer, never a shared heap, or non-atomic refcounts break); limits are accounted at the **root of the
  request tree**, never per isolate, or memory stops being attributable and the process worst case becomes
  unbounded in what a script chooses to spawn; and a child's grants are its parent's, optionally narrowed.
  Isolation is one implementation shared with request handling — if you find yourself writing a second
  arena setup or a second teardown path, that is the bug.
- **Nothing is untyped, and no type ever changes by itself.** Every binding — parameter, property,
  constant, local, loop variable, closure parameter, return — declares a type, and that declared type is
  fixed for its lifetime. A value's type changes only through an explicit checked conversion that throws
  rather than coercing, or by using a second binding. `mixed` is the *only* unchecked position, it is where
  untrusted input lands, and getting a value out of it is an explicit conversion — that is the security
  argument, not an ergonomic detail. `int` is signed, `uint` is unsigned, `int + uint` is a compile error,
  and integer overflow throws rather than silently becoming a `float`. Array keys are **always** strings.
  ([ADR 0007](docs/adr/0007-explicit-type-system.md)). If you find yourself writing type *inference* in the
  compiler, stop: it belongs in `mwl convert`, and the absence of it is what pays for the mandatory
  annotations.
- **Every function is a method, and every constant is a class constant; there is no free function and no
  global constant.** `function` and `const` are rejected with a diagnostic naming the replacement anywhere
  outside a class body — no exception for the standard library. Built-ins live under `Core`, a reserved
  namespace organised into domain classes (`Core\Str`, `Core\Arr`, `Core\Math`, …), one per PHP-extension-
  shaped grouping rather than one class holding everything; a call reaches them through ordinary `use`/
  fully-qualified resolution, with nothing auto-imported
  ([ADR 0011](docs/adr/0011-functions-and-constants-are-class-members.md)). Anonymous functions and arrow
  functions are unaffected — they are values, not named declarations, and creating one inside a method body
  or the script's own frame is not the free-floating name this rule closes.
- **`static` marks a class member; nothing else holds state behind a function's back.** Static methods,
  static properties and late static binding (`static::`, `new static()`, `: static`) are kept exactly as
  PHP has them. A function-scope `static` variable and a `static` closure are **rejected with a
  diagnostic naming the replacement**, and `global` does not exist
  ([ADR 0008](docs/adr/0008-static-and-global.md)). State that outlives a call lives in a class static, a
  constant, or an object property, and that list is exhaustive — a top-level `$x` is a local
  of the script's own frame and no function can reach it. A closure captures `$this` only when its body
  uses it. If you find yourself adding a second per-isolate slot table so one keyword can survive a
  return, that is this decision being undone.
- **No variable is ever populated by the host — PHP's superglobals do not exist.** `$_SERVER`, `$_GET`,
  `$_POST`, `$_COOKIE`, `$_FILES`, `$_SESSION` and `$_ENV` become `static` method calls on reserved `Core`
  classes (`Core\Server`, `Core\Request`, `Core\Session`, `Core\Env`), populated per isolate by the host;
  the CLI SAPI's `$argv`/`$argc` become `Core\Cli`, and MWL's own spawn-script `$_ARGS` becomes
  `Core\Script::args()`. `$GLOBALS` and `$_REQUEST` are dropped outright, with **no** replacement — there is
  nothing left for `$GLOBALS` to expose once every top-level variable is already unreachable from a function
  per the rule above, and `$_REQUEST`'s only job was hiding which of `$_GET`/`$_POST`/`$_COOKIE` a value
  came from. Inside a spawned isolate, `Core\Request`/`Core\Server`/`Core\Session` throw rather than
  returning the parent's data ([ADR 0012](docs/adr/0012-no-superglobals.md)).
- **Ordering two objects requires the global `Comparable` interface; there is no property-walk fallback.**
  `<`, `>`, `<=`, `>=` and `<=>` between two objects lower to a call to `compareTo(self $other): int`
  (negative/zero/positive, like `<=>` on scalars); a class that does not implement `Comparable` makes those
  operators a **compile-time diagnostic**, not PHP's ambient recursive property-by-property comparison. Two
  different classes are never directly orderable, even when both implement it — there is no cross-class
  overload. `==`/`===`/`!=`/`!==` are untouched by this and keep their existing behaviour
  ([ADR 0013](docs/adr/0013-comparable-interface.md)).
- **A property access runs its own hook first, then a declared `PropertyObserver` second; there is no
  `__get`/`__set`-by-name and no `__call`/`__callStatic` at all.** Per-property `get`/`set` hooks stay
  exactly PHP 8.4's. A class additionally implementing the global `PropertyObserver` interface
  (`onPropertyGet(string $name, mixed $value): void`, `onPropertySet(string $name, mixed $value): void`) has
  those methods called, purely as an observer, after every property's own hook (or plain storage) has
  already settled the value — `PropertyObserver` never overrides what a read returns or what a write stores,
  and it runs whether or not the specific property being accessed has its own hook. Accessing a property
  that is not declared on the class is **always a hard error** — a compile-time diagnostic for a literal
  name, a checked throw for a computed one — so unlike PHP, `__get`/`__set` are never reached as a fallback
  for a missing property; there is no such fallback. `__call`/`__callStatic` are not recognized by name
  anywhere: calling an undeclared method is already a diagnostic, and a method literally named `__call`
  compiles as an ordinary method the runtime never invokes on its own
  ([ADR 0014](docs/adr/0014-property-observer.md)).
- **Nothing gets a second runtime-reachable name.** `class_alias()` does not exist in `Core` and never will;
  `use Path\To\Name as Other;` is a diagnostic, not an import — a class, interface, trait or enum is reachable
  only under its declared short name or a fully-qualified path; and trait composition keeps only `insteadof`
  — both the renaming and the visibility-only forms of trait-use `as` are rejected, with the replacement
  being an ordinary overriding method that calls `TraitName::method()` explicitly. The one alias kept is a
  new **`type Name = TypeExpr;`** declaration: a compile-time-only synonym for a type *expression*, erased
  entirely by the checker, that may not name a single bare class/interface/enum on its own — that case is
  import aliasing wearing the type grammar as a disguise, and is rejected the same way
  ([ADR 0015](docs/adr/0015-no-name-aliasing.md)).
- **Architecture assumptions are tested, not remembered.** [benches/abi-probe/](benches/abi-probe/) guards
  the ABI, coroutine, sandbox and cost claims on every CI run. If a change makes one of those tests fail,
  the ADR it points at needs revisiting — do not adjust the threshold to make it pass.

## Commands

```sh
cargo build                                                    # debug; deps still built at opt-level 2
cargo test                                                     # unit + integration
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test --release -p mwl-abi-probe                          # cost guards (skipped in debug)
cargo test --release -p mwl-abi-probe --features wasm-probe     # + sandbox probes (pulls in Wasmtime)
```

## Writing docs here

The docs are optimised for an agent that reads one file and starts working. Keep them that way:

- **State a fact once.** Put it in the home named in *Where to look*, and link to it from everywhere else.
  Summarising an ADR into another document creates a second copy that will silently go stale — that is how
  the plan came to carry a superseded ABI cost and a superseded unwinding design at the same time.
- **Never quote a measured number outside the ADR that owns it.** Numbers live with their guard test.
- **Front-load.** Decision first, reasoning below it. Assume the reader stops after the first screen.
- A choice that would be expensive to reverse gets its own numbered ADR if the reasoning is subtle or
  contested; otherwise a paragraph in *Decisions taken at project start* in
  [docs/adr/README.md](docs/adr/README.md).
- Crates for later milestones are created when their milestone starts, not left sitting empty.
