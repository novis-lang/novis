A declaration's attached-attribute list is fixed by its source, so `Core\Attributes::get<T>` and
`::all<T>` are answered by `nvs check` and the call is **replaced** with the answer:

- no payload was written under `T` — a compiled-in `null`, or the empty array under `all<T>`;
- exactly one was — that payload itself, as a constant, with no lookup at run time;
- more than one was under `get<T>` — `E0728`, naming `::all<T>` as the fix. Only a `repeatable`
  declaration can have more than one (`rule:attributes/repeatable`).

So an ambiguous retrieval of a repeated attribute fails at the call site that would be wrong, while
compiling, and never on the first request that reaches a site with two matches.

There is no runtime lookup and no reflection table in the compiled unit. The two registered
`Core\Attributes` members name a body that aborts, so a program that ran at all never called either.

Because the call becomes the payload, every value in a *matched* payload needs a constant form, folded
through the scope the payload was written in — a class constant whose own declaration folds to no value
is `E0731`. A `T` that is not an attribute declaration is `E1008`
(`rule:attributes/a-retrieval-matches-the-declaration-it-names`), and a `$target` that names no
declaration is `E0730`.
