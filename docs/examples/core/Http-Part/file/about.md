Sends a file as the body of a request, or as one field of a form upload, without loading the whole
file into memory first.

`Core\Http\Part::file` does not read the file. It checks that the file exists and returns a part
that contains the path. The file is read while the request is being sent, so a large upload uses
little memory. You give the part to `body`, or to one field of `multipart`. The other server sees
the last part of the path as the file name, and `filename` changes it. `contentType` sets the media
type.

The program needs the `fs.read` capability for the path, in `[capabilities.fs]` in `nvs.toml`.
Without it, the call throws a `RuntimeError` on the line that names the file. If there is no file at
the path, it throws an `IOError`.

**Good to know:** the file is read when the request is sent. If the file changes before that, the
new content is sent.
