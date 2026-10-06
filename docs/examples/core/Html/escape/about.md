Makes a text safe to show on an HTML page.

`Core\Html::escape` writes the five characters `&`, `<`, `>`, `"` and `'` as character references,
so a browser shows them as text and never reads them as tags. It always escapes all five, so the
result is safe inside an element and inside an attribute value. A direction control that is never
closed is replaced with `�`, so it cannot reverse the text after it.

The result is a `Core\Html\Markup` value, not a `string`. A page writes a `Markup` value exactly as
it is, so the text is never escaped twice. Text from a visitor is `tainted` (it came from outside the
program), and `escape` is how such a text goes into a page.

**Good to know:** a text that already contains `&amp;` is escaped again, and becomes `&amp;amp;`.
