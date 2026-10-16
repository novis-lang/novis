---
looper: 1
title: "`nvs dap` drives VS Code's debugger UI over a launched program"
summary: "nvs dap launches nvs run, nvs test or a development nvs serve over stdio, steps over, into and out, honours conditions, hit counts, logpoints, exception filters, skipFiles and pathMappings, shows the value a step out returned, VS Code reaches it through a descriptor factory and a launch.json schema alone, every designed rule this goal owns is shipped, and the checks below pass."
context: [crates/nvs-cli/src/main.rs, crates/nvs-cli/src/serve.rs, crates/nvs-host/src/scheduler.rs, editors/vscode/src/extension.ts]
stages:
  - id: 1
    title: "`nvs dap` launches a program over stdio and stops it at a breakpoint"
    status: open
    context: [crates/nvs-cli/src/main.rs, crates/nvs-cli/src/serve.rs, crates/nvs-cli/src/runner.rs, crates/nvs-cli/src/script.rs, crates/nvs-lsp/tests/stdout_policy.rs, docs/rules/ide/the-debug-adapter-does-not-wait-for-an-editor.md]
    extra:
      context:
        rules: [ide/the-debug-adapter-does-not-wait-for-an-editor, ide/stdout-belongs-to-the-protocol, testing/a-stop-pauses-the-whole-task-tree, testing/a-debugged-program-is-compiled-with-a-debug-switch]
  - id: 2
    title: stepping over, into and out, and the value a step out returned
    status: open
    context: [crates/nvs-codegen/src/emit.rs, crates/nvs-ir/src/lower/expr.rs, crates/nvs-runtime/src/callable.rs, crates/nvs-runtime/src/ctx/safepoint.rs, docs/rules/ide/the-debugger-ui-is-as-deep-as-the-adapter.md]
    extra:
      context:
        rules: [ide/the-debugger-ui-is-as-deep-as-the-adapter, testing/a-debugged-program-is-compiled-with-a-debug-switch, testing/a-stop-pauses-the-whole-task-tree]
  - id: 3
    title: conditions, hit counts, logpoints and exception filters
    status: open
    context: [crates/nvs-runtime/src/throwable.rs, crates/nvs-types/src/check.rs, crates/nvs-codegen/src/emit.rs, docs/rules/ide/the-debugger-ui-is-as-deep-as-the-adapter.md]
    extra:
      context:
        rules: [ide/the-debugger-ui-is-as-deep-as-the-adapter, errors/escalation-ladder, security/redaction-reaches-the-tools-own-renderings]
  - id: 4
    title: VS Code launches `nvs dap`, and the adapter has its feature proofs
    status: open
    context: [editors/vscode/package.json, editors/vscode/src/extension.ts, editors/vscode/src/binary.ts, editors/vscode/test/host/surfaces.test.ts, editors/vscode/test/contributions/contributions.test.ts, docs/reference/tools/40-editor.md, docs/rules/ide/the-extension-builds-no-ui-the-editor-already-has.md]
    extra:
      context:
        rules: [ide/the-debug-adapter-does-not-wait-for-an-editor, ide/the-extension-builds-no-ui-the-editor-already-has, ide/the-debugger-ui-is-as-deep-as-the-adapter, testing/feature-proofs]
checks:
  - id: owners-closes
    stage: 4
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, debugger-adapter]
    reach: true
    floor: false
extra:
  milestone: M10
---

# `nvs dap` drives VS Code's debugger UI over a launched program

## Why

`debugger-core` gives the host a Rust controller that stops a request's task tree at a statement and
reads its frames and variables. Nothing outside a Rust test can use it: there is no `nvs dap` command
(`crates/nvs-cli/src/main.rs:853` is `nvs lsp`, and no `dap` variant exists), and the VS Code extension
contributes no debugger (`editors/vscode/package.json:29`). A developer still debugs with
`Core\Debug::dump`. This goal puts the Debug Adapter Protocol on top of the controller, adds stepping,
breakpoint conditions and exception filters, and wires VS Code's own debugger UI to it. The two
`ide/` rules list every capability the adapter reports, and the editor adds no UI of its own.

## Standing decisions

**Launch only, a development `nvs serve` included.** The user decided this. `nvs dap` launches `nvs run`,
`nvs test` or `nvs serve` in development, over stdio. Under serve, a breakpoint stops whichever request
reaches it. There is no attach to a running process, so there is no debug socket.
`rule:ide/the-debug-adapter-does-not-wait-for-an-editor` holds it. Trade-offs: nothing on the request
path of a production server, which cannot be debugged at all. Usability: a server must be started from
the editor to be debugged. Simplicity: no listener, no authentication and no second protocol.

**The stop is the request's task tree, and each task is a DAP thread.** The user decided this, and
`debugger-core` builds it (`rule:testing/a-stop-pauses-the-whole-task-tree`). This goal presents it:
`threads` lists the stopped tree's tasks, and every other request keeps being served.

**The program runs in the adapter's own process.** The planning session's call. The adapter calls the
controller directly, and the program's stdout and stderr become `output` events. A child process would
need a second protocol between the adapter and the child. The cost: a bug in the launched program's
output path could reach the protocol's stdout, so a test reads every byte of stdout in each mode and
checks that each one belongs to a frame.

**The adapter is a module of `nvs-cli`, `crates/nvs-cli/src/dap/`.** The planning session's call. It
launches through the same code `nvs run`, `nvs test` and `nvs serve` use (`crates/nvs-cli/src/main.rs:3096`,
`crates/nvs-cli/src/runner.rs:322`, `crates/nvs-cli/src/serve.rs:161`). A crate of its own would need a
launcher interface back into `nvs-cli` for each mode.

**The DAP messages are serde types written here.** The planning session's call. DAP's framing is the
`Content-Length` header LSP uses, and the adapter answers about twenty requests. A protocol crate would
add a dependency to read a schema that small, and security is the first priority.

**A launched `nvs serve` starts in `development` whatever `[mode] default` says.** The planning
session's call, written into `rule:ide/the-debug-adapter-does-not-wait-for-an-editor`. The switch slows
every statement and a stop holds a request, so a debugged server is never a production one.

**A condition cannot call a function.** The user decided this, and it is written into
`rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`. A condition and a logpoint's holes read variables,
properties and array elements with operators and comparisons, and are type-checked against the frame's
slot table. A call is an error the editor shows on the breakpoint. A call at a breakpoint could write,
throw or `await`, and the program would then behave differently under the debugger. Usability cost:
`count($items) > 3` is not a valid condition.

**"Uncaught" is decided at the throw.** The planning session's call. Under the switch, each frame record
keeps the classes its enclosing `try` blocks catch, so the adapter knows at `nvs_raise` whether the
exception will be caught, while every frame is still there to show. That is one push and one pop per
`try`, under the switch only.

**Evaluating a watch expression is not built.** The rule does not ask for it, and the adapter does not
report `supportsEvaluateForHovers` or answer `evaluate` beyond a variable's own name. It is in the
Backlog.

**Trade-offs, all together.** Request path: nothing outside a debug session. Inside one, the switch's
cost (`debugger-core`), one handler push and pop per `try`, and a condition evaluated only where its
site is armed. Memory: one entry per breakpoint and one variable reference per expanded value per
stopped thread, freed on resume. Usability: VS Code's whole debugger UI, launched from `launch.json`,
for scripts, tests and a development server. Simplicity: one new command, no socket, and no UI code in
the extension.

## Stages

### 1. `nvs dap` launches a program over stdio and stops it at a breakpoint

**Does:** a DAP client launches `nvs run`, `nvs test` or a development `nvs serve` through `nvs dap`,
sets a breakpoint, and reads threads, frames and variables when it stops.

- [ ] `nvs dap` joins the command list beside `nvs lsp`, reading `Content-Length` frames on stdin and writing them on stdout only (`rule:ide/the-debug-adapter-does-not-wait-for-an-editor`, `rule:ide/stdout-belongs-to-the-protocol`). `crates/nvs-cli/src/main.rs:853`, `crates/nvs-cli/src/main.rs:2035`
- [ ] `initialize`, `launch` with `mode` `run`, `test` or `serve`, `configurationDone`, `disconnect` and `terminate`. A launched serve starts in `development`, and the program's stdout and stderr become `output` events. `crates/nvs-cli/src/main.rs:3096`, `crates/nvs-cli/src/runner.rs:322`, `crates/nvs-cli/src/serve.rs:161`
- [ ] `setBreakpoints`, `threads`, `stackTrace`, `scopes`, `variables`, `continue` and `pause` over `debugger-core`'s controller, each task of the stopped tree a thread (`rule:testing/a-stop-pauses-the-whole-task-tree`). `crates/nvs-host/src/scheduler.rs:306`
- [ ] Rust tests with a scripted DAP client over a fixture in each mode: a breakpoint stops with the right values, a second request is answered while a served one is stopped, and every byte on stdout belongs to a frame even when the program prints a frame-shaped line.

### 2. stepping over, into and out, and the value a step out returned

**Does:** `next`, `stepIn` and `stepOut` move one thread by one statement, and the caller's frame shows
the value the callee returned.

- [ ] A step arms the thread's next stop site in the same frame, in any deeper frame, or in its caller, and only that thread's sites (`rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`). `crates/nvs-codegen/src/emit.rs:1218-1238`
- [ ] A step over an `await` stops at the next statement of the same frame when the task resumes, and the task's siblings run while it waits. `crates/nvs-ir/src/lower/expr.rs:3920`
- [ ] A step into a call through a `callable` lands in the function or closure it names, and a step into a `Core` method steps over it. `crates/nvs-runtime/src/callable.rs:283`
- [ ] The frame record keeps the value its function returned, and after a step out the caller's first scope shows it. `skipFiles` keeps a step into from stopping in a file it matches.
- [ ] Tests for each step kind, across an `await`, into a closure passed as a `callable`, and past a skipped file.

### 3. conditions, hit counts, logpoints and exception filters

**Does:** a breakpoint stops only when its condition and hit count say so, a logpoint writes and does not
stop, and the two exception filters stop at a throw.

- [ ] A condition and a logpoint's `{…}` holes are type-checked against the frame's slot table when the breakpoint is set, through a new entry beside `check_program`, and any call, assignment or `await` in them is an error. Each is compiled by the same compiler as a function whose parameters are the frame's slots. A breakpoint whose condition fails to compile is unverified, with the message (`rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`). `crates/nvs-types/src/check.rs:84`
- [ ] The condition runs only where its site is armed. Hit conditions `N`, `>N`, `>=N`, `==N` and `%N` are counted per breakpoint. A logpoint sends an `output` event, with a `secret` value redacted (`rule:security/redaction-reaches-the-tools-own-renderings`), and does not stop.
- [ ] Under the switch, a `try` pushes the classes it catches onto the frame record and pops them when it ends, and the throw path asks the controller whether to stop for `thrown` or `uncaught` (`rule:errors/escalation-ladder`). `crates/nvs-runtime/src/throwable.rs:1013`, `crates/nvs-runtime/src/throwable.rs:1064`, `crates/nvs-runtime/src/throwable.rs:1140`
- [ ] A throw caught inside a `skipFiles` file never stops. Tests for each condition form, each hit form, a logpoint, both filters, and a condition that tries to call a method.

### 4. VS Code launches `nvs dap`, and the adapter has its feature proofs

**Does:** VS Code's own debugger UI reaches `nvs dap` through a descriptor factory and a `launch.json`
schema, and the feature is documented and proven.

- [ ] A `debuggers` contribution with the `launch.json` schema, its snippets and the `nvs` language for breakpoints. `editors/vscode/package.json:29`
- [ ] A `DebugAdapterDescriptorFactory` that starts `nvs dap` with the binary `binary.ts` resolves, or the launch configuration's `nvsCommand`, registered in `activate` (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`). `editors/vscode/src/extension.ts:121`, `editors/vscode/src/binary.ts`
- [ ] `pathMappings` maps every path in and out of the adapter. A host test sets a breakpoint through VS Code's API on the fixture and reads the right variable values, and the contributions test checks the schema. `editors/vscode/test/host/surfaces.test.ts`, `editors/vscode/test/contributions/contributions.test.ts`
- [ ] A `# nvs dap` section in `docs/reference/tools/40-editor.md`, the help text, and what `bun nv proofs --id 'tools:editor/nvs-dap'` still owes, its attack the program that prints a protocol frame. `docs/reference/tools/40-editor.md:8`
- [ ] The three rules `shipped`, and `bun nv owners --closes debugger-adapter` passes.

## Handoff

### State

The goal is new. No stage is started. The anchors above were checked against the tree when it was
written. `crates/nvs-cli/src/main.rs` was being edited by another session at the time, so find its
anchors by name (`Lsp {`, `Command::Lsp`, `fn run_run`) if a line has moved. Every stage needs
`debugger-core`'s controller, slot table and frame records, which do not exist yet.

### Next

Stage 1, `nvs dap` over stdio, on `crates/nvs-cli/src/main.rs` and a new `crates/nvs-cli/src/dap/`:
- the command and the frame reader and writer (`rule:ide/the-debug-adapter-does-not-wait-for-an-editor`). `crates/nvs-cli/src/main.rs:853`
- `launch` in each mode through the existing entry points. `crates/nvs-cli/src/main.rs:3096`, `crates/nvs-cli/src/serve.rs:161`
- breakpoints, threads, frames and variables over the controller. `crates/nvs-host/src/scheduler.rs:306`

### Backlog

- `evaluate` for watch and hover expressions is not asked for by any rule; a goal that wants it writes it into `docs/rules/ide/the-debugger-ui-is-as-deep-as-the-adapter.md` first.
- A condition that may call a pure `Core` function would need a purity mark on `Core` methods (`docs/rules/ide/the-debugger-ui-is-as-deep-as-the-adapter.md`).
- PhpStorm's debugger stays deferred with the PhpStorm client (`docs/rules/ide/phpstorm-bridges-to-the-same-server.md`).
