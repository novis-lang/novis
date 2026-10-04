The same attribute — named or bare, with the same field values or different ones — may be attached to
one declaration any number of times.

```php
#[Cache(ttlSeconds: 60)]
#[Cache(ttlSeconds: 300, tag: "long")]
public function show(): Response { … }
```

Nothing about attaching one attribute is aware of how many others share its site. There is no arity
check and no deduplication, so attaching the identical payload object twice keeps both copies and `all<T>`
answers with both, in attach order: `count(all<T>(...))` counts attachments, never distinct payloads.

Ambiguity is entirely retrieval's problem (`rule:attributes/retrieval-folds-while-checking`). A rule
that needs exactly one attachment says so itself and refuses the second where it is written, the way
`#[Access]` does (`rule:attributes/access-payload`).
