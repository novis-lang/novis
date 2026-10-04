The terminal's substitution is uniform, so a `Core\IO\File` over descriptor 1 or 2 would not be a
convenience beside the sink — it would be the way around it, available to exactly the computed-escape
case `rule:tooling/terminal-output-is-a-sink` exists to catch. `Cli::write` already *is* writing to
those streams, and no operation is reachable two ways
(`rule:core-api/one-paradigm-per-operation`). So the standard-stream surface is `Core\IO::stdin()`
alone, the reading half being neither a sink nor a second spelling of anything. The price, recorded
rather than hidden: a program cannot emit byte-exact binary on its standard output, and one whose
output is bytes names a file.

The same reasoning keeps four more things out of `Core\Cli`. Cursor primitives, per
`rule:tooling/in-place-output-is-a-scoped-live-region`. Reading the clipboard, setting the window
title, or any other `OSC` capability — offering them would re-open, as a feature, the exact channel
the sink closes. Spinners and table rendering, which are pure text composition over `Str::format` and
`displayWidth` and need no terminal privilege, so they are a package's natural first offering rather
than Tier 0 (`rule:core-api/tier-placement`). And a TUI widget layer — panes, focus, event loops —
which is an application framework, not a language surface.
