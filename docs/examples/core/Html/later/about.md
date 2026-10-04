Builds a slow part of a page in a separate task, while the rest of the page is written.

`Core\Html::later` takes a closure and returns a placeholder at once. You write the placeholder
where the part belongs. When the page is finished, the output of the closure replaces the
placeholder. Several `later` calls run at the same time, so a page with three slow parts waits
for the slowest one, not for all three in a row.

If the closure throws an error, the page shows the `error` markup in its place, and the status
stays the same. `deadline` sets the longest time the closure may run. On a route with
`slotted: true`, the browser gets the page first and each part as soon as it is ready.

**Good to know:** a closure in `later` cannot set a header, a cookie or the status. The page has
already decided those.

**The examples below** show two parts that run at the same time, a part that fails, and a part
that takes too long.
