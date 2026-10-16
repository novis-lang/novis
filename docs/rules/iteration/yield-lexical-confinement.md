`yield` is confined to the generator's own body. A helper function called from a generator cannot
yield into it, and neither can an anonymous function written inside one — an anonymous function has its own body, so a
`yield` there does not make the enclosing method a generator; it is refused.

To yield another iterable's values, a generator writes the loop out:

```
foreach ($inner as $v) { yield $v; }
```

This is the price of the state-machine lowering (`rule:iteration/generators`): the transform can
split the body it compiles and nothing else, so a suspension point in a frame it did not generate has
nowhere to be recorded.
