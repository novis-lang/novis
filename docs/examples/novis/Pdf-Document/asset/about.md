Adds a file that the HTML of a document names, such as an image, a stylesheet, a font or a PDF file.

A render reads no file from the disk and nothing from the network. It reads only its assets and
`data:` URIs. `asset("logo.svg", $bytes)` gives the bytes for every place the HTML, the CSS or an
SVG image names `logo.svg`. The names must match exactly. Any other name, a `file:` address and an
`http:` address make `render` throw a `LogicError` that names it.

A program that needs an image from another server downloads it first, and then adds it as an asset.

**Good to know:** a PDF file can be an asset too. `<img src="terms.pdf#page=2">` draws page 2 of it,
and `@page { background-image: url(letterhead.pdf) }` draws it under every page.

**The examples below** add a logo and a stylesheet, show the error for a file that was not added, and
print a contract on a letterhead.
