`Core\Html::later(callable(): void|Markup $fn, {placeholder?, error?, deadline?}): Markup` starts `fn`
at once as a task whose parent is the request, and returns a placeholder the page writes where the
output belongs:

```nvs
<?= Core\Html::later(fn() => Comments::render($post->id),
                     {placeholder: html`<p>Loading comments…</p>`}) ?>
```

The output is what `fn` echoes through the request's escaping sink, followed by the `Markup` it
returns. `placeholder` defaults to empty and `deadline` has no default. A `later` inside a `later`
callable is the request's child too, and its placeholder is part of its parent's output. Without
`later`, every response is buffered whole, so a page's first byte waits for its slowest query, and
`Core\Response::stream`, the only early byte, gives up `echo` and its escaping
(`rule:security/response-body-is-one-typed-member`).

**The route decides delivery, never the call**, so a component is written once:

| route | the page goes out | each output |
|---|---|---|
| normal | once every `later` task has ended | replaces its placeholder in one pass over the body |
| `#[Core\Route(…, slotted: true)]`, or `Core\Response::slotted()` before the main script ends | when the main script ends, placeholders in place | follows in the same response as a `<template for>` fill, in finishing order, with an inline polyfill sent once |

A slotted route sends nothing before the main script ends, and a slotted request that never calls
`later` is sent whole. `Core\Response::slotted()` after the main script ends, or inside a `later`
callable, throws `LogicError`. A slotted response is chunked, has no `Content-Length`, carries
`X-Accel-Buffering: no`, holds back a closing `</body></html>` until after the last fill, and keeps
its admission place until that fill. When its client goes,
`rule:http-server/a-request-outlives-a-client-that-goes-away` applies to the `later` tasks as to the
main script. Where the response's CSP limits scripts, the hashes of the polyfill and of the call after
each fill are added to it.

The placeholder is `<?start name="nvs-<token>-<n>">…<?end>`, where `<token>` is 96 random bits drawn
once per request and written as 16 base64url characters, and `<n>` counts the request's `later`
calls from 1. A visitor's string is escaped before the sink and an author cannot know the token, so
only `later` can name a slot, and `Markup` needs no second slot to carry one. A placeholder written
nowhere cancels its task with a `Warn`; one written twice throws `LogicError`.

**`later` has no limit of its own.** Its tasks are charged to the request tree
(`rule:security/isolate-budget-is-the-trees`), and a breach fails the request as it would without
`later`; on a slotted route, where the head is already out, every unfilled slot shows `error` and the
response ends. A task that throws or passes its own `deadline` shows `error` too, and the page keeps the
status the main script set. **A `later` callable cannot change the response head**: a status, a header,
a redirect, a body method, a cookie or a session regeneration inside one throws `LogicError`, and is a
compile error where the checker sees it. Reading the request and the session is allowed. Outside an
HTML response, or outside an HTTP request, `later` runs `fn` in place and returns its output, so a
component still works there.

A request that never calls `later` pays nothing. A normal route pays one pass over the body to
assemble the page; a slotted route sends its first byte when the main script ends and its last when
the slowest slot ends. A browser without JavaScript shows the placeholders.

- **Memory:** each `later` task's output is held until it is sent, and a slotted shell from the end
  of the main script until it is written: per request, O(in-flight), under the request's `memory`
  cap.

**Declined:**

- **Declarative shadow DOM as the markup.** It needs no script in any current browser, and the
  content would sit in a shadow root that the page's CSS does not reach and
  `document.getElementById` does not find.
- **Delivery chosen by the call site**, a `later` that streams and one that does not. A component
  would depend on the route that includes it.
- **A carrier slot on `Markup` for the placeholder.** The token in the bytes is enough to find it, and
  `+`, `Core\Html::join` and partials carry it with no change.
- **Flushing the shell before the main script ends.** The status and headers would be fixed before
  the script that sets them has finished.
- **A limit of `later`'s own**, such as a slot count or a per-slot memory share. A page with `later`
  is bounded exactly as the same page without it, and a second set of limits is something every
  developer must learn.
- **A slot in `<head>`.** The head is sent first, so a `<title>` cannot be filled later.
