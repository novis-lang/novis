Output is a pure function of the input bytes of every file in the unit, the mode, the target dialect,
the rule-table digest and the explicit flags — nothing else. Concretely, and each is a rule the
implementation may not break:

1. **File discovery is sorted** by byte-wise path, and units are processed in that order.
2. **No ambient input** — no clock, locale, environment, network, random seed, absolute path in
   output, or hash-map iteration order anywhere a decision or an emission order depends on it.
3. **The pass pipeline is fixed and each pass runs once**, over the tree in source order. There is no
   run-to-fixpoint; a rule needing rewritten input names the earlier pass that produces it.
4. **Rule precedence is total**: innermost matching node first, then by rule id. Two rules that could
   both apply at one site are a table error CI catches.
5. **Every generated name is a pure function of source facts.** Where a counter is unavoidable it is
   per-file, in source order, and the rule says so.
6. **Formatting is not the converter's business.** It emits a tree and prints it through `nvs fmt`'s
   one unconfigurable style; it has no formatting options, and output is UTF-8 without a BOM with `\n`
   line endings on every platform.
7. **Every output file carries a header** naming the source path, source digest, rule-table digest,
   mode and dialect, so two runs that differ are attributable to one of those five inputs.

**No model, no heuristic outside the table, no probability.** A rewrite the table does not state
does not happen. This is what makes the output reviewable and the tool re-runnable, and it is why
LLM assistance is refused outright (`rule:tooling/convert-never-does`).
