Returns the files that a form uploaded. A `foreach` loop reads them one at a time. Each file is a
`Core\Request\Part` with the form field name, the file name that the client sent, the declared
type and the content.

Read the content while the loop is on that file. `readAll` returns it as one value, `content`
returns it in pieces, and `saveTo` writes it to a file. A file that you skip is read past. The file
name and the type are tainted, which means they came from outside your program.

The text fields of the form are not in the loop. Read them with `Core\Request::post` after the
loop. When the request has no `multipart/form-data` body, the loop runs zero times.

The body can be read only once in this way. After `files`, the methods `body`, `bytes`,
`bodyStream` and `json` throw a `LogicError`. A command-line program answers no request, so
`files` throws a `LogicError` there.
