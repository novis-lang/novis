Ends the Markdown text and returns the HTML of the blocks that are not finished yet.

Call `finish` after the last piece. It returns a `Core\Html\Markup` with the last blocks of the
text and its footnotes. Add it after the `done()` parts, and remove the last `open()` part. The
`done()` parts and the result of `finish` together are the whole text as HTML. If no link uses an
address that is defined later in the text, this is the same HTML that `Core\Markdown::toHtml`
returns.

**Good to know:** after `finish`, the object is closed. A second `finish` or another `push` throws a
`RuntimeError`. For the next text, create a new one with `Core\Markdown::live`.
