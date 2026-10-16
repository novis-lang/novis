A value written in the file and a value handed to `Core\Config::set` go through **one parser**. Every
shipped default parses identically through the boot path and through `set`, and the two refusals the
boot makes — a value that is not a quantity, a name no directive governs — are the same two `set`
answers `false` with. A depth written as a bare integer and one written as `"64"` are the same value
wherever they arrive.

This is the shape `rule:expressions/preparation-preserves-behaviour` already establishes: a prepared
path and a runtime path cannot diverge when there is one implementation. `Core\Config` itself
marshals a string in and a string out and holds none of the rules — what a name resolves to, what a
set may do and where the ceiling comes from all live in the configuration crate, and the parser is not
reachable from the stdlib member at all, which is what makes the claim true by construction rather
than by discipline.
