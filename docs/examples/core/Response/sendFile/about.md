Sends a file from the disk as the whole response. Use it for a download, a PDF or an
image. The program gives only the path. The server reads the file in small pieces, so a large
file does not use the memory of the program.

The server chooses the content type from the file name: `.html` is sent as HTML and `.csv` as CSV.
A browser can also ask for only part of the file. So it can jump to any point in a video, and resume
a download. If the file changed, the browser gets the whole new file.

A request may send files in its application folder, except `nvs.toml` and names that start with `.`.
Other paths need the `fs.read` capability in `nvs.toml`. A path that is not allowed throws a
`RuntimeError`. A missing file throws an `IOError`, and a folder throws a `LogicError`. A path from
the request is tainted, and does not compile here.
