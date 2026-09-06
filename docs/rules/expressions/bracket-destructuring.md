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
