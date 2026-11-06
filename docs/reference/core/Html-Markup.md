---
summary: HTML that is already safe to print. Html templates and `Core\Html` methods return it, and `Core\Response::html` takes it
keywords: markup, safe HTML, trusted HTML, raw HTML, escaping, double escaping, html template, XSS, print HTML, as Markup, toSource, fragment, partial
---

`Core\Html\Markup` is HTML that is already safe to print. An html template, ``html`…` ``, returns
one. `Core\Html::escape`, `Core\Html::sanitize`, `Core\Html::join` and `Core\Html::later` return one
too. In a web request, `Core\Out::capture` returns what its function printed as a `Markup`.
`Core\Response::html` takes a `Markup`, and a `string` there does not compile.

In a web request, `echo` escapes every `string` and writes a `Markup` unchanged. A template does the
same with each `{$…}` value. A `string` is escaped, and a `Markup` is written unchanged. So you can
put a fragment from one template into another template, and nothing is escaped twice.

A string becomes a `Markup` in three ways. `Core\Html::escape` escapes it as text.
`Core\Html::sanitize` keeps only the safe elements and attributes of an HTML string. A string written
directly in the code converts with `as Core\Html\Markup`. A computed string does not compile there.
A `tainted` string (text that came from outside the program) does not compile there either.

`+` joins two `Markup` values. `.` and `as string` on a `Markup` do not compile.
`Core\Html::toSource` returns the text of a `Markup` as a `string`. It needs a reason, written as a
string in the code.

```nvs
<?nvs
tainted string $name = 'Tom & <Jerry>';

// The template escapes the string in `{$name}`.
Core\Html\Markup $bold = html`<b>{$name}</b>`;

// `$bold` is a `Markup`, so this template writes it unchanged.
Core\Html\Markup $line = html`<p>Hello {$bold}</p>`;
echo $line, "\n";

// A string written in the code converts with `as`.
Core\Html\Markup $star = '<i class="star"></i>' as Core\Html\Markup;
echo $star + Core\Html::escape($name), "\n";

string $source = Core\Html::toSource($star, 'the icon is saved in a cache');
echo Core\Str::length($source), "\n";
```
```output
<p>Hello <b>Tom &amp; &lt;Jerry&gt;</b></p>
<i class="star"></i>Tom &amp; &lt;Jerry&gt;
20
```
