A ternary is on one line, or its `?` and its `:` each start a line one level in from the line its
condition starts on. A line break its author wrote anywhere at the ternary's own level breaks it:
before or after the `?`, or before or after the `:`. A short ternary breaks the same way, with `?:`
starting the line.

```nvs
echo $room * 2 > $limit ? "more than half of the limit is free\n"
    : "less than half of the limit is free\n";

echo $room * 2 > $limit
    ? "more than half of the limit is free\n"
    : "less than half of the limit is free\n";
```

The first statement formats to the second. A break inside the condition or inside a branch, such as
inside a call's arguments, belongs to that part. A condition that is a broken operator chain keeps the
layout `rule:tooling/fmt-every-binary-operator-breaks-operator-first` gives it, and the `?` and `:`
lines follow it at the same level.

**Declined:**
- **`?` at the end of the condition's line, as some styles write it.** The operator-first layout of
  every other chain puts what joins two lines at the start of the second.
