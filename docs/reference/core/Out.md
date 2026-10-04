---
summary: output buffering scoped to a callable — what it echoes is captured as the sink's carrier instead of reaching the output
keywords: ob_start, ob_get_clean, ob_get_contents, ob_end_clean, output buffering, capture, echo
---

`Core\Out::capture` runs a callable and returns everything it `echo`ed as the carrier of the sink in
force — a `Core\Cli\Text`, or a `Core\Html\Markup` under an HTTP request, typed
`Core\Html\Markup|Core\Cli\Text` — never a plain `string`, so re-emitting it is one more `echo` and it
is not escaped a second time. `as string` reads the text out. Nothing the callable wrote
reaches the output below; a capture nests by call nesting, and there is no global buffer stack, no
`ob_get_contents` and no implicit flush. `{through: $fn}` transforms the carrier before it is
returned, and the callable's own return value is discarded. `Core\Debug::dump` is never captured —
it writes to stderr.

```nvs
<?nvs
var $captured = Core\Out::capture(fn (): void => {
    echo "a";
    echo "b";
});
echo "captured=[", $captured, "]\n";

echo Core\Out::capture(fn (): void => {
    echo "outer<";
    echo Core\Out::capture(fn (): void => { echo "inner"; });
    echo ">";
}), "\n";

echo "[", Core\Out::capture(fn (): void => {}), "]\n";

try {
    echo Core\Out::capture(fn (): void => {
        echo "lost";
        throw new RuntimeError("stop");
    });
} catch (RuntimeError $e) {
    echo "threw: ", $e->message, "\n";
}
```
```output
captured=[ab]
outer<inner>
[]
threw: stop
```
