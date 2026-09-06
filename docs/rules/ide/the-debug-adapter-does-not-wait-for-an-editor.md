`nvs dap` — the debug adapter, using safepoints for breakpoints (`rule:testing/debug-probes`) — is a
complete, testable deliverable on its own. A working adapter and a working debugger UI in a given editor
are two different integrations, the same way a language server and a syntax-highlighting extension are,
so a stalled or under-scoped editor integration cannot block the adapter from shipping and being useful
to a CLI-driven client or a third editor.

For VS Code the editor-side wiring is small and lands with the adapter: a `DebugAdapterDescriptorFactory`
and a `launch.json` configuration schema targeting `nvs dap`. DAP is a wire protocol, and VS Code
already renders breakpoints, call stack, variables and watches for any adapter that speaks it, so the
extension authors no debugger UI — the acceptance is a breakpoint set in VS Code's UI hitting in
JIT-compiled code with correct variable values, through the descriptor factory and schema alone
(`rule:ide/the-extension-builds-no-ui-the-editor-already-has`). How deep that UI goes is the adapter's
capability set, not the extension's.

PhpStorm's `XDebugger` UI wired to a DAP backend stays deferred (`rule:ide/phpstorm-bridges-to-the-same-server`).
The two clients are deliberately asymmetric here; a future PhpStorm-depth pass has to address or
explicitly accept that.
