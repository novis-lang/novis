Sends a body exactly as it is, with the content type you give. Use it for anything that is not
an HTML page, plain text or JSON: an image, a CSV file, a PDF or a calendar file. The other body
methods, `Core\Response::html`, `text` and `json`, choose the content type for you. This one needs
it as its second argument.

The body is `bytes`, so it can contain any value, even ones that are not valid text. Nothing is
escaped or changed. The content type becomes the `Content-Type` header. It must not be empty, and
every character must be printable ASCII. A line break, a control character or a letter like `é`
throws a `LogicError`, and nothing is sent.

The examples show an image, a content type that is not allowed, and a CSV report that the browser
saves as a file.
