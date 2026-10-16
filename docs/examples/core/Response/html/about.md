Sends an HTML page as the response. It also adds the header `Content-Type: text/html;
charset=utf-8`, so the browser shows the body as a page.

The body must be a `Core\Html\Markup`, not a `string`. A `Markup` is HTML that is already safe to
send. You get one from an html template such as ``html`<p>Hello</p>` ``. You also get one from
`Core\Html::escape`, from `Core\Html::join`, or with `as Core\Html\Markup` on a string you wrote in
the program. An html template escapes every value you put in `{...}`. So text from a visitor is
shown as text, and a `<script>` tag in it does not run. `Core\Response::html` sends the bytes as they are and escapes nothing a second time.

The examples show a small page, text from a visitor inside a page, and a list of products built
from an array.
