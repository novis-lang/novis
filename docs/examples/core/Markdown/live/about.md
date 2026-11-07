Creates an object that converts Markdown to HTML while the text is still arriving.

Use it for text that comes in pieces, such as the answer of a language model that you show while it
is written. `Core\Markdown::live` returns a `Core\Markdown\Live` with no text yet. You give it each
piece with `push`, and you call `finish` when the text is complete. The functions `link`, `image`,
`heading` and `code` are optional. They work the same way as in `Core\Markdown::toHtml`.

**Good to know:** a block (a paragraph, a list, a heading or a code block) is converted again at
each `push` until it is finished. So your `code` or `heading` function can run more than once for
the same block. It should return the same result each time.

**The examples below** show one reply in three pieces, how often a `code` function runs, and a chat
reply whose links are checked.
