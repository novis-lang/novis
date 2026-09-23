Cleans a piece of HTML that somebody else wrote, so you can show it on your page safely.

`Core\Html::sanitize` reads the HTML the way a web browser does. Then it builds a new document from a
fixed list of safe elements, like `p`, `b`, `em`, `a`, `img`, lists and tables. A tag that is not on
the list is removed, and its text is kept. A `script` or `style` element is removed together with its
content. Every attribute that could run code or change your page is removed, for example `onclick`,
`style`, `class` and `id`. A link to `javascript:` loses its `href`.

The result is a `Core\Html\Markup` value, so a page writes it exactly as it is. The list cannot be
changed.

**Good to know:** cleaning the result a second time gives the same result. If you want to show the
HTML as plain text, use `Core\Html::escape` instead.
