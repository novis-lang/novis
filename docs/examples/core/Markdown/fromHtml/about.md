Converts HTML to Markdown. Text in the HTML stays text, so it never becomes Markdown syntax.

You can pass HTML as a `string`, as a `Core\Html\Markup` or as a `Core\Xml\Node` from
`Core\Html::parse`. A character such as `*`, `[` or `#` in the text gets a backslash, so it shows as
the same character. Elements that are not safe, such as `<script>`, are deleted with their text. A
link to a URL that is not safe, such as `javascript:`, keeps only its text. The result is a
`tainted string`, because it is made from text you did not write.

**Good to know:** a table that Markdown cannot show, for example one with `colspan`, is written as
safe HTML on one line. A URL is written percent-encoded, so `/Straße` is `/Stra%C3%9Fe`.
`fromHtml` does not look for the main content of a page. Pass it the element you want.

**The examples below** show a short piece of HTML, one article from a whole page, and a server that
returns a page as Markdown when the request has the header `Accept: text/markdown`.
