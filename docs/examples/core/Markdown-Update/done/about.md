Returns the HTML of the blocks that the last piece finished.

A block is a paragraph, a heading, a list, a code block or another top-level part of the document.
A block is finished when the first line of the next block is complete. Add the result of `done` after
the HTML you already have. A finished block never changes, so you never replace it. When the piece
finished no block, `done` returns an empty `Core\Html\Markup`.

**Good to know:** a reference definition that arrives after a block is finished does not change that
block. A link in that block that needs the definition stays plain text.

**The examples below** show when a paragraph is finished, how to build the whole page from every
`done` part, and when a code block is finished.
