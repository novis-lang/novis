Creates the body of a request, or one field of a form upload, from a string or `bytes` value your
program already has.

You give it the data and a file name. The file name is required, because the other server sees it
as the name of the uploaded file. The result is a `Core\Http\Part`. You pass it as `body` to send it
as the whole request, or put it in `multipart` to send it as one field of a form. `contentType` sets
the media type, for example `text/csv`. If you leave it out, a form field is sent as
`application/octet-stream`.

A `Core\Http\Part` has no methods, so you cannot read the data back from it. In a form upload, a
file name or a media type with a quote or a line break is not allowed. The request throws a
`RuntimeError` before anything is sent.

**Good to know:** the examples run without a network, so every request in them is refused before it
is sent. They show an upload of a picture, a file name with a quote, and a nightly CSV report.
