// Returns the version of this file. The driver changes `1` to `2` while the server runs.
export default function (req, res) {
  res.writeHead(200, { "Content-Type": "text/html; charset=UTF-8" });
  res.end("version 1\n");
}
