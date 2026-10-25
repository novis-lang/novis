`nvs dap` — the debug adapter, stopping at the stop sites its debug compile switch puts at every
statement (`rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`) — is a complete, testable
deliverable on its own. A working adapter and a working debugger UI in a given editor
are two different integrations, the same way a language server and a syntax-highlighting extension are,
so a stalled or under-scoped editor integration cannot block the adapter from shipping and being useful
to a CLI-driven client or a third editor.

**It launches, and never attaches.** `nvs dap` speaks DAP over its own stdin and stdout and answers
`launch` alone: it compiles the program with the switch and runs it in its own process, as `nvs run`,
`nvs test` or `nvs serve` would. A launched `nvs serve` starts in `development` whatever `[mode] default`
says, and a breakpoint stops whichever request reaches it while every other request is served
(`rule:testing/a-stop-pauses-the-whole-task-tree`). There is no `attach` request, so no running process
ever listens for a debugger and there is no debug socket to bind, guard or forget open. A production
server cannot be debugged, which is the point: the switch slows every statement and a stop holds a
request.

stdout carries DAP frames and nothing else, on the terms
`rule:ide/stdout-belongs-to-the-protocol` sets for the language server. What the launched program
writes to its stdout and stderr reaches the client as `output` events, so a program that prints a
frame-shaped line shows text and cannot speak the protocol.

The `launch` arguments are the whole interface: `mode` (`run`, `test` or `serve`), `program`, `args`,
`cwd`, `env`, `stopOnEntry`, the test `filter` under `test`, `skipFiles` and `pathMappings`
(`rule:ide/the-debugger-ui-is-as-deep-as-the-adapter`). `nvsCommand`, read by the editor and never by
the adapter, names the command that starts `nvs dap`, so the adapter can run inside a container through
`docker exec -i` and still be reached over stdio.

For VS Code the editor-side wiring is small and lands with the adapter: a `DebugAdapterDescriptorFactory`
and a `launch.json` configuration schema targeting `nvs dap`. DAP is a wire protocol, and VS Code
already renders breakpoints, call stack, variables and watches for any adapter that speaks it, so the
extension authors no debugger UI — the acceptance is a breakpoint set in VS Code's UI hitting in
JIT-compiled code with correct variable values, through the descriptor factory and schema alone
(`rule:ide/the-extension-builds-no-ui-the-editor-already-has`). How deep that UI goes is the adapter's
capability set, not the extension's.


- **Memory:** the adapter's own state is one entry per breakpoint and one per stopped thread's
  variable references, freed on each resume. What the debugged program holds is the switch's
  (`rule:testing/a-debugged-program-is-compiled-with-a-debug-switch`).

**Missing:** `nvs dap` and the VS Code wiring. The stop it drives is built: `nvs_host::debug`'s
controller parks a task at an armed stop site or a `DEBUG_BREAK` poll, and resumes it.

**Declined:**
- **The adapter ships together with an editor's debugger UI.** A stalled editor integration would hold
  back an adapter that a CLI-driven client can already use.
- **`attach` to a running server.** It needs a listener in every process that might be debugged, with
  its own authentication, and a production process with that listener is one misconfiguration from
  handing out its memory. Launching gives the same view of a development server with no listener.
- **Running the program as a child process of the adapter.** The adapter would need a second protocol
  to the child to reach its stops, and in one process the controller is a Rust call.
