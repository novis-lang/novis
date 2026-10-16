`output: 'capture'` is the default: the child's output lands on the result and is charged to the
tree's `max_output`. `output: 'inherit'` appends it to the parent's stream **when the result is
awaited**, which keeps ordering deterministic under concurrency. Capture is the default because the
alternative silently mixes another script's bytes into a response the parent is responsible for.

A child's `echo` writes to the **parent's sink**, so the captured output carries that sink's carrier
type rather than a plain `string` (`rule:security/capture-answers-the-carrier`). That is what lets a
parent re-emit a captured result without escaping it twice, and it is why `'inherit'` needs no
separate rule: the two carriers already match. `Core\Cli`'s *members* still throw inside an isolate —
that rule is about owning the terminal, which an isolate's buffered output never touches.
