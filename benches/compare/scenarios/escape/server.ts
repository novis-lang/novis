// Returns the search term `?q=` in a heading, written with a template literal on `/` and joined
// with `+` on `/direct`.
export default {
  fetch(req: Request): Response {
    const url = new URL(req.url);
    const term = url.searchParams.get("q") ?? "";
    const body = url.pathname === "/direct" ? "<h1>Results for " + term + "</h1>\n" : `<h1>Results for ${term}</h1>\n`;
    return new Response(body, { headers: { "Content-Type": "text/html; charset=UTF-8" } });
  },
};
