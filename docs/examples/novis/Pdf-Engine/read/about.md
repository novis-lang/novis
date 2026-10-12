Reads the page count, the metadata and the attached files of a PDF file.

`Engine::read` takes the bytes of a PDF file and returns a shape. `pageCount` is the number of
pages. `metadata` has the `title`, `author`, `subject`, `keywords`, `creator` and `producer` of the
file, and each of them is `null` when the file does not have it. `attachments` lists the attached
files, each with its `name` and its `data`. `invoiceXml` is the XML of an electronic invoice, or
`null` when the file is not one.

A file that is encrypted, or that is not a PDF file, throws a `ParseError`. An invoice XML that is
not UTF-8 text also throws a `ParseError`.

**Good to know:** most programs do not call `Engine::read` directly. `Reader::open` calls it and
returns a `Reader` with the same values.
