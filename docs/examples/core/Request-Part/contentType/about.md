Returns the media type that the client sent with an uploaded file, such as `image/png` or
`application/pdf`. When the client sent no type, the result is `text/plain`.

The result is a `tainted string`, which means it came from outside your program. It is what the client
says the file is, and the client can say anything. A file sent as `image/png` can contain a web page
or a program. Compare the type with a short list of types you accept. If the content matters, also
check the first bytes of the file.

The examples show the type of each file, the type of a file sent without one, and how to accept only
a few image types.
