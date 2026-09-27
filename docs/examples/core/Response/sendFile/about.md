Sends a file from the disk as the whole response. Use it for a download, a PDF, an image or a
page that is already saved as a file. The program gives only the path. The server reads the file
in small pieces, so a large file does not use the memory of the program.

The server chooses the content type from the end of the file name: `.html` is sent as HTML and
`.csv` as CSV. It also answers a browser that asks for only part of the file.

The path must be allowed by `fs.read` in `nvs.toml`. A path that is not allowed throws a
`RuntimeError`, a missing file throws an `IOError`, and a folder throws a `LogicError`. A path that
comes from the request is tainted, and does not compile here.

The examples show a saved page, a file that does not exist, and a manual that the visitor chooses
from a fixed list.
