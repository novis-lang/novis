Converts a Markdown document to HTML one part at a time, so a large document does not have to be in
memory all at once.

You read the result with a `foreach` loop. Each part is a `Core\Html\Markup` with one or more whole
blocks, such as a heading, a paragraph or a list. The parts joined together are the same HTML that
`Core\Markdown::toHtml` returns. The document can be an open file, an uploaded file,
`Core\Request::bodyStream()`, `bytes`, or a list or loop that gives `bytes` pieces. The `link`,
`image`, `heading` and `code` functions work as they do for `toHtml`.

A file is read twice. The first read finds every link definition, so a link can use a definition at
the end of the file. After that, only one part of the file is in memory at a time. Every other
source is kept in memory until the loop ends.

**Good to know:** a block that is never closed is one part. A code block with no closing fence, or
one long list, is one part, however large it is. Bytes that are not UTF-8 throw a `ParseError` when
the loop reaches them.
