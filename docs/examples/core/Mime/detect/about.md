`Core\Mime::detect` finds out what kind of file some bytes are, such as a PNG image, a PDF or a ZIP
archive. It reads the first bytes, where most file formats write a fixed signature.

The result is a case of the enum `Core\Mime\Type`, for example `Core\Mime\Type::Png`. When no
signature matches, the result is `Core\Mime\Type::Unknown`. Text formats such as JSON, CSV, HTML and
SVG have no signature, so they are always `Unknown`.

`detect` has no parameter for a file name. A file name comes from the user, and anybody can call a
program `photo.png`. Only the bytes decide the answer.

**Good to know:** the result does not make the bytes safe. Bytes from a request are still
`tainted` (they came from outside the program) after `detect` read them.

**The examples below** show checking one upload, then a file name that is not a type, then an upload
form that accepts only images.
