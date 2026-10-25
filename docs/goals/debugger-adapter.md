---
looper: 1
title: "`nvs dap` drives VS Code's debugger UI over a launched program"
summary: "nvs dap launches nvs run, nvs test or a development nvs serve over stdio, steps over, into and out, honours conditions, hit counts, logpoints, exception filters, skipFiles and pathMappings, shows the value a step out returned, VS Code reaches it through a descriptor factory and a launch.json schema alone, every designed rule this goal owns is shipped, and the checks below pass."
context: [crates/nvs-cli/src/main.rs, crates/nvs-cli/src/serve.rs, crates/nvs-host/src/scheduler.rs, editors/vscode/src/extension.ts]
stages:
  - id: 1
    title: "`nvs dap` speaks DAP over stdio and launches a program"
    status: done
    context: [crates/nvs-cli/src/main.rs, crates/nvs-cli/src/lsp.rs, crates/nvs-cli/src/run.rs, crates/nvs-lsp/tests/stdout_policy.rs, docs/rules/ide/the-debug-adapter-does-not-wait-for-an-editor.md, docs/rules/ide/stdout-belongs-to-the-protocol.md]
    extra:
      context:
        rules: [ide/the-debug-adapter-does-not-wait-for-an-editor, ide/stdout-belongs-to-the-protocol]
  - id: 5
    title: "`nvs dap` launches tests and a development server, and stops at a breakpoint"
    status: done
    context: [crates/nvs-cli/src/dap/mod.rs, crates/nvs-cli/src/runner.rs, crates/nvs-cli/src/serve.rs, crates/nvs-host/src/debug.rs, docs/rules/ide/the-debug-adapter-does-not-wait-for-an-editor.md]
    extra:
      context:
        rules: [ide/the-debug-adapter-does-not-wait-for-an-editor, testing/a-stop-pauses-the-whole-task-tree, testing/a-debugged-program-is-compiled-with-a-debug-switch]
  - id: 2
    title: stepping over, into and out, and the value a step out returned
    status: done
    context: [crates/nvs-codegen/src/emit.rs, crates/nvs-ir/src/lower/expr.rs, crates/nvs-runtime/src/callable.rs, crates/nvs-runtime/src/ctx/frames.rs, crates/nvs-host/src/debug.rs, docs/rules/ide/the-debugger-ui-is-as-deep-as-the-adapter.md]
    extra:
      context:
        rules: [ide/the-debugger-ui-is-as-deep-as-the-adapter, testing/a-debugged-program-is-compiled-with-a-debug-switch, testing/a-stop-pauses-the-whole-task-tree]
  - id: 3
    title: conditions, hit counts, logpoints and exception filters
    status: open
    context: [crates/nvs-runtime/src/throwable.rs, crates/nvs-types/src/check.rs, crates/nvs-codegen/src/emit.rs, crates/nvs-cli/src/dap/mod.rs, docs/rules/ide/the-debugger-ui-is-as-deep-as-the-adapter.md]
    extra:
      context:
        rules: [ide/the-debugger-ui-is-as-deep-as-the-adapter, errors/escalation-ladder, security/redaction-reaches-the-tools-own-renderings]
  - id: 4
    title: VS Code launches `nvs dap`, and the adapter has its feature proofs
    status: open
    context: [editors/vscode/package.json, editors/vscode/src/extension.ts, editors/vscode/src/binary.ts, editors/vscode/test/host/surfaces.test.ts, editors/vscode/test/contributions/contributions.test.ts, docs/reference/tools/40-editor.md, docs/rules/ide/the-extension-builds-no-ui-the-editor-already-has.md]
    extra:
      context:
        rules: [ide/the-debug-adapter-does-not-wait-for-an-editor, ide/the-extension-builds-no-ui-the-editor-already-has, ide/the-debugger-ui-is-as-deep-as-the-adapter, ide/every-feature-is-staged-behind-its-dependency, testing/feature-proofs]
  - id: 6
    title: the goal is verified once, and every failure is fixed
    status: open
    gate: true
checks:
  - id: owners-closes
    stage: 4
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, debugger-adapter]
    reach: true
    floor: false
---

# `nvs dap` drives VS Code's debugger UI over a launched program

## Why

`debugger-core` gives the host a Rust controller that stops a request's task tree at a statement and
reads its frames and variables. Nothing outside a Rust test can use it: there is no `nvs dap` command
(`crates/nvs-cli/src/main.rs:395` is `nvs lsp`, and no `dap` variant exists), and the VS Code extension
contributes no debugger (`editors/vscode/package.json:29`). A developer still debugs with
`Core\Debug::dump`. This goal puts the Debug Adapter Protocol on top of the controller, adds stepping,
breakpoint conditions and exception filters, and wires VS Code's own debugger UI to it. The `ide/`
rules list every capability the adapter reports, and the editor adds no UI of its own.

## Standing decisions

**The gate is `debugger-core` reached.** Stages 5, 2 and 3 need its controller
(`crates/nvs-host/src/debug.rs`), its slot table and its frame records
(`crates/nvs-runtime/src/ctx/frames.rs`). The gate holds when `docs/goals/debugger-core.md` is gone and
both files exist. Stage 1 needs none of it: framing, `initialize`, a `run` launch without breakpoints
and the `output` events run on today's tree. If the gate does not hold after stage 1, the session ends
the group and says so in the handoff.

**Launch only, a development `nvs serve` included.** The user decided this. `nvs dap` launches `nvs run`,
`nvs test` or `nvs serve` in development, over stdio. Under serve, a breakpoint stops whichever request
reaches it. There is no attach to a running process, so there is no debug socket.
`rule:ide/the-debug-adapter-does-not-wait-for-an-editor` holds it. Trade-offs: nothing on the request
path of a production server, which cannot be debugged at all. Usability: a server must be started from
the editor to be debugged. Simplicity: no listener, no authentication and no second protocol.

**The stop is the request's task tree, and each task is a DAP thread.** The user decided this, and
`debugger-core` builds it (`rule:testing/a-stop-pauses-the-whole-task-tree`). This goal presents it:
`threads` lists the stopped tree's tasks, and every other request keeps being served.

**The program runs in the adapter's own process.** The adapter calls the controller directly, and the
program's stdout and stderr become `output` events. A child process would need a second protocol
between the adapter and the child. The cost: a bug in the launched program's output path could reach
the protocol's stdout, so a test reads every byte of stdout in each mode and checks that each one
belongs to a frame.

**The adapter is a module of `nvs-cli`, `crates/nvs-cli/src/dap/`.** It launches through the same code
`nvs run`, `nvs test` and `nvs serve` use (`crates/nvs-cli/src/run.rs:108`,
`crates/nvs-cli/src/runner.rs:490`, `crates/nvs-cli/src/serve.rs:227`). A crate of its own would need a
launcher interface back into `nvs-cli` for each mode. `crates/nvs-cli/src/main.rs` moves under every
session: find its anchors by name (`Lsp(`, `Command::Lsp`, `fn run_run`), never by a line kept here.

**The DAP messages are serde types written here,** in `crates/nvs-cli/src/dap/protocol.rs`. DAP's
framing is the `Content-Length` header LSP uses, and the adapter answers about twenty requests. A
protocol crate would add a dependency to read a schema that small, and security is the first priority.

**A launched `nvs serve` starts in `development` whatever `[mode] default` says,** written into
`rule:ide/the-debug-adapter-does-not-wait-for-an-editor`. The switch slows every statement and a stop
holds a request, so a debugged server is never a production one.

**A condition cannot call a function.** The user decided this, and it is written into
`rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`. A condition and a logpoint's holes read variables,
properties and array elements with operators and comparisons, and are type-checked against the frame's
slot table. A call is an error the editor shows on the breakpoint. A call at a breakpoint could write,
throw or `await`, and the program would then behave differently under the debugger. Usability cost:
`count($items) > 3` is not a valid condition.

**"Uncaught" is decided at the throw, from the statement each frame is at.** The lowering records,
under the switch, which classes the `try` blocks around each statement catch
(`nvs_ir::ir::Function::debug_catches`, a `catch` expression's classes on its own statement), and a
frame record already names its statement. Every landing block first calls `nvs_debug_throw`, before
it releases anything, so the stop sees the exception pending and every frame linked; a flag on the
context, cleared by a fresh raise and kept by a `catch` that re-raises the same object, makes each
exception stop once. This replaces a push and pop per `try`: nothing runs when a `try` starts, and a
helper's fault stops the same way a `throw` does. Cost: one call per landing block a throw passes,
under the switch only, one `bool` per context, and one class list per statement inside a `try`.

**Evaluating a watch expression is not built.** The rule does not ask for it, and the adapter does not
report `supportsEvaluateForHovers` or answer `evaluate` beyond a variable's own name. It is in the
Backlog.

**Trade-offs, all together.** Request path: nothing outside a debug session. Inside one, the switch's
cost (`debugger-core`), one call per landing block a throw passes, and a condition evaluated only where its
site is armed. Memory: one entry per breakpoint and one variable reference per expanded value per
stopped thread, freed on resume. Usability: VS Code's whole debugger UI, launched from `launch.json`,
for scripts, tests and a development server. Simplicity: one new command, no socket, and no UI code in
the extension.

**The adapter moves the process's standard streams, not the program's sink.** `nvs dap` copies the
real stdin and stdout for the protocol, then points the process's stdout and stderr at two pipes and
its stdin at the null device (`crates/nvs-cli/src/dap/stdio.rs`). A pump thread per pipe turns what
arrives into `output` events. A sink on the program's context would cover `echo` alone, and a
diagnostic, a panic message or a child process that inherited the stream would still reach the
protocol. Moving the stream covers all of them, and the program cannot read the client's frames.
Trade-offs: two pipes and two threads per session, and one audited `dup2` or `SetStdHandle` call per
stream. `exited` is sent once both pipes have drained, so no output arrives after it.

**A launch's `env` is set in the adapter's own process, before the program's thread starts.**
The program runs in this process, so its environment is the process's. `dap::set_env` makes one
audited `set_var` or `remove_var` call per variable. At that moment the only other threads read
pipes and frames, and the runtime reads the environment only once the program runs. Trade-offs:
one `unsafe` block per call in `nvs-cli`, and a session launches one program, so nothing has to be
put back.

**A `test` launch runs one test at a time and shows each test's output.** A stop then names the one
test that is running, and output arrives in the order the tests ran. Trade-off: a debugged suite
uses one core.

**A `pause` with nothing attached is kept until something attaches,** and `stopOnEntry` is the
same pending pause with its own reason. A client's pause during the compile is therefore not lost.
Memory: two `bool`s per session.

**Lanes.** Stage 1 is done. Stage 5 follows it on `crates/nvs-cli/src/dap/`. Stages 2 and 3 both edit
`crates/nvs-codegen/src/emit.rs` and `crates/nvs-cli/src/dap/mod.rs`, and run in that order after 5.
Stage 4 touches no Rust (`editors/vscode/`, `docs/reference/tools/40-editor.md`), so once stage 5 lands
it runs as its own lane beside 2 and 3; only its last item, the rule flips, waits for 2 and 3.

**A launched `serve` listens where `launch`'s `args` say.** `args` takes `--listen <address>` or
`--port <port>`, the flags `nvs serve` takes, and with neither the tree's `[server] listen` applies.
Anything else fails the `launch` request with a message (`dap::serve_address`). A client that wants
a free port writes `--port 0`, and the `listening on` line arrives as an `output` event. A serve
launch names one `program`: a mount table with no named file is not launched from the editor.
Simplicity: no new launch field, and one parser of the same two flags.

**`development` is forced on the resolved tree, before any snapshot exists.**
`nvs_config::resolve::Resolved::start_in_development` sets `[mode] default`, removes `[mode] ceiling`
and clears every `[[app]] mode`, and `config::set_of` runs it while the process-wide flag
`config::serve_in_development` is set. A reload goes through `set_of` too, so it keeps the mode. The
ceiling goes because a written `production` ceiling would otherwise refuse the start. Trade-off: one
`AtomicBool` for the process, set only by `nvs dap`.

**A step is the context's, not the site table's.** The site table is the unit's, shared by every
request and task, so arming its bytes for a step would stop every task that reaches them.
`Ctx::set_step` raises a byte on the stepping context, a site calls `nvs_debug_site` when its armed
byte or that byte is set, and the controller decides by `Step::stops_at` over the frame depth and by
`skipFiles`. Trade-offs: one more byte load per statement under the switch, and one walk of the
frame chain per site while a step is set. Nothing outside the switch.

**The returned value is kept on the context, rendered at the return.** The callee's frame record is
gone once it returns, and keeping a `Value` would need a retain and a release that every exit path
must run. `nvs_debug_return` runs only while the step byte is set, and when the step's own frame
returns it renders the out-slot on the task's stack, where variables are already read, and moves the
step to the caller. The next stop shows it first in the innermost scope as `f() returned`. Memory:
one rendered tree per stepping task, freed at its next stop.

**A returned value is redacted unless the written return type rules out `secret`.** The checker
records no type per expression, so `Function::debug_returns_secret` is read off the declaration
(`nvs_ir::lower::may_return_secret`). A function with no written return type, a generator, a thunk
and a property hook show their returned value redacted. Usability cost: a step out of an untyped
arrow function does not show its value.

**`skipFiles` is a `globset` set,** matched against the file's path with `/` between its parts, and
on Windows with no regard to case. `globset` is already in the graph through `ignore`.

**A condition is a checked probe evaluated over the frame's values, not a compiled function.**
`nvs_types::condition` parses it with the language's parser and checks it against the slot table's
representations, and `dap::condition` evaluates it through `nvs_runtime::record::property` and
`element`, which run no program code. Compiling it would need each slot's checker type, which the IR
erased, and the program's classes to type a property, so each condition would recompile the program.
A property's type is therefore checked when the probe runs. Arithmetic follows
`rule:types/arithmetic`'s rows in the evaluator. A failed evaluation stops the program and writes why
to the console. Security: a probe reads and never runs code. Memory: one checked tree per breakpoint
per function per unit. Simplicity cost: a small second evaluator of the operators a condition may use.

**A plain hit condition `N` means `>=N`,** as the JavaScript and PHP adapters read it. A hit is
counted only where the condition holds, and the count is the breakpoint's across sites and units.

**A condition may not read a `secret` variable; a logpoint's hole may, and shows it redacted.** A
condition that stops by a secret's value would tell the person debugging something about it.

## Stages

### 1. `nvs dap` speaks DAP over stdio and launches a program

**Does:** a DAP client starts `nvs dap`, initializes, launches a script in `run` mode, receives its
output as events, and disconnects. No gate.

- [x] `nvs dap` joins the command list beside `nvs lsp`, with its help text (`rule:ide/the-debug-adapter-does-not-wait-for-an-editor`). `crates/nvs-cli/src/main.rs:395`, `crates/nvs-cli/src/main.rs:582`
- [x] The serde message types and the `Content-Length` frame reader and writer; stdout carries frames only, as `nvs lsp` does (`rule:ide/stdout-belongs-to-the-protocol`). `crates/nvs-cli/src/dap/protocol.rs:1`, `crates/nvs-cli/src/lsp.rs:37`
- [x] `initialize` with the capabilities stage 1 has, `launch` with `mode: run` through the `nvs run` path, `configurationDone`, `disconnect` and `terminate`. `crates/nvs-cli/src/dap/mod.rs:1`, `crates/nvs-cli/src/run.rs:108`
- [x] The program's stdout and stderr become `output` events, and its exit an `exited` and a `terminated` event (`rule:ide/stdout-belongs-to-the-protocol`). `crates/nvs-cli/src/dap/mod.rs:1`
- [x] A Rust test with a scripted DAP client: a launched script's output arrives as events, and every byte on stdout belongs to a frame even when the program prints a frame-shaped line (`rule:ide/stdout-belongs-to-the-protocol`). `crates/nvs-lsp/tests/stdout_policy.rs:1`, `crates/nvs-cli/src/dap/mod.rs:1`

### 5. `nvs dap` launches tests and a development server, and stops at a breakpoint

**Does:** `launch` takes `test` and `serve`, and a client sets a breakpoint and reads threads, frames
and variables when it stops.

**Gate:** `debugger-core` reached, as `## Standing decisions` names it.

- [x] `launch` with `mode: test` and `mode: serve`; a launched serve starts in `development` (`rule:ide/the-debug-adapter-does-not-wait-for-an-editor`). `crates/nvs-cli/src/runner.rs:490`, `crates/nvs-cli/src/serve.rs:227`
- [x] Every launch compiles with the debug switch and runs under `debugger-core`'s controller (`rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`). `crates/nvs-host/src/debug.rs:1`
  - [x] `run` mode: `run_run` takes the session, lowers under `with_debug_slots`, loads the unit and attaches the run's context
  - [x] the controller tells a listener each load's breakpoints (`Debugger::on_load`), and the adapter sends `breakpoint` events from it
  - [x] `test` mode, every test's isolate attached
  - [x] `serve` mode, through `Compiler::debugging`
- [x] `setBreakpoints`, `threads`, `stackTrace`, `scopes`, `variables`, `continue` and `pause`, each task of the stopped tree a thread (`rule:testing/a-stop-pauses-the-whole-task-tree`). `crates/nvs-cli/src/dap/mod.rs:1`, `crates/nvs-host/src/scheduler.rs:312`
  - [x] a file is named by `dap::file_key` (its canonical path) on both sides
  - [x] `dap/stopped.rs` files each round's stops, one thread per task, and answers the reads
  - [x] the requests answered in `Session::answer`
- [x] Rust tests with the scripted client over a fixture in each mode: a breakpoint stops with the right values, a second request is answered while a served one is stopped, and the stdout check of stage 1 holds in `test` and `serve` (`rule:ide/the-debug-adapter-does-not-wait-for-an-editor`). `crates/nvs-cli/src/dap/mod.rs:1`
  - [x] `run`: a breakpoint stops with `$price` and `$total`, and `continue` ends the program
  - [x] `run`: `pause` stops a program in a loop
  - [x] `run`: `stopOnEntry` stops at the first statement, and `env` reaches the program
  - [x] `test`: a breakpoint in a test, and the stdout check
  - [x] `serve`: a stopped request, a second request answered meanwhile, and the stdout check

### 2. stepping over, into and out, and the value a step out returned

**Does:** `next`, `stepIn` and `stepOut` move one thread by one statement, and the caller's frame shows
the value the callee returned.

**Gate:** `debugger-core` reached.

**Split:** once the four step mechanisms land, one writing subagent writes the step tests in the last
item over fixtures. The session reviews them.

- [x] A step arms the thread's next stop site in the same frame, in any deeper frame, or in its caller, and only that thread's sites (`rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`). `crates/nvs-codegen/src/emit.rs:1230`, `crates/nvs-host/src/debug.rs:1`
  - [x] `Step` and the context's step byte (`DEBUG_STEP_OFFSET`), read at each site beside the armed byte
  - [x] `Debugger::step` names a stop by `Stop::id`, and its task sets the step when it wakes
  - [x] the handler stops at an unarmed site only where `Step::stops_at` says, as `StopReason::Step`
  - [x] `next`, `stepIn` and `stepOut` in `Session::answer`, and the `step` reason
- [x] A step over an `await` stops at the next statement of the same frame when the task resumes, and the task's siblings run while it waits (`rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`). `crates/nvs-ir/src/lower/expr.rs:3972`
- [x] A step into a call through a `callable` lands in the function or closure it names, and a step into a `Core` method steps over it (`rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`). `crates/nvs-runtime/src/callable.rs:184`
- [x] The frame record keeps the value its function returned, and after a step out the caller's first scope shows it (`rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`). `crates/nvs-runtime/src/ctx/frames.rs:1`, `crates/nvs-codegen/src/emit.rs:3806`
  - [x] the context keeps it instead of the record (`## Standing decisions`), through `nvs_debug_return`
  - [x] `Function::debug_returns_secret` from the written return type
  - [x] `Stop::returned`, shown first in the innermost scope
- [x] `skipFiles` keeps a step into from stopping in a file it matches (`rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`). `crates/nvs-cli/src/dap/mod.rs:1`
  - [x] `LaunchArguments::skip_files`, built into a `globset` set by `dap::skip_files`
  - [x] `Debugger::skip_files`, read by `Shared::step_stops`
- [x] Rust tests for each step kind, across an `await`, into a closure passed as a `callable`, and past a skipped file. `crates/nvs-cli/src/dap/mod.rs:1`

### 3. conditions, hit counts, logpoints and exception filters

**Does:** a breakpoint stops only when its condition and hit count say so, a logpoint writes and does not
stop, and the two exception filters stop at a throw.

**Gate:** `debugger-core` reached, and stage 2 `done`.

**Split:** once the condition compiler and the throw hook land, one writing subagent writes the tests in
the last item over fixtures. The session reviews them.

- [x] A condition and a logpoint's `{…}` holes are type-checked against the frame's slot table when the breakpoint is set, through a new entry beside `check_program`, and any call, assignment or `await` in them is an error. Each is a checked probe evaluated over the frame's values (`## Standing decisions`). A breakpoint whose condition fails to check is unverified, with the message (`rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`). `crates/nvs-types/src/condition.rs:1`
  - [x] `nvs_types::condition::check_condition` and `check_log_message` over `ProbeSlot`s, through the language's parser
  - [x] `BreakpointSpec` and `BreakpointTest`, bound per function when a unit arms the line, `Breakpoint::message` when it does not bind
  - [x] `DebugUnit::new` takes each site's owner function, and `DebugVariable::ty` its representation
  - [x] `Breakpoint::message` reaches the `setBreakpoints` body and the `breakpoint` event
- [x] The condition runs only where its site is armed, and hit conditions `N`, `>N`, `>=N`, `==N` and `%N` are counted per breakpoint (`rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`). `crates/nvs-cli/src/dap/condition.rs:1`
  - [x] `DebugUnit::passes` runs the bound `SiteTest` at an armed site only
  - [x] `HitCondition`, counted on the `Rule`'s counter across sites and units
  - [x] the evaluator over `record::property` and `record::element`, an evaluation error stops and says why
- [x] A logpoint sends an `output` event, with a `secret` value redacted, and does not stop (`rule:security/redaction-reaches-the-tools-own-renderings`). `crates/nvs-cli/src/dap/condition.rs:1`
  - [x] holes rendered by `Val::shown`, a `secret` slot or property as `Node::Redacted`
  - [x] the three capabilities at `initialize`
- [x] Under the switch, the classes the `try` blocks around each statement catch are recorded, and the throw path asks the controller whether to stop for `thrown` or `uncaught` (`rule:errors/escalation-ladder`, `## Standing decisions`). `crates/nvs-runtime/src/ctx/frames.rs:1`
  - [x] `Function::debug_catches` and `Program::stmt_catches`, filled from `TryFrame::catches` at each statement
  - [x] `nvs_debug_throw` at every landing block under the switch, and `Ctx::throw_reported`
  - [x] `StopCause::Throw`, `StopReason::Exception`, `Stop::exception` and `Debugger::exception_filters`
  - [x] `setExceptionBreakpoints`, `exceptionInfo` and the two filters at `initialize`
- [x] A throw caught inside a `skipFiles` file never stops (`rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`). `crates/nvs-host/src/debug.rs:1`
  - [x] `Shared::throw_stops` reads the catching frame's file through the `skipFiles` test
- [ ] Rust tests for each condition form, each hit form, a logpoint, both filters, and a condition that tries to call a method. `crates/nvs-cli/src/dap/mod.rs:1`

### 4. VS Code launches `nvs dap`, and the adapter has its feature proofs

**Does:** VS Code's own debugger UI reaches `nvs dap` through a descriptor factory and a `launch.json`
schema, and the feature is documented and proven.

**Gate:** stage 5 `done`. The last item also waits for stages 2 and 3.

**Split:** after the section in `40-editor.md` lands, one writing subagent writes `about.md`, the three
examples and the attack at the paths `bun nv proofs --id 'tools:editor/nvs-dap'` prints, under § *Text
an end user reads*.

- [ ] A `debuggers` contribution with the `launch.json` schema, its snippets and the `nvs` language for breakpoints (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`). `editors/vscode/package.json:29`
- [ ] A `DebugAdapterDescriptorFactory` that starts `nvs dap` with the binary `binary.ts` resolves, or the launch configuration's `nvsCommand`, registered in `activate` (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`). `editors/vscode/src/extension.ts:121`, `editors/vscode/src/binary.ts:49`
- [ ] `pathMappings` maps every path in and out of the adapter (`rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`). `crates/nvs-cli/src/dap/mod.rs:1`
- [ ] A host test sets a breakpoint through VS Code's API on the fixture and reads the right variable values, and the contributions test checks the schema. `editors/vscode/test/host/surfaces.test.ts:72`, `editors/vscode/test/contributions/contributions.test.ts:140`
- [ ] A `# nvs dap` section after `# nvs lsp-test` in the editor chapter (`rule:testing/feature-proofs`). `docs/reference/tools/40-editor.md:229`
- [ ] `about.md`, three examples and the attack, a program that prints a protocol frame; `bun nv proofs --id 'tools:editor/nvs-dap'` prints nothing owed (`rule:testing/feature-proofs`). `docs/examples/tools/editor/nvs-dap/about.md:1`, `tests/hostile/tools/editor/nvs-dap/01-a-program-prints-a-protocol-frame.nvs:1`
- [ ] The four rules this goal owns `shipped`, and `bun nv owners --closes debugger-adapter` passes (`rule:ide/the-debug-adapter-does-not-wait-for-an-editor`, `rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`, `rule:ide/the-extension-builds-no-ui-the-editor-already-has`, `rule:ide/every-feature-is-staged-behind-its-dependency`). `data/rules/ide/the-debug-adapter-does-not-wait-for-an-editor.json:3`, `data/rules/ide/the-debugger-ui-is-as-deep-as-the-adapter.json:3`, `data/rules/ide/the-extension-builds-no-ui-the-editor-already-has.json:3`, `data/rules/ide/every-feature-is-staged-behind-its-dependency.json:3`

### 6. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [ ] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

Stages 1, 5 and 2 are done. `launch` reads `mode`, `program`, `args`, `cwd`, `env`,
`stopOnEntry`, `filter` and `skipFiles`. Not read yet: `pathMappings`.

Stage 3's first five items are in; only its tests are owed. The exception filters replace the
push and pop per `try` (revised decision *"Uncaught" is decided at the throw*): the lowering
records each statement's caught classes (`crates/nvs-ir/src/lower/mod.rs:4271`,
`Program::stmt_catches` `crates/nvs-ir/src/ir.rs:59`), every landing block under the switch first
calls `nvs_debug_throw` (`crates/nvs-codegen/src/emit.rs:1641`,
`crates/nvs-runtime/src/ctx/frames.rs:517`), and `Shared::throw_stops`
(`crates/nvs-host/src/debug.rs:836`) applies the filters and `skipFiles`. The adapter offers
`uncaught` (default on) and `thrown`, and answers `setExceptionBreakpoints` and `exceptionInfo`.
`DebugUnit::new` takes the catches as a fifth argument. The exception path is built and
clippy-clean, and has never run.

Tests so far: the condition units in `crates/nvs-types/src/condition.rs` and
`crates/nvs-cli/src/dap/condition.rs`, and one protocol test in `crates/nvs-cli/tests/dap.rs`.
May go red: a test that counts the runtime symbol table (`nvs_debug_throw` is new) or a debug
unit's calls at a landing block (`crates/nvs-codegen/tests/probes.rs`). The sweep's reds are
stale, and nightly `40174915924` is the `side-ci-green` loop's.

### Next

**Stage 3's last item: the tests.** First run the exception path once by hand: a fixture under
`dap.rs`'s helpers that throws inside a `try` and outside one, with `setExceptionBreakpoints`
`["uncaught"]` and then `["thrown"]`. Check that it stops once per exception, at the throwing
line, with the variables shown. Then check a `catch` that does not match, which re-raises the
same object and must not stop a second time. Fix what that shows before writing more.

Then item 6 by its `**Split:**` line: one writing subagent writes the remaining tests in
`crates/nvs-cli/tests/dap.rs` beside
`a_condition_stops_where_it_holds_and_a_logpoint_writes_without_stopping`. They cover each
condition form, each hit form, a `secret` hole redacted, both filters, a throw a `skipFiles`
file catches, a `Core` method's fault under `uncaught`, and a `catch` expression. Review them,
and add a unit test of `DebugUnit::catches_at` and `throw_stops` beside
`crates/nvs-host/src/debug.rs`'s `mod tests`.

Stage 4 touches no Rust and may run as its own lane beside stage 3 (`## Standing decisions`
§ *Lanes*). For its attack: `echo` writes a carriage return as `␍`, so a program cannot echo a
real frame header, and the attack shows the line arriving as an `output` event.

### Backlog

- `evaluate` for watch and hover expressions is not asked for by any rule; a goal that wants it writes it into `docs/rules/ide/the-debugger-ui-is-as-deep-as-the-adapter.md` first.
- A condition that may call a pure `Core` function would need a purity mark on `Core` methods (`docs/rules/ide/the-debugger-ui-is-as-deep-as-the-adapter.md`).
- An `inout` by-reference variable has no debug slot, so a stopped frame does not list it. The adapter can read it through its staged cell. `crates/nvs-ir/src/lower/mod.rs:2602`
- No test checks the name an anonymous function's frame reports, and this goal decides how the call stack names it. `crates/nvs-cli/tests/debugger.rs:241`
- A pump waits for every holder of the program's pipes, so a child process the program left running delays `exited` until it ends. `crates/nvs-cli/src/dap/mod.rs:223`
