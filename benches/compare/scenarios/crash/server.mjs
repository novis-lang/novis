// Throws an error for the path `/crash`, and returns `ok` for every other path.
export default function (req, res) {
  if (new URL(req.url, "http://localhost").pathname === "/crash") throw new Error("the handler failed");
  res.writeHead(200, { "Content-Type": "text/html; charset=UTF-8" });
  res.end("ok\n");
}
