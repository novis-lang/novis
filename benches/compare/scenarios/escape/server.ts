// Returns the search term `?q=` in a heading, written with a template literal.
export default {
  fetch(req: Request): Response {
    const term = new URL(req.url).searchParams.get("q") ?? "";
    return new Response(`<h1>Results for ${term}</h1>\n`, { headers: { "Content-Type": "text/html; charset=UTF-8" } });
  },
};
