// Returns one line for every request. This measures the server alone.
export default function (req, res) {
  res.writeHead(200, { "Content-Type": "text/html; charset=UTF-8" });
  res.end("hello, world\n");
}
