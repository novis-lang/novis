Adds the next piece of Markdown text and returns the HTML that changed.

A piece can end anywhere, also in the middle of a word or a line. `push` returns a
`Core\Markdown\Update` with two parts. `done()` is the HTML of the blocks that this piece finished.
Add it after the HTML you already show. `open()` is the HTML of the rest, converted as if the text
ended there. Show it after the finished HTML, and replace it at the next `push`.

A block (a paragraph, a list, a heading or a code block) is finished when the first line of the
next block is complete. Its HTML does not change after that.

**Good to know:** a link like `[the guide]` gets its address from a line such as
`[the guide]: /guide`. If that line comes after the block is finished, the link stays plain text.
After `finish`, `push` throws a `RuntimeError`.

**The examples below** show the two parts after each piece, a block that does not change, and a
reply sent to a browser while it is written.
