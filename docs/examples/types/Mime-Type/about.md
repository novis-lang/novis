What a run of bytes turned out to be, read from the bytes themselves. `Core\Mime::detect` answers
with one of these cases, one per format it can recognise: images, audio and video, documents and
archives.

A file name is never part of the answer. Whoever uploads a file chooses what to call it, so a type
taken from the `.png` on the end of it is a type a stranger chose for you. Detection reads the first
few bytes instead, which every one of these formats begins with.

`Unknown` is an ordinary answer and not a failure. Anything written as text — JSON, CSV, SVG, an
HTML page — starts with whatever its author typed and has no first bytes to recognise, so `Unknown`
is what you get for it, and your program has to handle it.

**Good to know:** a case has a spelling to send in a `Content-Type` header, and it only goes that
way. A `Content-Type` a visitor sent you never becomes one of these cases.
