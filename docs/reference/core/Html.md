---
summary: HTML as a page writes it — the ``html`…` `` template, `Core\Html\Markup`, escaping, sanitizing and parsing
keywords: html, markup, template, html template, page, view, render, escape, escaping, xss, htmlspecialchars, htmlentities, html_entity_decode, strip_tags, DOMDocument, loadHTML, sanitize, parse
---

`Core\Html\Markup` is the one type an HTTP request writes raw; every `string` written into a
page is escaped. A `Markup` is made in three ways: an ``html`…` `` template, whose text is trusted
because it was written in the source and whose `{$…}` holes are escaped; `Core\Html::escape`,
which escapes a string; and `Core\Html::sanitize`, which rebuilds an untrusted document with
only its safe parts. `+` joins two `Markup` values and `Core\Html::join` joins a list; `.` on a
`Markup` is a compile error. `Core\Html::toSource` is the only way back to a `string`, and it
takes a written reason. `Core\Html::parse` reads any document onto a `Core\Xml\Node` tree and
never fails.

```nvs
<?nvs
tainted string $name = "<script>alert(1)</script>";
string $bio = "<p>likes <b>tea</b></p><img src=x onerror=alert(1)>";
Core\Html\Markup $badge = html`<span class="badge">new</span>`;
Core\Html\Markup $line = html`<p>{$name} {$badge}</p>`;
Core\Html\Markup $safe = Core\Html::sanitize($bio);
echo $line + $safe, "\n";
echo Core\Html::join([$badge, $badge], html`, `), "\n";
echo Core\Html::toSource($badge, "logging the fragment"), "\n";
```
```output
<p>&lt;script&gt;alert(1)&lt;/script&gt; <span class="badge">new</span></p><p>likes <b>tea</b></p><img src="x">
<span class="badge">new</span>, <span class="badge">new</span>
<span class="badge">new</span>
```
