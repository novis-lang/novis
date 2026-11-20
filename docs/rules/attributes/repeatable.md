An attribute declared `repeatable` may be used on one declaration any number of times. Any other
attribute is used at most once per declaration, and a second use is `E0763` at the second, naming both
(`rule:attributes/an-attribute-is-declared-with-its-sites`).

```nvs
repeatable attribute Cache on method { ttlSeconds: int, tag?: string }

#[Cache(ttlSeconds: 60)]
#[Cache(ttlSeconds: 300, tag: "long")]
public function show(): Response { … }
```

A `repeatable` attribute has no deduplication, so the identical payload used twice keeps both copies,
and `all<T>` returns both, in attach order: `count(all<T>(...))` counts uses, never distinct payloads.
A retrieval of a `repeatable` attribute with `get<T>` is ambiguous where two uses exist, and that is
retrieval's error (`rule:attributes/retrieval-folds-while-checking`).

`Core`'s attributes follow the same rule: `#[Access]` and `#[Api]` are not `repeatable`, and `#[Route]`
and `#[TestWith]` are.

**Declined:**
- **Every attribute repeatable, with each rule that needs one counting on its own.** A second use that
  nothing reads compiled, and each attribute that needed exactly one wrote its own check and its own
  error code.
