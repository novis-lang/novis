A declaration's attached-attribute list is fixed by its source, so `Core\Attributes::get<T>` and
`::all<T>` are answered by `nvs check` and the call is **replaced** with the answer:

- nothing satisfies `T` — a compiled-in `null`, or the empty array under `all<T>`;
- exactly one does — that payload itself, as a constant, with no lookup at run time;
- more than one does under `get<T>` — `E0728`, naming `::all<T>` as the fix.

There is no runtime lookup and no reflection table in the compiled unit. The two registered
`Core\Attributes` members name a body that aborts, so a program that ran at all never called either.

Because the call becomes the payload, every value in a *matched* payload needs a constant form, folded
through the scope the payload was written in — a class constant whose own declaration folds to no value
is `E0731`. A `T` that is not a shape is `E0729`, and a `$target` that names no declaration is `E0730`.
