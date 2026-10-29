- **A comment added inside a `--FILE--` block moves every line its `--EXPECTF-ERROR--` pins, and
  source line 1 is file line 4.** Three things move: the `--> case.nvs:L:C` line, a pinned
  gutter number (`26 |`), and the arrow's indent, one space wider once the line reaches 10. Keep
  a comment rewrite the same length when the case pins a diagnostic, or update the expectation in
  the same edit, and say so in a writing subagent's brief. See `tests/conformance/core/a-route-capture-is-the-value-the-match-converted-not-the-segment-text.nvst`. `until: gone tests/conformance/core/a-route-capture-is-the-value-the-match-converted-not-the-segment-text.nvst:--EXPECTF-ERROR--`
