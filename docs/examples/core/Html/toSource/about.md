Gives you the HTML inside a `Core\Html\Markup` value as a normal `string`.

A `Core\Html\Markup` value is HTML that is safe to write into a page. You cannot cast it to a
`string` with `as string`. `Core\Html::toSource` is the only way to get the text. The text is
returned exactly as it is, so `&amp;` stays `&amp;`. The `Markup` value does not change.

The second argument is the reason: a short text that tells the next reader why the program needs the
text. You write it as text in quotes, or as a `const`. A reason in a variable does not compile. The
reason is not used for anything else. An empty reason throws a `LogicError`.

You need the text when HTML goes somewhere that is not the page: a cache, a database column, a file,
or a JSON response.

**Good to know:** this does not undo `Core\Html::escape`. The examples show the text, the empty reason,
and HTML inside a JSON response.
