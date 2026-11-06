`Core\Mime::detect` finds out what kind of file some data is, such as a PNG image, a PDF or a ZIP
archive. It reads only the first 12 bytes, where most file formats write a fixed signature.

You can pass `bytes`, an open `Core\IO\File`, an uploaded `Core\Request\Part` or the request body
stream. A file keeps its position. An upload is not used up, so `saveTo` still writes the whole file
after `detect`.

The result is a case of the enum `Core\Mime::Type`, such as `Core\Mime::Type::Png`. When no
signature matches, the result is `Core\Mime::Type::Unknown`. Text formats such as JSON, CSV and HTML
have no signature, so they are always `Unknown`.

`detect` has no parameter for a file name. Anybody can call a program `photo.png`, so only the bytes
decide the answer.

**Good to know:** the result does not make the data safe. Bytes from a request are still `tainted`
(they came from outside the program) after `detect` read them.

**The examples below** show checking some bytes, then a file name that is not a type, then an upload
form that accepts only images and saves them.
