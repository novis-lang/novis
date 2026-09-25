`Core\Mime::mediaType` gives the standard name of a file type, such as `image/png` or
`application/pdf`. This name is called a media type. It goes into a `Content-Type` header, or into a
database next to a stored file.

You give it a case of `Core\Mime\Type`, usually the one `Core\Mime::detect` returned. For
`Core\Mime\Type::Unknown` the result is `application/octet-stream`, which means "some bytes of an
unknown kind". That is the correct header for a file your program could not identify.

It works in one direction only. There is no method that turns a name like `image/png` back into a
case, so a program always compares cases, and a typing mistake in a case does not compile.

**The examples below** show the name of one type, then the name for `Unknown`, then the header a
download sends.
