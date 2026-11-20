// Returns the search term `?q=` in a heading, written with a template literal on `/` and joined
// with `+` on `/direct`.
export default function (req, res) {
  const url = new URL(req.url, "http://localhost");
  const term = url.searchParams.get("q") ?? "";
  const body = url.pathname === "/direct" ? "<h1>Results for " + term + "</h1>\n" : `<h1>Results for ${term}</h1>\n`;
  res.writeHead(200, { "Content-Type": "text/html; charset=UTF-8" });
  res.end(body);
}
