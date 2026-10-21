Sends a file from the disk as the whole response. Use it for a download, a PDF, an image or a
page that is already saved as a file. The program gives only the path. The server reads the file
in small pieces, so a large file does not use the memory of the program.

The server chooses the content type from the end of the file name: `.html` is sent as HTML and
`.csv` as CSV. It also answers a browser that asks for only part of the file. So a browser can
jump to any point in a video, and resume a download that stopped. If the file changed in the
meantime, the browser gets the whole new file.

A request may send the files in its application folder, except `nvs.toml` and names that start
with `.`. Any other path needs the `fs.read` capability in `nvs.toml`. A path that is not allowed
throws a `RuntimeError`, a missing file throws an `IOError`, and a folder throws a `LogicError`. A path that
comes from the request is tainted, and does not compile here.

The examples show a saved page, a file that does not exist, a manual that the visitor chooses
from a fixed list, and a download that only some visitors may have.
