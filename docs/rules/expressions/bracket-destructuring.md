`list(...)` as a destructuring target does not parse. It is `E0230`, naming `[...]`, whose element
grammar is identical in every position — key, nesting depth, skipped slot, reference marker.

```
list(int $a, string $b) = $pair;    // E0230
[int $a, string $b]     = $pair;    // the replacement, identical in every element position
```

The parser still reads the whole construct through to its `;` — the target's elements, the `=` and
the value expression — purely so the diagnostic spans the real statement and recovery resumes cleanly
at the next one. The parsed target is then discarded and the statement becomes an error node: a
rejected construct never reaches the tree as a live node.

`list` stays a reserved word. Freeing it would let a class or a method be named `list`, and keeping it
reserved is what lets the diagnostic fire.

A PHP spelling is reused only when its meaning has no other home in Novis, the test
`rule:statements/require-is-the-only-inclusion-construct` applies. `list(...)` fails it: `[...]`
already spells destructuring, and both built the same node from the same element grammar, so no
later stage ever told them apart. With one spelling, the bracket form is the grammar's only entry
into a destructuring statement. The conversion is a bracket for a parenthesis, and every
destructuring leaf already needs a type, so no PHP `list()` site parsed unedited in any case.

**Declined:**
- **`list(...)` kept as a second spelling.** Every argument for it ("PHP programmers know it")
  applies as well to every other refused PHP spelling (`rule:statements/nothing-gets-a-second-name`).
- **`list(...)` kept only for the shape with no keys and no nesting.** A spelling that works for
  `list($a, $b)` but not `list('id' => $id)` is a third rule to teach.
- **Freeing `list` as an identifier at the same time.** That is a reserved-word question with its own
  compatibility surface, unrelated to which destructuring spelling survives.
