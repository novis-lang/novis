`Core\Html::later(callable(): void|Markup $fn, {placeholder?, error?, deadline?}): Markup` starts `fn`
at once as a task whose parent is the request, and returns a placeholder the page writes where the
output belongs:

```php
<?= Core\Html::later(fn() => Comments::render($post->id),
                     {placeholder: html`<p>Loading comments…</p>`}) ?>
```

The output is what `fn` echoes through the request's escaping sink, followed by the `Markup` it
returns. **The route decides delivery, never the call**, so a component is written once:

| route | the page goes out | each output |
|---|---|---|
| normal | once every `later` task has ended | replaces its placeholder in one pass over the body |
| `#[Core\Route(…, slotted: true)]`, or `Core\Response::slotted()` before the main script ends | when the main script ends, placeholders in place | follows in the same response as a `<template for>` fill, in finishing order, with an inline polyfill sent once |

The placeholder is `<?start name="nvs-<token>-<n>">…<?end>`, where `<token>` is 96 random bits drawn
once per request. A visitor's string is escaped before the sink and an author cannot know the token, so
only `later` can name a slot, and `Markup` needs no second slot to carry one. A placeholder written
nowhere cancels its task with a `Warn`; one written twice throws `LogicError`.

**`later` has no limit of its own.** Its tasks are charged to the request tree
(`rule:security/isolate-budget-is-the-trees`), and a breach fails the request as it would without
`later`; on a slotted route, where the head is already out, every unfilled slot shows `error` and the
response ends. A task that throws or passes its own `deadline` shows `error` too, and the page keeps the
status the main script set. **A `later` closure cannot change the response head**: a status, a header,
a redirect, a body method, a cookie or a session regeneration inside one throws `LogicError`, and is a
compile error where the checker sees it. Outside an HTML response `later` runs `fn` in place and
returns its output.
