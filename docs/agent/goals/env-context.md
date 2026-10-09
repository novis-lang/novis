---
milestone: M8
position: last
---
# Loop goal — `Core\Env::context()` says what kind of run the code is in, and `Core\Env::OS` is a `Core\Env::Platform` case

A program cannot ask whether it is answering an HTTP request or running from the command line. The
only signals are side effects of other members: `Core\Server::traceId()` throws outside a request
(`crates/nvs-stdlib/src/server.rs:136-139`), `Core\Request::path()` throws the same way, and
`Core\Server::isDraining()` returns `false` in both (`server.rs:30-35`). `Core\Cli::arguments()` does
not help either: it returns an empty list inside a request (`crates/nvs-stdlib/src/cli.rs:130-141`).
So 59 files in the tree pick their half by catching `LogicError $noRequest`, and 19 more by catching
the `RuntimeError` a nested `Core\Test::request` throws.

This goal adds **`Core\Env::context(): Core\Env::Context`**, one case per kind of run the runtime
already tells apart, one per row of `rule:tooling/echo-always-has-a-sink`'s table:

```nvs
if (Core\Env::context() == Core\Env::Context::Request) {
    echo html`<p>Hello</p>`;
} else {
    echo "Run me with nvs serve.\n";
}
```

It also makes **`Core\Env::OS` a value of a new enum `Core\Env::Platform`**, so
`Core\Env::OS == Core\Env::Platform::Windows` replaces `Core\Env::OS == "Windows"`.

## Why here

The user asked for both on 2026-10-09 and placed the goal directly after goal `nvs-code`. It runs
after goal `class-scoped-enums`, so both enums are written once, as class members:
`Core\Env::Context` and `Core\Env::Platform`, beside `Core\Env::Mode`. Its file set is the context's
constructors in `nvs-runtime`, `nvs-host`'s isolate builder, the four hosts that build a run
(`nvs-cli`'s `main.rs`, `runner.rs`, `worker.rs`, `serve.rs`), `nvs-stdlib`'s `env.rs` and registry,
and the `.nvs` files that detect a request by catching an error. Goals `pdf` and `spreadsheet` touch
none of it. It carries `position: last` because every goal on the chain is pinned there.

## Stage 0 — the catch-up

Each sentence is rewritten whole by the session that makes it untrue, and not before:

- **By Stage 2:** `crates/nvs-runtime/src/ctx/mod.rs:1279` names a guard test,
  `a_finished_task_fits_the_stack_switch`, that does not exist. The guard is
  `a_tasks_entry_callable_leaves_the_stack_switch_room_to_spare` (`crates/nvs-host/src/scheduler.rs:1489`).
- **By Stage 3:** `crates/nvs-stdlib/src/env.rs:6-8` (the module doc counts three members and calls
  the class complete); `docs/spec/01-core-library.md:1183-1187` (the `Core\Env` row names one enum);
  `docs/rules/tooling/echo-always-has-a-sink.md` gains one sentence: its rows are
  `Core\Env::Context`'s cases.
- **By Stage 4:** `env.rs:172-179` (`OS`'s `desc` says a string); `tools/nv/cmd/reference.ts:90-97`
  (the placeholder quotes a string); `docs/spec/01-core-library.md:1183-1184` (`OS` is untyped there).
  `docs/novis.md` and `docs/ground-rules.md` are generated and are regenerated, never edited.

## Stage 1 — the floor

Whatever the chain carries when this goal is reached. Every check goal `nvs-code` turned green stays
green.

## Stage 2 — the keystone: every context knows its kind of run

**Does:** Gives every `Ctx` one answer to "what kind of run is this", written where the context is
built. **Today the runtime does not record it as one fact**, and three rows cannot be told apart at
all. `Ctx::carrier` (`crates/nvs-runtime/src/ctx/output.rs:494`) has two values: an HTTP request
attaches the HTML sink and everything else keeps the terminal one. A job and a `spawn script` child
are the same `nvs_host::Isolate` with nothing set. What does exist:

| Kind | What already marks it |
|---|---|
| `Request` | `Ctx::inbound` is `Some` (`crates/nvs-runtime/src/ctx/inbound.rs:85`, field `ctx/mod.rs:1287`), written by `nvs_host::Isolate::answering` (`crates/nvs-host/src/isolate.rs:304`, applied at `:715`) |
| `Connection` | `Ctx::has_peer` (`ctx/isolate.rs:51`) or `Ctx::has_event_stream_connection` (`ctx/output.rs:739`), written by `over_socket` and `over_event_stream` (`nvs-host/src/isolate.rs:478`, `:507`, applied at `:732-746`). `Core\Topic` already asks exactly this (`crates/nvs-stdlib/src/topic.rs:447`) |
| `Schedule` | nothing on the context. The fire is `Isolate::new(..).recording(..)` (`crates/nvs-cli/src/serve.rs:1812`) |
| `Job` | nothing. The job is `Isolate::new(..).narrowed_by(..).run(ctx)` (`crates/nvs-cli/src/worker.rs:1460`) |
| `Isolate` | nothing. `spawn script` is `Isolate::new` in `start_isolate` (`crates/nvs-host/src/group.rs:257`); the context is `Ctx::isolate` / `Ctx::method_isolate` (`ctx/isolate.rs:425`, `:504`) |
| `Test` | nothing. A `#[Test]` method runs in `run_the_test` (`crates/nvs-cli/src/runner.rs:1105`) |
| `Command` | nothing. The CLI program's context is `Ctx::stdout()` (`crates/nvs-cli/src/main.rs:3133`) |

One file set: `crates/nvs-runtime/src/ctx/mod.rs`, `ctx/wiring.rs` (`Ctx::new`, `:98`),
`ctx/isolate.rs`, `crates/nvs-host/src/isolate.rs`, `crates/nvs-host/src/group.rs`,
`crates/nvs-cli/src/serve.rs`, `worker.rs`, `runner.rs`.

- **The type:** a `Copy` enum in `nvs-runtime`'s `ctx` module with the seven variants, and
  `Ctx::run_kind()`. It returns `Request` whenever `inbound` is `Some` and `Connection` whenever a
  peer or a connection event stream is attached, whatever was stored. Otherwise it returns a stored
  one-byte field: `Command` from `Ctx::new`, `Isolate` from `Ctx::isolate` and its two siblings.
- **The writers:** a builder on `nvs_host::Isolate` that stores the kind on the child context it
  builds (`isolate.rs:712-746`). `serve.rs:1812` stores `Schedule`, `worker.rs:1460` stores `Job`,
  `runner.rs`'s test isolate stores `Test`. Nothing else writes it.
- **Pinned by** the Stage 2 checks.

## Stage 3 — the member

**Does:** Adds `Core\Env::context()`, its enum, its rule and its spec row.

One file set: `crates/nvs-stdlib/src/env.rs` (the `CLASS` rows at `:114`, `mode` at `:136`, the `MODE`
enum at `:281`, `address` at `:310`, the `mode` helper at `:429`), `docs/spec/01-core-library.md`,
`docs/rules/core-classes/`, `tests/conformance/core/`, a new `crates/nvs-cli/tests/env_context.rs`.

- **The member, whole in one slice:** the registry row, its reference card, the helper (one read of
  `Ctx::run_kind`), the `address` arm and the three `.nvst` cases. A row without its cases fails the
  three-case floor, so they land together.
- **The enum** `Core\Env::Context`, registered the way `Core\Env::Mode` is after goal
  `class-scoped-enums`.
- **The four kinds a `.nvst` cannot reach** — `Schedule`, `Job`, `Test`, `Connection` — and the served
  `Request` are proved by `crates/nvs-cli/tests/env_context.rs`, through the real host for each.
- **The rule:** create `core-classes/env-context` from § *Standing decisions*, with the tradeoffs.
  Add the sentence to `tooling/echo-always-has-a-sink` (Stage 0).
- **Pinned by** the Stage 3 checks.

## Stage 4 — `Core\Env::OS` is a `Core\Env::Platform` case

**Does:** Types the constant as an enum and moves every reader off the string.

One file set: `env.rs` (the `OS` row `:172-179` and the mapping `:193-216`),
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-types/src/core_lib.rs`, `tools/nv/cmd/reference.ts`,
the spec row, `tests/conformance/core/env-the-three-constants-are-closed-facts-and-not-values-to-print.nvst`,
and the 25 Process files: `benches/members/core/Process*`, `tests/hostile/core/Process*` and
`tests/conformance/core/process-*`.

- **The registry can already state it.** `Const::EnumCase` (`registry.rs:1044`) exists for option
  defaults, the checker lowers it to the case's integer (`core_lib.rs:452`), and `nvs meta` and the
  editor stubs already print it (`crates/nvs-cli/src/meta.rs:455`, `crates/nvs-lsp/src/stubs.rs:518`).
  What refuses it is the agreement guard `every_registered_constant_matches_its_declared_type`
  (`registry.rs:6140`): it gains the pair `(CoreTy::Enum(e), Const::EnumCase(e, _))` with the same
  `e`. Check that a constant goes through `core_lib::constant` (`core_lib.rs:285`) to a folded
  integer. If it does not, that path gains the arm, and nothing else changes.
- **The mapping** at `env.rs:193` names a case instead of a string. The six cases stay the six
  families it has today.
- **The readers:** the Process files compare with `Core\Env::Platform::Windows`. The conformance case
  `env-the-three-constants-…` loops over the cases instead of strings and keeps its expected output.
  The placeholder at `reference.ts:97` names the case.
- **Pinned by** the Stage 4 checks.

## Stage 5 — the feature proofs

**Does:** Writes the feature proofs for `Core\Env::context`, the one new feature on the roster. An enum
and a constant are not features (`bun nv proofs --group 'Core\Env'` lists methods only).

One file set: `docs/examples/core/Env/context/`, `tests/hostile/core/Env/context/`,
`benches/members/core/Env/context.nvs`.

- **`about.md`** says what each case means in one line each.
- **The examples:** print the kind of run; one file that serves a page in a request and tests itself
  from the command line; a `match` over all seven cases.
- **The attack:** a request tries to look like a command-line program: through a header, an
  environment variable, `Core\Config::set`, and a `spawn script` child that reports back. Every answer
  is `Request` for the request and `Isolate` for the child.
- **The bench:** one call per round, constant, with 0 allocations and 0 calls declared.
- **Every user-facing text** follows AGENTS.md § *Text an end user reads*.
- **Pinned by** the Stage 5 checks.

## Stage 6 — readers stop catching an error to find out

**Does:** Moves every file that picks its half by catching an error to `Core\Env::context()`.

One file set: the files the two searches below print.

- `git grep -l -F 'catch (LogicError $noRequest)' -- '*.nvs' '*.nvst' '*.md'`: 59 files in
  `benches/members/core/{Html,Response,Test,Test-Response}`, `docs/examples/core/{Test,Test-Response}`,
  `tests/hostile/core/{Test,Test-Response}`, `tests/conformance/core` and
  `website/snippets/guides/testing/test-a-route`.
- `git grep -l -E 'catch \(RuntimeError \$(inside|insideTheRequest)\)' -- '*.nvs' '*.nvst'`: 19 files,
  mostly the `Core\Html::later` and `Core\Response::slotted` examples and `tests/conformance/core`.
- **A `catch` whose subject is the throw stays.** `docs/examples/core/Server/traceId/03-a-command-line-program-has-no-id.nvs`,
  `tests/conformance/core/server-trace-id-throws-where-no-request-arrived.nvst` and
  `an-in-process-request-may-not-be-made-from-inside-one.nvst` prove the error itself.
- An example's comment is rewritten whole for the new line. A bench edited outside its timed loop
  re-records its figure with `bun nv proofs --record-perf` if `bun nv proofs --gate` asks (not checked
  whether it will).
- **Pinned by** the Stage 6 check.

## Standing decisions

- **The user's calls, as instructions.** `Core\Env::context()` returning `Core\Env::Context`, one case
  per row of the echo table. `Core\Env::OS` keeps its name and becomes typed `Core\Env::Platform`, with
  the six cases `Windows`, `Darwin`, `Linux`, `BSD`, `Solaris`, `Unknown`. The name `Platform` keeps
  the enum from differing from `OS` only by case. **The user may still rename it**, so the name is
  written in the registry row and the spec and nowhere a rename would have to hunt for it. No decision
  record. Placed after goal `nvs-code`.
- **The goal writer's calls, not confirmed by the user, also standing.**
  - **The case names** are `Request`, `Command`, `Isolate`, `Schedule`, `Job`, `Test`, `Connection`.
    A script's main task and a `#[Command]` method are both `Command`. A `.nvst` program is `Command`:
    `nvs test` runs it as a script, not as a `#[Test]` method.
  - **`Request` and `Connection` are read from what makes them one**, never from the stored byte, so
    a host that forgets to store a kind cannot make a request answer `Command`. A program may branch on
    the answer to print more on the command line (Security and request isolation).
  - **A limit handler's isolate** (`Ctx::handler_isolate`, `ctx/isolate.rs:568`) answers `Isolate`. It
    has no request of its own, as `rule:security/request-state-throws-in-an-isolate` says of every
    isolate. Fallback if a test shows it carries the request: it answers `Request`, and the rule says so.
  - **The member never throws** and takes no capability. It is the same kind of process fact as
    `Core\Env::mode`.
- **What it spends.** `context()`: one byte per in-flight context, and likely none if it fits the
  struct's padding (not checked). A context crosses the scheduler boxed (`scheduler.rs:281`), so the
  field cannot break a task's start. Per call: two pointer tests and one load, no allocation. Nothing
  per request beyond that byte. `Platform`: nothing at run time. The constant is folded at compile time
  to an integer, so a comparison is an integer compare where it was a string compare.
- **The tradeoffs**, stated here and in the rule because AGENTS.md asks. Performance: none on the
  request path. Memory: as above. Usability: a program asks one member instead of catching an error.
  A misspelled platform or kind of run is a compile error, where a misspelled string compared
  `false`. The user expects a `match` over either enum to be checked for every case. No rule or
  case says a `match` missing a case does not compile (not checked further), so the rule states
  only what a test pins. Printing a platform needs a `match` to the case name (`rule:enums/no-class-machinery`), where the
  string printed directly. Simplicity: one member, two enums, and no new limit or setting.
- **Neutral names only** in every test, example and rule: `Shop`, `Blog`, `example.com`.
